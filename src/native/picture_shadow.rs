//! Soft, click-through picture shadows. No timer or retained application bitmap.
use super::*;

pub const PAD: i32 = 16;
const PROPERTY: &str = "QingquPictureShadow";

unsafe extern "system" fn window_proc(hwnd: HWND, msg: u32, w: WPARAM, l: LPARAM) -> LRESULT {
    match msg {
        WM_NCHITTEST => HTTRANSPARENT as isize,
        WM_MOUSEACTIVATE => MA_NOACTIVATE as isize,
        WM_ERASEBKGND => 1,
        WM_PAINT => {
            ValidateRect(hwnd, null());
            0
        }
        _ => DefWindowProcW(hwnd, msg, w, l),
    }
}

pub unsafe fn create(owner: HWND) -> Result<(), String> {
    const CLASS: &str = "QingquOCR.Native.PictureShadow";
    static REGISTERED: std::sync::OnceLock<bool> = std::sync::OnceLock::new();
    if !REGISTERED.get_or_init(|| {
        RegisterClassExW(&WNDCLASSEXW {
            cbSize: size_of::<WNDCLASSEXW>() as u32,
            lpfnWndProc: Some(window_proc),
            hInstance: GetModuleHandleW(null()),
            lpszClassName: platform::wide(CLASS).as_ptr(),
            ..Default::default()
        }) != 0
    }) {
        return Err("无法注册图片阴影窗口。".into());
    }
    let hwnd = CreateWindowExW(
        WS_EX_LAYERED | WS_EX_TRANSPARENT | WS_EX_NOACTIVATE | WS_EX_TOOLWINDOW,
        platform::wide(CLASS).as_ptr(),
        null(),
        WS_POPUP,
        0,
        0,
        0,
        0,
        owner,
        null_mut(),
        GetModuleHandleW(null()),
        null(),
    );
    if hwnd.is_null() {
        return Err("无法创建图片阴影。".into());
    }
    if SetPropW(owner, platform::wide(PROPERTY).as_ptr(), hwnd) == 0 {
        DestroyWindow(hwnd);
        return Err("无法初始化图片阴影。".into());
    }
    // Windows destroys this owned window with its picture, including init errors.
    sync(owner);
    Ok(())
}

fn alpha(x: i32, y: i32, w: i32, h: i32) -> u8 {
    static ALPHA: std::sync::OnceLock<[[u8; 18]; 18]> = std::sync::OnceLock::new();
    let table = ALPHA.get_or_init(|| {
        std::array::from_fn(|dy| {
            std::array::from_fn(|dx| {
                let distance = ((dx * dx + dy * dy) as f32).sqrt();
                (48. * (1. - distance / (PAD + 1) as f32).max(0.).powi(2)).round() as u8
            })
        })
    });
    let dx = (PAD - x).max(x - (PAD + w - 1)).clamp(0, 17) as usize;
    let dy = (PAD + 2 - y).max(y - (PAD + h + 1)).clamp(0, 17) as usize;
    table[dy][dx]
}

pub unsafe fn paint_canvas(bitmap: &Bitmap, stride: i32, surface_height: i32, visible: RECT) {
    let (w, h) = (visible.right - visible.left, visible.bottom - visible.top);
    for y in (visible.top - PAD).max(0)..(visible.bottom + PAD).min(surface_height) {
        let ranges = if (visible.top..visible.bottom).contains(&y) {
            [
                (visible.left - PAD).max(0)..visible.left,
                visible.right..(visible.right + PAD).min(stride),
            ]
        } else {
            [
                (visible.left - PAD).max(0)..(visible.right + PAD).min(stride),
                0..0,
            ]
        };
        for range in ranges {
            for x in range {
                *bitmap.pixels.add(((y * stride + x) * 4 + 3) as usize) =
                    alpha(x - visible.left + PAD, y - visible.top + PAD, w, h);
            }
        }
    }
}

pub unsafe fn sync(owner: HWND) {
    thread_local! { static SYNCING: std::cell::Cell<bool> = const { std::cell::Cell::new(false) }; }
    if SYNCING.with(|busy| busy.replace(true)) {
        return;
    }
    struct Reset;
    impl Drop for Reset {
        fn drop(&mut self) {
            SYNCING.with(|busy| busy.set(false));
        }
    }
    let _reset = Reset;
    let hwnd = GetPropW(owner, platform::wide(PROPERTY).as_ptr()) as HWND;
    if hwnd.is_null() || IsWindow(hwnd) == 0 {
        return;
    }
    if !GetPropW(owner, platform::wide("QingquPictureCanvas").as_ptr()).is_null() {
        ShowWindow(hwnd, SW_HIDE);
        return;
    }
    // Use this top-level window's visibility bit; IsWindowVisible also considers
    // the desktop, which is deliberately hidden in isolated clipboard checks.
    if GetWindowLongPtrW(owner, GWL_STYLE) as u32 & WS_VISIBLE == 0 || IsIconic(owner) != 0 {
        ShowWindow(hwnd, SW_HIDE);
        return;
    }
    let r = super::picture::visible_bounds(owner);
    let (w, h) = (r.right - r.left, r.bottom - r.top);
    let (width, height) = (w + PAD * 2, h + PAD * 2);
    let mut old = RECT::default();
    GetWindowRect(hwnd, &mut old);
    if GetWindowLongPtrW(hwnd, GWL_STYLE) as u32 & WS_VISIBLE != 0
        && old.left == r.left - PAD
        && old.top == r.top - PAD
        && old.right - old.left == width
        && old.bottom - old.top == height
    {
        return;
    }
    if old.right - old.left != width || old.bottom - old.top != height {
        let Ok(bitmap) = Bitmap::new(width, height) else {
            return;
        };
        let pixels = std::slice::from_raw_parts_mut(bitmap.pixels, (width * height * 4) as usize);
        pixels.fill(0);
        for y in 0..height {
            let ranges = if (PAD..PAD + h).contains(&y) {
                [0..PAD, PAD + w..width]
            } else {
                [0..width, 0..0]
            };
            for range in ranges {
                for x in range {
                    pixels[((y * width + x) * 4 + 3) as usize] = alpha(x, y, w, h);
                }
            }
        }
        // The interior already has zero alpha. A separate hollow window region
        // would briefly clip the old surface to new geometry during resizing.
        let blend = BLENDFUNCTION {
            BlendOp: AC_SRC_OVER as u8,
            BlendFlags: 0,
            SourceConstantAlpha: 255,
            AlphaFormat: AC_SRC_ALPHA as u8,
        };
        let updated = UpdateLayeredWindow(
            hwnd,
            null_mut(),
            &POINT {
                x: r.left - PAD,
                y: r.top - PAD,
            },
            &SIZE {
                cx: width,
                cy: height,
            },
            bitmap.dc,
            &POINT::default(),
            0,
            &blend,
            ULW_ALPHA,
        );
        if updated == 0 {
            return;
        }
        // UpdateLayeredWindow owns its backing surface; release our temporary DIB.
    }
    SetWindowPos(
        hwnd,
        null_mut(),
        r.left - PAD,
        r.top - PAD,
        width,
        height,
        SWP_NOACTIVATE | SWP_NOZORDER | SWP_SHOWWINDOW,
    );
}

pub unsafe fn forget(owner: HWND) {
    RemovePropW(owner, platform::wide(PROPERTY).as_ptr());
}
