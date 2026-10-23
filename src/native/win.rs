//! Small Win32 wrappers. GDI objects are owned and released with their windows.
use crate::platform::wide;
use std::{
    mem::size_of,
    ptr::{null, null_mut},
};
pub use windows_sys::Win32::{
    Foundation::*,
    Graphics::Gdi::*,
    System::LibraryLoader::{GetModuleHandleW, LoadLibraryW},
    UI::{
        Controls::*,
        HiDpi::*,
        Input::KeyboardAndMouse::*,
        Shell::{DefSubclassProc, DragAcceptFiles, DragFinish, DragQueryFileW, SetWindowSubclass},
        WindowsAndMessaging::*,
    },
};

pub const EXGETSEL: u32 = WM_USER + 52;
pub const EXSETSEL: u32 = WM_USER + 55;
pub const SETBKGNDCOLOR: u32 = WM_USER + 67;
pub const SETCHARFORMAT: u32 = WM_USER + 68;
pub const SETUNDOLIMIT: u32 = WM_USER + 82;
pub const REDO: u32 = WM_USER + 84;
pub const SETTEXTMODE: u32 = WM_USER + 89;
pub const SETZOOM: u32 = WM_USER + 225;
pub const EXLIMITTEXT: u32 = WM_USER + 53;
pub const GETSCROLLPOS: u32 = WM_USER + 221;
pub const SETSCROLLPOS: u32 = WM_USER + 222;

#[repr(C)]
#[derive(Clone, Copy, Default, Debug)]
pub struct Range {
    pub start: i32,
    pub end: i32,
}
#[repr(C)]
pub struct CharFormat {
    size: u32,
    mask: u32,
    effects: u32,
    height: i32,
    offset: i32,
    color: u32,
    charset: u8,
    pitch: u8,
    face: [u16; 32],
}

pub fn color(rgb: u32) -> COLORREF {
    ((rgb & 0xff0000) >> 16) | (rgb & 0xff00) | ((rgb & 0xff) << 16)
}
#[derive(Clone, Copy)]
pub struct Theme {
    pub bg: COLORREF,
    pub surface: COLORREF,
    pub text: COLORREF,
    pub muted: COLORREF,
    pub border: COLORREF,
    pub hover: COLORREF,
    pub accent: COLORREF,
}
impl Theme {
    pub fn new(palette: &str, dark: bool) -> Self {
        let (bg, surface, text, muted, border, hover, accent) = match (palette, dark) {
            ("catppuccin", true) => (
                0x1e1e2e, 0x181825, 0xcdd6f4, 0xa6adc8, 0x45475a, 0x313244, 0x89b4fa,
            ),
            ("catppuccin", false) => (
                0xeff1f5, 0xe6e9ef, 0x4c4f69, 0x6c6f85, 0xbcc0cc, 0xdce0e8, 0x1e66f5,
            ),
            (_, true) => (
                0x1f1f1f, 0x181818, 0xcccccc, 0x9d9d9d, 0x2b2b2b, 0x313131, 0x0078d4,
            ),
            _ => (
                0xffffff, 0xf8f8f8, 0x3b3b3b, 0x616161, 0xe5e5e5, 0xe8e8e8, 0x005fb8,
            ),
        };
        Self {
            bg: color(bg),
            surface: color(surface),
            text: color(text),
            muted: color(muted),
            border: color(border),
            hover: color(hover),
            accent: color(accent),
        }
    }
}
pub struct Font(pub HFONT);
impl Font {
    pub fn editor(points: f32, dpi: u32) -> Self {
        Self::new(points * 96. / 72., dpi)
    }
    pub fn new(px: f32, dpi: u32) -> Self {
        unsafe {
            Self(CreateFontW(
                -(px * dpi as f32 / 96.).round() as i32,
                0,
                0,
                0,
                FW_NORMAL as i32,
                0,
                0,
                0,
                DEFAULT_CHARSET as u32,
                OUT_DEFAULT_PRECIS as u32,
                CLIP_DEFAULT_PRECIS as u32,
                CLEARTYPE_QUALITY as u32,
                DEFAULT_PITCH as u32,
                wide("Microsoft YaHei UI").as_ptr(),
            ))
        }
    }
}
impl Drop for Font {
    fn drop(&mut self) {
        unsafe {
            DeleteObject(self.0);
        }
    }
}
pub struct Brush(pub HBRUSH);
impl Brush {
    pub fn new(c: COLORREF) -> Self {
        Self(unsafe { CreateSolidBrush(c) })
    }
}
impl Drop for Brush {
    fn drop(&mut self) {
        unsafe {
            DeleteObject(self.0);
        }
    }
}
pub fn rect(x: i32, y: i32, w: i32, h: i32) -> RECT {
    RECT {
        left: x,
        top: y,
        right: x + w,
        bottom: y + h,
    }
}
// Keep the resize/snap styles, but own the entire non-client frame. The themed
// WS_THICKFRAME otherwise paints a recessed dark band whenever activation changes.
pub const WINDOW_FRAME: i32 = 2;
pub const PICTURE_FRAME: i32 = 1;
pub unsafe fn window_frame_width(hwnd: HWND) -> i32 {
    if GetPropW(hwnd, wide("QingquRole").as_ptr()) as usize == 3 {
        PICTURE_FRAME
    } else {
        WINDOW_FRAME
    }
}
pub unsafe fn paint_window_frame(hwnd: HWND, dc: HDC) {
    let mut r = RECT::default();
    GetWindowRect(hwnd, &mut r);
    paint_window_frame_at(hwnd, dc, r.right - r.left, r.bottom - r.top);
}
pub unsafe fn paint_window_frame_at(hwnd: HWND, dc: HDC, width: i32, height: i32) {
    let dark = (GetPropW(hwnd, wide("QingquTheme").as_ptr()) as usize).saturating_sub(1) % 2 == 1;
    let active = GetPropW(hwnd, wide("QingquActive").as_ptr()) as usize != 0;
    let ink = color(match (dark, active) {
        (false, true) => 0x1769cf,
        (false, false) => 0x9cc7f5,
        (true, true) => 0x73b5ff,
        (true, false) => 0x315d87,
    });
    let brush = Brush::new(ink);
    let mut frame = rect(0, 0, width, height);
    for _ in 0..window_frame_width(hwnd) {
        FrameRect(dc, &frame, brush.0);
        InflateRect(&mut frame, -1, -1);
    }
}
pub unsafe fn window_frame(hwnd: HWND) {
    if GetPropW(hwnd, wide("QingquRole").as_ptr()) as usize == 3
        && GetWindowLongPtrW(hwnd, GWL_EXSTYLE) as u32 & WS_EX_LAYERED != 0
    {
        InvalidateRect(hwnd, null(), 0);
        return;
    }
    let dc = GetWindowDC(hwnd);
    if !dc.is_null() {
        paint_window_frame(hwnd, dc);
        ReleaseDC(hwnd, dc);
    }
}
pub unsafe fn custom_frame(hwnd: HWND) {
    use windows_sys::Win32::Graphics::Dwm::{
        DwmSetWindowAttribute, DWMNCRP_DISABLED, DWMWA_NCRENDERING_POLICY,
    };
    // Pictures use their own faint shadow, avoiding a second system shadow.
    let policy = DWMNCRP_DISABLED;
    DwmSetWindowAttribute(
        hwnd,
        DWMWA_NCRENDERING_POLICY as u32,
        (&policy as *const i32).cast(),
        size_of_val(&policy) as u32,
    );
    SetWindowPos(
        hwnd,
        null_mut(),
        0,
        0,
        0,
        0,
        SWP_NOMOVE | SWP_NOSIZE | SWP_NOZORDER | SWP_NOACTIVATE | SWP_FRAMECHANGED,
    );
    window_frame(hwnd);
}
pub unsafe fn fill(dc: HDC, r: &RECT, c: COLORREF) {
    let b = Brush::new(c);
    FillRect(dc, r, b.0);
}
pub unsafe fn label(dc: HDC, r: &RECT, s: &str, c: COLORREF, font: HFONT, flags: u32) {
    let previous = SelectObject(dc, font);
    SetBkMode(dc, TRANSPARENT as i32);
    SetTextColor(dc, c);
    let mut r = *r;
    let text = wide(s);
    DrawTextW(
        dc,
        text.as_ptr(),
        (text.len() - 1) as i32,
        &mut r,
        flags | DT_NOPREFIX,
    );
    SelectObject(dc, previous);
}
pub unsafe fn text(hwnd: HWND) -> String {
    if super::scintilla::is_editor(hwnd) {
        return super::scintilla::text(hwnd);
    }
    let len = GetWindowTextLengthW(hwnd).max(0) as usize;
    let mut buf = vec![0u16; len + 1];
    let mut class = [0u16; 64];
    GetClassNameW(hwnd, class.as_mut_ptr(), class.len() as i32);
    let n = if String::from_utf16_lossy(&class).starts_with("RICHEDIT") {
        #[repr(C)]
        struct GetText {
            bytes: u32,
            flags: u32,
            codepage: u32,
            default_char: *const u8,
            used_default: *mut i32,
        }
        let options = GetText {
            bytes: (buf.len() * 2) as u32,
            flags: 0,
            codepage: 1200,
            default_char: null(),
            used_default: null_mut(),
        };
        SendMessageW(
            hwnd,
            WM_USER + 94,
            (&options as *const GetText) as usize,
            buf.as_mut_ptr() as isize,
        )
        .max(0) as usize
    } else {
        GetWindowTextW(hwnd, buf.as_mut_ptr(), buf.len() as i32).max(0) as usize
    };
    String::from_utf16_lossy(&buf[..n])
        .replace("\r\n", "\n")
        .replace('\r', "\n")
}
pub unsafe fn set_text(hwnd: HWND, s: &str) {
    SetWindowTextW(
        hwnd,
        wide(&s.replace("\r\n", "\n").replace('\n', "\r\n")).as_ptr(),
    );
}
pub unsafe fn stream_text(hwnd: HWND, s: &str) {
    let mut selection = Range::default();
    let mut scroll = POINT::default();
    SendMessageW(hwnd, EXGETSEL, 0, (&mut selection as *mut Range) as isize);
    SendMessageW(hwnd, GETSCROLLPOS, 0, (&mut scroll as *mut POINT) as isize);
    let mut vertical = SCROLLINFO {
        cbSize: size_of::<SCROLLINFO>() as u32,
        fMask: SIF_ALL,
        ..Default::default()
    };
    let at_bottom = GetScrollInfo(hwnd, SB_VERT, &mut vertical) != 0
        && vertical.nPos >= vertical.nMax - vertical.nPage as i32 + 1 - 2;
    let follow = selection.start == selection.end
        && (at_bottom || SendMessageW(hwnd, WM_GETTEXTLENGTH, 0, 0) == 0);
    let visible = IsWindowVisible(hwnd) != 0;
    if visible {
        SendMessageW(hwnd, WM_SETREDRAW, 0, 0);
    }
    set_text(hwnd, s);
    if follow {
        SendMessageW(hwnd, EM_SETSEL, usize::MAX, -1);
        SendMessageW(hwnd, EM_SCROLLCARET, 0, 0);
    } else {
        SendMessageW(hwnd, EXSETSEL, 0, (&selection as *const Range) as isize);
        SendMessageW(hwnd, SETSCROLLPOS, 0, (&scroll as *const POINT) as isize);
    }
    if visible {
        SendMessageW(hwnd, WM_SETREDRAW, 1, 0);
    }
    InvalidateRect(hwnd, null(), 0);
}
pub unsafe fn selected(hwnd: HWND) -> (Range, String) {
    let mut r = Range::default();
    SendMessageW(hwnd, EXGETSEL, 0, (&mut r as *mut Range) as isize);
    let whole = text(hwnd)
        .replace('\n', "\r")
        .encode_utf16()
        .collect::<Vec<_>>();
    let (a, b) = (r.start.max(0) as usize, r.end.max(0) as usize);
    let s = if a < b && b <= whole.len() {
        String::from_utf16_lossy(&whole[a..b])
    } else {
        String::from_utf16_lossy(&whole)
    };
    let r = if a == b {
        Range {
            start: 0,
            end: whole.len() as i32,
        }
    } else {
        r
    };
    (r, s.replace("\r\n", "\n").replace('\r', "\n"))
}
pub unsafe fn range_text(hwnd: HWND, r: Range) -> String {
    let body = text(hwnd)
        .replace('\n', "\r")
        .encode_utf16()
        .collect::<Vec<_>>();
    let start = (r.start.max(0) as usize).min(body.len());
    let end = if r.end < 0 {
        body.len()
    } else {
        (r.end as usize).min(body.len())
    };
    if start > end {
        return String::new();
    }
    String::from_utf16_lossy(&body[start..end]).replace('\r', "\n")
}
pub unsafe fn replace(hwnd: HWND, r: Range, s: &str) {
    SendMessageW(hwnd, WM_USER + 88, 0, 0);
    SendMessageW(hwnd, EXSETSEL, 0, (&r as *const Range) as isize);
    let body = wide(&s.replace("\r\n", "\n").replace('\n', "\r\n"));
    SendMessageW(hwnd, EM_REPLACESEL, 1, body.as_ptr() as isize);
    SendMessageW(hwnd, WM_USER + 88, 0, 0);
}
/// Formatting should keep the reader's selection and viewport. Normal paste,
/// typing and search replacement retain their usual caret-following behavior.
pub unsafe fn replace_preserving_view(hwnd: HWND, r: Range, s: &str) {
    let mut selection = Range::default();
    let mut scroll = POINT::default();
    SendMessageW(hwnd, EXGETSEL, 0, (&mut selection as *mut Range) as isize);
    SendMessageW(hwnd, GETSCROLLPOS, 0, (&mut scroll as *mut POINT) as isize);
    let visible = IsWindowVisible(hwnd) != 0;
    if visible {
        SendMessageW(hwnd, WM_SETREDRAW, 0, 0);
    }
    replace(hwnd, r, s);
    let inserted = s
        .replace("\r\n", "\n")
        .replace('\r', "\n")
        .encode_utf16()
        .count() as i32;
    let length = SendMessageW(hwnd, WM_GETTEXTLENGTH, 0, 0).max(0) as i32;
    let map = |position: i32| {
        let mapped = if position <= r.start {
            position
        } else if r.end >= 0 && position >= r.end {
            position - (r.end - r.start) + inserted
        } else {
            r.start + (position - r.start).min(inserted)
        };
        mapped.clamp(0, length)
    };
    selection = Range {
        start: map(selection.start),
        end: map(selection.end),
    };
    SendMessageW(hwnd, EXSETSEL, 0, (&selection as *const Range) as isize);
    SendMessageW(hwnd, SETSCROLLPOS, 0, (&scroll as *const POINT) as isize);
    if visible {
        SendMessageW(hwnd, WM_SETREDRAW, 1, 0);
    }
    InvalidateRect(hwnd, null(), 0);
}
pub unsafe fn edit_style(hwnd: HWND, theme: Theme, size: f32, dpi: u32) {
    if super::scintilla::is_editor(hwnd) {
        super::scintilla::style(hwnd, theme, size, dpi);
        return;
    }
    SendMessageW(hwnd, SETBKGNDCOLOR, 0, theme.bg as isize);
    let actual = GetDpiForWindow(hwnd).max(96);
    // Plain-text Rich Edit rejects EM_SETZOOM on this Windows build. Apply
    // the monitor/simulation ratio to its single character format instead.
    SendMessageW(hwnd, SETZOOM, 1, 1);
    let mut cf = CharFormat {
        size: size_of::<CharFormat>() as u32,
        // Rich Edit can retain formatting from pasted text and earlier zooms.
        // Set size explicitly in twips, in every editor and at every DPI.
        mask: 0x68000000 | 0x80000000,
        effects: 0,
        height: (size * 20. * dpi as f32 / actual as f32).round() as i32,
        offset: 0,
        color: theme.text,
        charset: DEFAULT_CHARSET,
        pitch: 0,
        face: [0; 32],
    };
    let name = wide("Microsoft YaHei UI");
    cf.face[..name.len()].copy_from_slice(&name);
    SendMessageW(hwnd, SETCHARFORMAT, 0, (&cf as *const CharFormat) as isize);
    SendMessageW(hwnd, SETCHARFORMAT, 4, (&cf as *const CharFormat) as isize);
}
pub unsafe fn child(
    parent: HWND,
    class: &str,
    title: &str,
    style: u32,
    id: u16,
    font: HFONT,
) -> HWND {
    let h = CreateWindowExW(
        0,
        wide(class).as_ptr(),
        wide(title).as_ptr(),
        WS_CHILD | WS_VISIBLE | style,
        0,
        0,
        1,
        1,
        parent,
        id as usize as HMENU,
        GetModuleHandleW(null()),
        null(),
    );
    SendMessageW(h, WM_SETFONT, font as usize, 0);
    h
}
pub unsafe fn edit(parent: HWND, id: u16, font: HFONT, readonly: bool) -> HWND {
    let h = child(
        parent,
        "RICHEDIT50W",
        "",
        WS_TABSTOP
            | WS_VSCROLL
            | ES_MULTILINE as u32
            | ES_AUTOVSCROLL as u32
            | ES_WANTRETURN as u32
            | ES_NOHIDESEL as u32
            | if readonly { ES_READONLY as u32 } else { 0 },
        id,
        font,
    );
    SendMessageW(h, SETTEXTMODE, 1 | 8, 0);
    // These plain editors use one Unicode UI font. Automatic keyboard-driven
    // font binding reformats the entire document on the first CJK keystroke.
    // Keep the remaining language/IME options and normal glyph fallback.
    use windows::Win32::UI::Controls::RichEdit::{
        EM_GETLANGOPTIONS, EM_SETLANGOPTIONS, IMF_AUTOFONT,
    };
    let language = SendMessageW(h, EM_GETLANGOPTIONS, 0, 0);
    SendMessageW(h, EM_SETLANGOPTIONS, 0, language & !(IMF_AUTOFONT as isize));
    SendMessageW(h, SETUNDOLIMIT, 100, 0);
    SendMessageW(h, EXLIMITTEXT, 0, 4 * 1024 * 1024);
    SendMessageW(h, WM_USER + 69, 0, ENM_CHANGE as isize);
    h
}
// Rich Edit notifications omitted from windows-sys metadata.
pub const ENM_CHANGE: u32 = 1;
pub unsafe fn move_to(h: HWND, r: RECT) {
    if h.is_null() {
        return;
    }
    MoveWindow(
        h,
        r.left,
        r.top,
        (r.right - r.left).max(1),
        (r.bottom - r.top).max(1),
        0,
    );
    // Layout runs while the parent state is borrowed. Synchronous redraw
    // would send WM_DRAWITEM back into that parent before it can draw. Queue
    // repaint for every moved control (and its children) after layout instead.
    RedrawWindow(h, null(), null_mut(), RDW_INVALIDATE | RDW_ALLCHILDREN);
}
pub unsafe fn show_main(h: HWND) {
    let mut placement = WINDOWPLACEMENT {
        length: size_of::<WINDOWPLACEMENT>() as u32,
        ..Default::default()
    };
    GetWindowPlacement(h, &mut placement);
    ShowWindow(
        h,
        if IsIconic(h) != 0 {
            SW_RESTORE
        } else {
            SW_SHOW
        },
    );
    let mut bounds = RECT::default();
    GetWindowRect(h, &mut bounds);
    if (bounds.right - bounds.left < 240 || bounds.bottom - bounds.top < 100)
        && placement.rcNormalPosition.right - placement.rcNormalPosition.left >= 240
        && placement.rcNormalPosition.bottom - placement.rcNormalPosition.top >= 100
    {
        placement.showCmd = SW_SHOWNORMAL as u32;
        SetWindowPlacement(h, &placement);
    }
    SetForegroundWindow(h);
}
pub unsafe fn menu(owner: HWND, items: &[(u16, &str)]) -> u16 {
    menu_checked(owner, items, &[])
}
pub unsafe fn menu_checked(owner: HWND, items: &[(u16, &str)], checked: &[u16]) -> u16 {
    let m = CreatePopupMenu();
    for &(id, s) in items {
        if id == 0 {
            AppendMenuW(m, MF_SEPARATOR, 0, null());
        } else {
            AppendMenuW(
                m,
                MF_STRING | if checked.contains(&id) { MF_CHECKED } else { 0 },
                id as usize,
                wide(s).as_ptr(),
            );
        }
    }
    let mut p = POINT::default();
    GetCursorPos(&mut p);
    let result = TrackPopupMenu(
        m,
        TPM_RETURNCMD | TPM_RIGHTBUTTON,
        p.x,
        p.y,
        0,
        owner,
        null(),
    ) as u16;
    DestroyMenu(m);
    result
}
pub unsafe fn error(owner: HWND, s: &str) {
    MessageBoxW(
        owner,
        wide(s).as_ptr(),
        wide("轻取 OCR").as_ptr(),
        MB_ICONINFORMATION,
    );
}
pub fn dimensions(h: HWND) -> (i32, i32) {
    unsafe {
        let mut r = RECT::default();
        GetClientRect(h, &mut r);
        (r.right, r.bottom)
    }
}
pub unsafe fn set_top(h: HWND, top: bool) {
    SetWindowPos(
        h,
        if top { HWND_TOPMOST } else { HWND_NOTOPMOST },
        0,
        0,
        0,
        0,
        SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE,
    );
}

/// Draw a dirty rectangle offscreen and publish it with one blit. The window
/// DC retains its update/child clipping; intermediate background clears never
/// reach the screen. Allocation is proportional to the dirty rectangle.
pub struct PaintBuffer {
    bitmap: Bitmap,
    bounds: RECT,
}
impl PaintBuffer {
    pub unsafe fn new(bounds: RECT) -> Option<Self> {
        let width = bounds.right - bounds.left;
        let height = bounds.bottom - bounds.top;
        if width <= 0 || height <= 0 {
            return None;
        }
        let bitmap = Bitmap::new(width, height).ok()?;
        SetViewportOrgEx(bitmap.dc, -bounds.left, -bounds.top, null_mut());
        IntersectClipRect(
            bitmap.dc,
            bounds.left,
            bounds.top,
            bounds.right,
            bounds.bottom,
        );
        Some(Self { bitmap, bounds })
    }
    pub fn dc(&self) -> HDC {
        self.bitmap.dc
    }
    pub unsafe fn present(&self, target: HDC) {
        let r = self.bounds;
        BitBlt(
            target,
            r.left,
            r.top,
            r.right - r.left,
            r.bottom - r.top,
            self.bitmap.dc,
            r.left,
            r.top,
            SRCCOPY,
        );
    }
}

pub struct Bitmap {
    pub dc: HDC,
    bitmap: HBITMAP,
    old: HGDIOBJ,
    pub pixels: *mut u8,
}
impl Bitmap {
    pub fn new(width: i32, height: i32) -> Result<Self, String> {
        unsafe {
            let dc = CreateCompatibleDC(null_mut());
            let mut pixels = null_mut();
            let info = BITMAPINFO {
                bmiHeader: BITMAPINFOHEADER {
                    biSize: size_of::<BITMAPINFOHEADER>() as u32,
                    biWidth: width,
                    biHeight: -height,
                    biPlanes: 1,
                    biBitCount: 32,
                    biCompression: BI_RGB,
                    ..Default::default()
                },
                ..Default::default()
            };
            let bitmap = CreateDIBSection(dc, &info, DIB_RGB_COLORS, &mut pixels, null_mut(), 0);
            if dc.is_null() || bitmap.is_null() {
                if !bitmap.is_null() {
                    DeleteObject(bitmap);
                }
                if !dc.is_null() {
                    DeleteDC(dc);
                }
                return Err("无法分配绘图位图。".into());
            }
            let old = SelectObject(dc, bitmap);
            if old.is_null() || old == -1isize as HGDIOBJ {
                DeleteObject(bitmap);
                DeleteDC(dc);
                return Err("无法初始化绘图位图。".into());
            }
            Ok(Self {
                dc,
                bitmap,
                old,
                pixels: pixels.cast(),
            })
        }
    }
}
impl Drop for Bitmap {
    fn drop(&mut self) {
        unsafe {
            SelectObject(self.dc, self.old);
            DeleteObject(self.bitmap);
            DeleteDC(self.dc);
        }
    }
}
