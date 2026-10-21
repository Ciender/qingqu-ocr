//! Frozen desktop selection with a single retained screenshot and clipped updates.
use super::*;
use windows_sys::Win32::Graphics::Dwm::{
    DwmGetWindowAttribute, DWMWA_CLOAKED, DWMWA_EXTENDED_FRAME_BOUNDS,
};

struct Selection {
    // GDI capture is stored in BGRA channel order.
    image: RgbaImage,
    tile: crate::native::win::Bitmap,
    mask_dc: HDC,
    mask_bitmap: HBITMAP,
    old_mask: HGDIOBJ,
    windows: Option<Vec<RECT>>,
    a: POINT,
    b: POINT,
    drag: bool,
    press: POINT,
    manual: bool,
    result: Option<RgbaImage>,
}
impl Selection {
    fn new(mut image: RgbaImage, windows: Option<Vec<RECT>>) -> Result<Self, String> {
        let (width, height) = image.dimensions();
        if width == 0 || height == 0 {
            return Err("屏幕区域无效。".into());
        }
        for p in image.as_mut().chunks_exact_mut(4) {
            p.swap(0, 2);
        }
        let tile = crate::native::win::Bitmap::new(512, 512)?;
        let (mask_dc, mask_bitmap, old_mask) = unsafe {
            let screen = GetDC(null_mut());
            if screen.is_null() {
                return Err("无法创建截图遮罩。".into());
            }
            let dc = CreateCompatibleDC(screen);
            let bitmap = CreateCompatibleBitmap(screen, 1, 1);
            ReleaseDC(null_mut(), screen);
            if dc.is_null() || bitmap.is_null() {
                if !bitmap.is_null() {
                    DeleteObject(bitmap);
                }
                if !dc.is_null() {
                    DeleteDC(dc);
                }
                return Err("无法创建截图遮罩。".into());
            }
            let previous = SelectObject(dc, bitmap);
            if previous.is_null() || previous == -1isize as HGDIOBJ {
                DeleteObject(bitmap);
                DeleteDC(dc);
                return Err("无法初始化截图遮罩。".into());
            }
            let brush = CreateSolidBrush(0);
            let rect = RECT {
                left: 0,
                top: 0,
                right: 1,
                bottom: 1,
            };
            FillRect(dc, &rect, brush);
            DeleteObject(brush);
            (dc, bitmap, previous)
        };
        if mask_dc.is_null() {
            return Err("无法创建截图遮罩。".into());
        }
        Ok(Self {
            image,
            tile,
            mask_dc,
            mask_bitmap,
            old_mask,
            windows,
            a: POINT::default(),
            b: POINT::default(),
            drag: false,
            press: POINT::default(),
            manual: false,
            result: None,
        })
    }
    fn rect(&self) -> RECT {
        RECT {
            left: self.a.x.min(self.b.x),
            top: self.a.y.min(self.b.y),
            right: self.a.x.max(self.b.x),
            bottom: self.a.y.max(self.b.y),
        }
    }
    fn update(&mut self, point: POINT) {
        if self.drag {
            if self.manual
                || (point.x - self.press.x)
                    .abs()
                    .max((point.y - self.press.y).abs())
                    >= 4
            {
                self.manual = true;
                self.a = self.press;
                self.b = point;
            }
            return;
        }
        if let Some(windows) = &self.windows {
            let r = windows.iter().find(|r| {
                point.x >= r.left && point.x < r.right && point.y >= r.top && point.y < r.bottom
            });
            let r = r.copied().unwrap_or_default();
            self.a = POINT {
                x: r.left,
                y: r.top,
            };
            self.b = POINT {
                x: r.right,
                y: r.bottom,
            };
        }
    }
    fn finish(&mut self) -> bool {
        let r = self.rect();
        if r.right <= r.left || r.bottom <= r.top {
            return false;
        }
        self.result = Some(
            if r.left == 0
                && r.top == 0
                && r.right == self.image.width() as i32
                && r.bottom == self.image.height() as i32
            {
                std::mem::take(&mut self.image)
            } else {
                let result = image::imageops::crop_imm(
                    &self.image,
                    r.left as u32,
                    r.top as u32,
                    (r.right - r.left) as u32,
                    (r.bottom - r.top) as u32,
                )
                .to_image();
                self.image = RgbaImage::new(0, 0);
                result
            },
        );
        if let Some(result) = &mut self.result {
            for pixel in result.as_mut().chunks_exact_mut(4) {
                pixel.swap(0, 2);
            }
        }
        true
    }
    // One reusable 1 MiB DIB, regardless of dirty-region or monitor size.
    // Compose every tile completely before presenting it; never expose a bright intermediate frame.
    unsafe fn paint_to(&self, target: HDC, dirty: RECT) {
        let width = self.image.width() as i32;
        let height = self.image.height() as i32;
        for top in (dirty.top.max(0)..dirty.bottom.min(height)).step_by(512) {
            for left in (dirty.left.max(0)..dirty.right.min(width)).step_by(512) {
                let r = RECT {
                    left,
                    top,
                    right: (left + 512).min(dirty.right).min(width),
                    bottom: (top + 512).min(dirty.bottom).min(height),
                };
                if RectVisible(target, &r) == 0 {
                    continue;
                }
                // Flush previous GDI use before the CPU writes into this DIB again.
                GdiFlush();
                for y in top..r.bottom {
                    let start = ((y * width + left) as usize) * 4;
                    std::ptr::copy_nonoverlapping(
                        self.image.as_raw().as_ptr().add(start),
                        self.tile.pixels.add(((y - top) * 512 * 4) as usize),
                        ((r.right - left) * 4) as usize,
                    );
                }
                let saved = SaveDC(self.tile.dc);
                SetViewportOrgEx(self.tile.dc, -left, -top, null_mut());
                IntersectClipRect(self.tile.dc, left, top, r.right, r.bottom);
                self.paint_overlay(self.tile.dc, r);
                RestoreDC(self.tile.dc, saved);
                BitBlt(
                    target,
                    left,
                    top,
                    r.right - left,
                    r.bottom - top,
                    self.tile.dc,
                    0,
                    0,
                    SRCCOPY,
                );
            }
        }
    }
    unsafe fn paint_overlay(&self, dc: HDC, dirty: RECT) {
        let r = self.rect();
        let w = self.image.width() as i32;
        let h = self.image.height() as i32;
        let alpha = BLENDFUNCTION {
            BlendOp: AC_SRC_OVER as u8,
            BlendFlags: 0,
            SourceConstantAlpha: 71,
            AlphaFormat: 0,
        };
        let mut masks = [
            RECT {
                left: 0,
                top: 0,
                right: w,
                bottom: r.top,
            },
            RECT {
                left: 0,
                top: r.bottom,
                right: w,
                bottom: h,
            },
            RECT {
                left: 0,
                top: r.top,
                right: r.left,
                bottom: r.bottom,
            },
            RECT {
                left: r.right,
                top: r.top,
                right: w,
                bottom: r.bottom,
            },
        ];
        if r.right <= r.left || r.bottom <= r.top {
            masks = [
                RECT {
                    left: 0,
                    top: 0,
                    right: w,
                    bottom: h,
                },
                RECT::default(),
                RECT::default(),
                RECT::default(),
            ];
        }
        for mask in masks {
            let left = mask.left.max(dirty.left);
            let top = mask.top.max(dirty.top);
            let right = mask.right.min(dirty.right);
            let bottom = mask.bottom.min(dirty.bottom);
            if right > left && bottom > top {
                AlphaBlend(
                    dc,
                    left,
                    top,
                    right - left,
                    bottom - top,
                    self.mask_dc,
                    0,
                    0,
                    1,
                    1,
                    alpha,
                );
            }
        }
        if r.right > r.left && r.bottom > r.top {
            let pen = CreatePen(PS_SOLID, 2, 0x00c09020);
            let old = SelectObject(dc, pen);
            let brush = SelectObject(dc, GetStockObject(NULL_BRUSH));
            Rectangle(dc, r.left, r.top, r.right, r.bottom);
            SelectObject(dc, brush);
            SelectObject(dc, old);
            DeleteObject(pen);
        }
        let font = SelectObject(dc, GetStockObject(DEFAULT_GUI_FONT));
        SetBkMode(dc, OPAQUE as i32);
        SetBkColor(dc, 0x0037312e);
        SetTextColor(dc, 0x00ffffff);
        let text = wide(&format!(
            "{} × {}    {}    Esc / 右键取消",
            r.right - r.left,
            r.bottom - r.top,
            if self.windows.is_some() {
                "单击识别窗口 · 拖动手动框选，松开立即识别"
            } else {
                "拖动框选，松开左键立即识别"
            },
        ));
        TextOutW(dc, 16, 16, text.as_ptr(), (text.len() - 1) as i32);
        SelectObject(dc, font);
    }
}
impl Drop for Selection {
    fn drop(&mut self) {
        unsafe {
            if !self.old_mask.is_null() {
                SelectObject(self.mask_dc, self.old_mask);
            }
            if !self.mask_bitmap.is_null() {
                DeleteObject(self.mask_bitmap);
            }
            if !self.mask_dc.is_null() {
                DeleteDC(self.mask_dc);
            }
        }
    }
}

pub fn select_region(image: RgbaImage, x: i32, y: i32) -> Result<Option<RgbaImage>, String> {
    let windows = window_rects(x, y, image.width() as i32, image.height() as i32);
    select(image, x, y, Some(windows))
}
pub fn select_window(image: RgbaImage, x: i32, y: i32) -> Result<Option<RgbaImage>, String> {
    // Preserve old hotkeys while routing them to the unified picker.
    select_region(image, x, y)
}
fn clip_window(r: RECT, x: i32, y: i32, width: i32, height: i32) -> Option<RECT> {
    let r = RECT {
        left: (r.left - x).clamp(0, width),
        top: (r.top - y).clamp(0, height),
        right: (r.right - x).clamp(0, width),
        bottom: (r.bottom - y).clamp(0, height),
    };
    (r.right > r.left && r.bottom > r.top).then_some(r)
}
fn window_rects(x: i32, y: i32, width: i32, height: i32) -> Vec<RECT> {
    unsafe {
        unsafe extern "system" fn collect(hwnd: HWND, param: LPARAM) -> i32 {
            (*(param as *mut Vec<HWND>)).push(hwnd);
            1
        }
        let mut handles = Vec::<HWND>::new();
        // EnumWindows preserves stacking order, so a point selects the frontmost window.
        EnumWindows(Some(collect), (&mut handles as *mut Vec<HWND>) as isize);
        handles
            .into_iter()
            .filter_map(|hwnd| {
                let mut process = 0;
                GetWindowThreadProcessId(hwnd, &mut process);
                if process == GetCurrentProcessId()
                    || IsWindowVisible(hwnd) == 0
                    || IsIconic(hwnd) != 0
                {
                    return None;
                }
                let mut cloaked: u32 = 0;
                DwmGetWindowAttribute(
                    hwnd,
                    DWMWA_CLOAKED as u32,
                    (&mut cloaked as *mut u32).cast(),
                    4,
                );
                if cloaked != 0 {
                    return None;
                }
                let mut class = [0u16; 128];
                let len = GetClassNameW(hwnd, class.as_mut_ptr(), class.len() as i32);
                let class = String::from_utf16_lossy(&class[..len.max(0) as usize]);
                if matches!(
                    class.as_str(),
                    "Progman" | "WorkerW" | "Shell_TrayWnd" | "Shell_SecondaryTrayWnd"
                ) {
                    return None;
                }
                let mut r = RECT::default();
                if DwmGetWindowAttribute(
                    hwnd,
                    DWMWA_EXTENDED_FRAME_BOUNDS as u32,
                    (&mut r as *mut RECT).cast(),
                    std::mem::size_of::<RECT>() as u32,
                ) < 0
                    && GetWindowRect(hwnd, &mut r) == 0
                {
                    return None;
                }
                clip_window(r, x, y, width, height)
            })
            .collect()
    }
}
fn select(
    image: RgbaImage,
    x: i32,
    y: i32,
    windows: Option<Vec<RECT>>,
) -> Result<Option<RgbaImage>, String> {
    unsafe {
        let mut state = Box::new(Selection::new(image, windows)?);
        let class = wide("QingquOCR.selection");
        let module = GetModuleHandleW(null());
        let wc = WNDCLASSW {
            lpfnWndProc: Some(selection_proc),
            hInstance: module,
            lpszClassName: class.as_ptr(),
            hCursor: LoadCursorW(null_mut(), IDC_CROSS),
            ..std::mem::zeroed()
        };
        RegisterClassW(&wc);
        let hwnd = CreateWindowExW(
            WS_EX_TOPMOST | WS_EX_TOOLWINDOW,
            class.as_ptr(),
            wide("截图选择 · 松开左键立即识别 · Esc / 右键取消").as_ptr(),
            WS_POPUP,
            x,
            y,
            state.image.width() as i32,
            state.image.height() as i32,
            null_mut(),
            null_mut(),
            module,
            null(),
        );
        if hwnd.is_null() {
            return Err("无法打开截图选择窗口。".into());
        }
        SetWindowLongPtrW(
            hwnd,
            GWLP_USERDATA,
            (&mut *state) as *mut Selection as isize,
        );
        if state.windows.is_some() {
            let mut point = POINT::default();
            if GetCursorPos(&mut point) != 0 {
                ScreenToClient(hwnd, &mut point);
                state.update(point);
            }
        }
        ShowWindow(hwnd, SW_SHOW);
        SetForegroundWindow(hwnd);
        SetFocus(hwnd);
        let mut msg: MSG = std::mem::zeroed();
        while IsWindow(hwnd) != 0 {
            let status = GetMessageW(&mut msg, null_mut(), 0, 0);
            if status <= 0 {
                DestroyWindow(hwnd);
                if status < 0 {
                    return Err("截图选择窗口消息处理失败。".into());
                }
                break;
            }
            TranslateMessage(&msg);
            DispatchMessageW(&msg);
        }
        Ok(state.result.take())
    }
}
unsafe extern "system" fn selection_proc(hwnd: HWND, msg: u32, w: WPARAM, l: LPARAM) -> LRESULT {
    if msg == WM_ERASEBKGND {
        return 1;
    }
    let ptr = GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *mut Selection;
    if ptr.is_null() {
        return DefWindowProcW(hwnd, msg, w, l);
    }
    let s = &mut *ptr;
    let point = POINT {
        x: ((l as u32 & 0xffff) as i16 as i32).clamp(0, s.image.width() as i32),
        y: (((l as u32 >> 16) & 0xffff) as i16 as i32).clamp(0, s.image.height() as i32),
    };
    match msg {
        WM_LBUTTONDOWN => {
            let old = s.rect();
            s.update(point);
            s.press = point;
            s.manual = false;
            if s.windows.is_none() {
                s.a = point;
                s.b = point;
            }
            s.drag = true;
            SetCapture(hwnd);
            let r = s.rect();
            invalidate_selection(hwnd, old, r);
            return 0;
        }
        WM_MOUSEMOVE if s.drag || s.windows.is_some() => {
            let old = s.rect();
            s.update(point);
            invalidate_selection(hwnd, old, s.rect());
            return 0;
        }
        WM_LBUTTONUP if s.drag => {
            // Use the release coordinates even when no final WM_MOUSEMOVE was delivered.
            s.update(point);
            s.drag = false;
            let finished = s.finish();
            ReleaseCapture();
            if finished {
                DestroyWindow(hwnd);
            } else {
                InvalidateRect(hwnd, null(), 0);
            }
            return 0;
        }
        WM_CAPTURECHANGED => {
            s.drag = false;
            return 0;
        }
        WM_RBUTTONUP | WM_CLOSE => {
            DestroyWindow(hwnd);
            return 0;
        }
        WM_KEYDOWN if w == VK_ESCAPE as usize => {
            DestroyWindow(hwnd);
            return 0;
        }
        WM_PAINT => {
            let mut ps: PAINTSTRUCT = std::mem::zeroed();
            let dc = BeginPaint(hwnd, &mut ps);
            s.paint_to(dc, ps.rcPaint);
            EndPaint(hwnd, &ps);
            return 0;
        }
        _ => {}
    }
    DefWindowProcW(hwnd, msg, w, l)
}
unsafe fn invalidate_selection(hwnd: HWND, old: RECT, new: RECT) {
    for r in selection_damage(old, new) {
        InvalidateRect(hwnd, &r, 0);
    }
    let label = RECT {
        left: 0,
        top: 0,
        right: 1000,
        bottom: 48,
    };
    InvalidateRect(hwnd, &label, 0);
}

// Only the symmetric difference changes brightness. Repaint both outlines too;
// dragging an almost fullscreen rectangle by one pixel must not repaint its interior.
fn selection_damage(old: RECT, new: RECT) -> Vec<RECT> {
    let mut out = Vec::with_capacity(16);
    for (a, b) in [(old, new), (new, old)] {
        if a.right <= a.left || a.bottom <= a.top {
            continue;
        }
        let i = RECT {
            left: a.left.max(b.left),
            top: a.top.max(b.top),
            right: a.right.min(b.right),
            bottom: a.bottom.min(b.bottom),
        };
        if i.right <= i.left || i.bottom <= i.top {
            out.push(a);
        } else {
            out.extend([
                RECT { bottom: i.top, ..a },
                RECT { top: i.bottom, ..a },
                RECT {
                    top: i.top,
                    bottom: i.bottom,
                    right: i.left,
                    ..a
                },
                RECT {
                    top: i.top,
                    bottom: i.bottom,
                    left: i.right,
                    ..a
                },
            ]);
        }
        out.extend([
            RECT {
                left: a.left - 3,
                top: a.top - 3,
                right: a.right + 3,
                bottom: a.top + 3,
            },
            RECT {
                left: a.left - 3,
                top: a.bottom - 3,
                right: a.right + 3,
                bottom: a.bottom + 3,
            },
            RECT {
                left: a.left - 3,
                top: a.top - 3,
                right: a.left + 3,
                bottom: a.bottom + 3,
            },
            RECT {
                left: a.right - 3,
                top: a.top - 3,
                right: a.right + 3,
                bottom: a.bottom + 3,
            },
        ]);
    }
    out.retain(|r| r.right > r.left && r.bottom > r.top);
    out
}
