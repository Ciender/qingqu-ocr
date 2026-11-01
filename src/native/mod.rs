//! Native Windows UI: Scintilla plain editors, Rich Edit document preview and GDI.
mod auxiliary;
mod backend;
mod controls;
mod editor_view;
mod history;
mod icons;
mod main_window;
mod paragraph_menu;
mod picture;
mod picture_shadow;
mod picture_zoom;
mod rendered;
mod scintilla;
mod service_help;
mod settings;
mod table_layout;
pub mod win;

use crate::{config, platform};
use auxiliary::{Find, Note, Preview, Tables};
use backend::{Notice, Proxy};
use icons::{Atlas, Icon};
use main_window::Main;
use picture::Picture;
use settings::SettingsEditor;
use std::{
    cell::RefCell,
    mem::size_of,
    ptr::{null, null_mut},
    rc::Rc,
};
use win::*;

pub const WAKE: u32 = WM_APP + 10;
pub const COMMAND: u32 = WM_APP + 11;
pub const EXIT: u32 = WM_APP + 12;
pub const APPLY_THEME: u32 = WM_APP + 13;
pub const LAYOUT: u32 = WM_APP + 14;
pub const CLOSE: u16 = 10;
pub const MAX: u16 = 11;
pub const MIN: u16 = 12;
pub const PIN: u16 = 13;
pub const SETTINGS: u16 = 14;
pub const LINES: u16 = 15;
pub const CAPTURE: u16 = 100;
pub const WINDOW: u16 = 101;
pub const CLIPBOARD: u16 = 102;
pub const OPEN: u16 = 103;
pub const FORMAT: u16 = 104;
pub const SPLIT: u16 = 105;
pub const MERGE: u16 = 106;
pub const PUNCTUATION: u16 = 107;
pub const CHINESE: u16 = 108;
pub const TRANSLATE: u16 = 109;
pub const COPY: u16 = 110;
pub const CLEAR: u16 = 111;
pub const UNDO: u16 = 112;
pub const REDO: u16 = 113;
pub const SEARCH: u16 = 114;
pub const TOGGLE: u16 = 115;
pub const CANCEL: u16 = 116;
pub const ZOOM_IN: u16 = 117;
pub const ZOOM_OUT: u16 = 118;
pub const ZOOM_RESET: u16 = 119;
pub const TABLES: u16 = 120;
pub const MENU: u16 = 121;
pub const PASTE: u16 = 122;
pub const CUT: u16 = 123;
pub const RESTORE: u16 = 124;
pub const NOTE: u16 = 125;
pub const TABLE_CAPTURE: u16 = 126;
pub const VERTICAL_CAPTURE: u16 = 127;
pub const FORMULA_CAPTURE: u16 = 128;
pub const LATEX_PREVIEW: u16 = 129;
pub const MARKDOWN_PREVIEW: u16 = 130;
pub const PARAGRAPH: u16 = 131;
pub const AUTO_PARAGRAPH: u16 = 132;
pub const AUTO_TRANSLATE: u16 = 133;
pub const PICTURE_ZOOM: u32 = WM_APP + 22;
pub const PICTURE_INFO: u32 = WM_APP + 23;

pub enum Content {
    Main(Box<Main>),
    Note(Note),
    Picture(Picture),
    Settings(Box<SettingsEditor>),
    Toolbar,
    Preview(Preview),
    Tables(Tables),
    Find(Find),
    ServiceHelp(service_help::ServiceHelp),
    PictureZoom(picture_zoom::Zoom),
}
impl Content {
    fn role(&self) -> usize {
        match self {
            Self::Main(_) => 1,
            Self::Note(_) => 2,
            Self::Picture(_) => 3,
            Self::Settings(_) => 4,
            Self::Toolbar => 5,
            Self::Preview(_) => 6,
            Self::Tables(_) => 7,
            Self::Find(_) => 8,
            Self::ServiceHelp(_) => 9,
            Self::PictureZoom(_) => 10,
        }
    }
    fn init(&mut self, h: &mut Host) -> Result<(), String> {
        match self {
            Self::Main(s) => s.init(h),
            Self::Note(s) => s.init(h),
            Self::Picture(s) => s.init(h),
            Self::Settings(s) => s.init(h),
            Self::Preview(s) => s.init(h),
            Self::Tables(s) => s.init(h),
            Self::Find(s) => s.init(h),
            Self::ServiceHelp(s) => s.init(h),
            Self::PictureZoom(s) => s.init(h),
            Self::Toolbar => {
                unsafe {
                    for (id, icon, tip) in [
                        (CAPTURE, Icon::Text, "区域截图"),
                        (CLIPBOARD, Icon::Clipboard, "当前剪贴板 · Alt+V"),
                        (TRANSLATE, Icon::Translate, "翻译原文或选区"),
                        (TOGGLE, Icon::Window, "打开主窗口"),
                        (CLOSE, Icon::Close, "隐藏工具条"),
                    ] {
                        h.button(id, Some(icon), tip);
                    }
                }
                Ok(())
            }
        }
    }
    fn layout(&mut self, h: &mut Host) {
        match self {
            Self::Main(s) => s.layout(h),
            Self::Note(s) => s.layout(h),
            Self::Picture(s) => s.layout(h),
            Self::Settings(s) => s.layout(h),
            Self::Preview(s) => s.layout(h),
            Self::Tables(s) => s.layout(h),
            Self::Find(s) => s.layout(h),
            Self::ServiceHelp(s) => s.layout(h),
            Self::PictureZoom(s) => s.layout(h),
            Self::Toolbar => {
                let d = h.px(34.);
                let x = h.px(22.);
                for (i, b) in h.buttons.iter().enumerate() {
                    unsafe {
                        move_to(b.hwnd, rect(x + i as i32 * d, 0, d, h.px(30.)));
                    }
                }
            }
        }
    }
    fn command(&mut self, h: &mut Host, id: u16, code: u16, from: HWND) {
        match self {
            Self::Main(s) => s.command(h, id, code, from),
            Self::Note(s) => s.command(h, id, code),
            Self::Picture(s) => s.command(h, id),
            Self::Settings(s) => s.command(h, id, code),
            Self::Preview(s) => s.command(h, id),
            Self::Tables(s) => s.command(h, id, code),
            Self::Find(s) => s.command(h, id),
            Self::ServiceHelp(s) => s.command(h, id),
            Self::PictureZoom(s) => s.command(h, id),
            Self::Toolbar => {
                if let Some(p) = &h.proxy {
                    p.action(match id {
                        CAPTURE => "capture",
                        WINDOW => "window",
                        CLIPBOARD => "clipboard",
                        TRANSLATE => "translate",
                        CLOSE => "hide-toolbar",
                        _ => "show",
                    });
                }
            }
        }
    }
    unsafe fn paint(&self, h: &Host, dc: HDC) {
        match self {
            Self::Main(s) => s.paint(h, dc),
            Self::Picture(s) => s.paint(h, dc),
            Self::Settings(s) => s.paint(h, dc),
            Self::Preview(s) => s.paint(h, dc),
            Self::ServiceHelp(s) => s.paint(h, dc),
            Self::Toolbar => {
                // Use the same Fluent SVG as the other controls; keep the drag target.
                if let Some(atlas) = &h.small {
                    atlas.paint(dc, rect(0, 0, h.px(22.), h.px(30.)), Icon::Grip);
                }
            }
            _ => {}
        }
    }
    fn close(&mut self, h: &mut Host) -> bool {
        match self {
            Self::Main(s) => s.close(h),
            Self::Picture(s) => {
                s.stop_motion(h);
                true
            }
            Self::Settings(_) => {
                if let Some(p) = &h.proxy {
                    p.send(Notice::Record(false));
                }
                true
            }
            _ => true,
        }
    }
    fn theme(&mut self, h: &mut Host) {
        match self {
            Self::Main(s) => s.theme(h),
            Self::Note(s) => s.theme(h),
            Self::Settings(s) => s.theme(h),
            Self::Preview(s) => s.theme(h),
            Self::Tables(s) => s.theme(h),
            Self::Picture(s) => s.layout(h),
            _ => {}
        }
    }
}
pub struct Button {
    pub id: u16,
    pub hwnd: HWND,
    pub icon: Option<Icon>,
    pub text: String,
    pub active: bool,
    _tip: Vec<u16>,
}
pub struct Host {
    pub hwnd: HWND,
    pub root: usize,
    pub proxy: Option<Proxy>,
    pub theme: Theme,
    pub palette: String,
    pub dark: bool,
    pub dpi: u32,
    pub font: Font,
    pub editor_font: Font,
    pub font_size: f32,
    pub bg: Brush,
    pub surface: Brush,
    pub buttons: Vec<Button>,
    pub controls: Vec<HWND>,
    pub editors: Vec<HWND>,
    pub tooltip: HWND,
    pub title: String,
    pub top: bool,
    pub resizable: bool,
    pub min_size: (f32, f32),
    small: Option<Atlas>,
    large: Option<Atlas>,
}
impl Host {
    pub fn px(&self, n: f32) -> i32 {
        (n * self.dpi as f32 / 96.).round() as i32
    }
    pub unsafe fn button(&mut self, id: u16, icon: Option<Icon>, tip: &str) -> HWND {
        let hwnd = child(
            self.hwnd,
            "BUTTON",
            if icon.is_none() { tip } else { "" },
            BS_OWNERDRAW as u32 | WS_TABSTOP,
            id,
            self.font.0,
        );
        SetWindowSubclass(hwnd, Some(control_proc), id as usize, self.hwnd as usize);
        let mut label = platform::wide(tip);
        if !self.tooltip.is_null() {
            let tool = TTTOOLINFOW {
                cbSize: size_of::<TTTOOLINFOW>() as u32,
                uFlags: TTF_IDISHWND,
                hwnd: self.hwnd,
                uId: hwnd as usize,
                lpszText: label.as_mut_ptr(),
                ..Default::default()
            };
            SendMessageW(
                self.tooltip,
                TTM_ADDTOOLW,
                0,
                (&tool as *const TTTOOLINFOW) as isize,
            );
        }
        self.buttons.push(Button {
            id,
            hwnd,
            icon,
            text: tip.into(),
            active: false,
            _tip: label,
        });
        self.controls.push(hwnd);
        hwnd
    }
    pub unsafe fn button_tip(&mut self, id: u16, icon: Icon, tip: &str) {
        if let Some(b) = self.buttons.iter_mut().find(|b| b.id == id) {
            if b.text == tip {
                return;
            }
            b.icon = Some(icon);
            b.text = tip.into();
            b._tip = platform::wide(tip);
            let tool = TTTOOLINFOW {
                cbSize: size_of::<TTTOOLINFOW>() as u32,
                hwnd: self.hwnd,
                uId: b.hwnd as usize,
                lpszText: b._tip.as_mut_ptr(),
                ..Default::default()
            };
            SendMessageW(
                self.tooltip,
                TTM_UPDATETIPTEXTW,
                0,
                (&tool as *const TTTOOLINFOW) as isize,
            );
            InvalidateRect(b.hwnd, null(), 0);
        }
    }
    #[allow(dead_code)] // Retained for future dynamic action labels.
    pub unsafe fn button_label(&mut self, id: u16, label: &str) {
        if let Some(button) = self.buttons.iter_mut().find(|b| b.id == id) {
            button.text = label.into();
            set_text(button.hwnd, label);
            InvalidateRect(button.hwnd, null(), 0);
        }
    }
    pub unsafe fn rich(&mut self, id: u16, readonly: bool) -> HWND {
        let hwnd = if matches!(id, 200 | 201 | 220 | 230 | 231) {
            scintilla::create(self.hwnd, id, readonly)
        } else {
            edit(self.hwnd, id, self.editor_font.0, readonly)
        };
        edit_style(hwnd, self.theme, self.font_size, self.dpi);
        SetWindowSubclass(hwnd, Some(control_proc), id as usize, self.hwnd as usize);
        SetPropW(hwnd, platform::wide("QingquEditor").as_ptr(), 1 as HANDLE);
        self.editors.push(hwnd);
        self.controls.push(hwnd);
        hwnd
    }
    pub unsafe fn control(&mut self, class: &str, title: &str, style: u32, id: u16) -> HWND {
        let style = style
            | if class == "COMBOBOX" {
                CBS_OWNERDRAWFIXED as u32 | CBS_HASSTRINGS as u32
            } else if class == "LISTBOX" && id == 500 {
                LBS_OWNERDRAWFIXED as u32 | LBS_HASSTRINGS as u32
            } else {
                0
            };
        let hwnd = child(self.hwnd, class, title, style, id, self.font.0);
        controls::install(hwnd, class, id);
        self.controls.push(hwnd);
        hwnd
    }
    pub unsafe fn place(&self, id: u16, x: f32, y: f32, w: f32, height: f32) {
        let hwnd = GetDlgItem(self.hwnd, id as i32);
        if !hwnd.is_null() {
            move_to(
                hwnd,
                rect(self.px(x), self.px(y), self.px(w), self.px(height)),
            );
        }
    }
    pub unsafe fn title_buttons(&mut self, settings: bool, lines: bool) {
        self.button(CLOSE, Some(Icon::Close), "关闭");
        self.button(MAX, Some(Icon::Max), "最大化 / 还原");
        self.button(MIN, Some(Icon::Min), "最小化");
        self.button(
            PIN,
            Some(if self.top { Icon::Pin } else { Icon::Unpin }),
            "置顶 / 取消置顶",
        );
        if settings {
            self.button(SETTINGS, Some(Icon::Settings), "设置");
        }
        if lines {
            self.button(LINES, Some(Icon::Lines), "显示 / 隐藏行号");
        }
    }
    pub fn layout_title(&self) {
        let (w, _) = dimensions(self.hwnd);
        let mut x = w;
        for id in [CLOSE, MAX, MIN, SETTINGS, PIN, LINES] {
            if let Some(b) = self.buttons.iter().find(|b| b.id == id) {
                x -= self.px(22.);
                unsafe {
                    move_to(b.hwnd, rect(x, 0, self.px(22.), self.px(22.)));
                }
            }
        }
    }
    pub fn invalidate(&self) {
        unsafe {
            InvalidateRect(self.hwnd, null(), 0);
        }
    }
    pub fn change_theme(&mut self, palette: &str, dark: bool) {
        unsafe {
            SetPropW(
                self.hwnd,
                platform::wide("QingquTheme").as_ptr(),
                (1 + usize::from(dark) + if palette == "catppuccin" { 2 } else { 0 }) as HANDLE,
            );
        }
        self.palette = palette.into();
        self.dark = dark;
        self.theme = Theme::new(palette, dark);
        self.bg = Brush::new(self.theme.bg);
        self.surface = Brush::new(self.theme.surface);
        self.small = Atlas::new(self.px(18.), self.theme.muted).ok();
        self.large = Atlas::new(self.px(25.), self.theme.text).ok();
        unsafe {
            window_frame(self.hwnd);
        }
        for &editor in &self.editors {
            unsafe {
                edit_style(editor, self.theme, self.font_size, self.dpi);
            }
        }
        for &control in &self.controls {
            unsafe {
                InvalidateRect(control, null(), 1);
            }
        }
        self.invalidate();
    }
    pub fn change_dpi(&mut self, dpi: u32) {
        self.dpi = dpi.max(96);
        unsafe {
            SetPropW(
                self.hwnd,
                platform::wide("QingquDpi").as_ptr(),
                self.dpi as usize as HANDLE,
            );
        }
        let ui = Font::new(12., self.dpi);
        let editor = Font::editor(self.font_size, self.dpi);
        for &control in &self.controls {
            unsafe {
                SendMessageW(
                    control,
                    WM_SETFONT,
                    if self.editors.contains(&control) {
                        editor.0
                    } else {
                        ui.0
                    } as usize,
                    1,
                );
            }
        }
        self.font = ui;
        self.editor_font = editor;
        let p = self.palette.clone();
        self.change_theme(&p, self.dark);
    }
    pub fn set_font_size(&mut self, size: f32) {
        self.font_size = size.clamp(8., 48.);
        let font = Font::editor(self.font_size, self.dpi);
        for &editor in &self.editors {
            unsafe {
                SendMessageW(editor, WM_SETFONT, font.0 as usize, 1);
            }
        }
        self.editor_font = font;
    }
}
struct Window {
    host: Host,
    content: Content,
}

pub fn create(
    title: &str,
    size: (f32, f32),
    content: Content,
    proxy: Option<Proxy>,
    palette: &str,
    dark: bool,
    font_size: f32,
) -> Result<HWND, String> {
    unsafe {
        let dpi = proxy.as_ref().map_or_else(
            || GetDpiForSystem().max(96),
            |p| GetDpiForWindow(p.notify.hwnd as HWND).max(96),
        );
        let theme = Theme::new(palette, dark);
        let role = content.role();
        let top = match &content {
            Content::Main(main) => main.settings.default_topmost,
            _ => true,
        };
        let host = Host {
            hwnd: null_mut(),
            root: proxy.as_ref().map_or(0, |p| p.notify.hwnd),
            proxy,
            theme,
            palette: palette.into(),
            dark,
            dpi,
            font: Font::new(12., dpi),
            editor_font: Font::editor(font_size, dpi),
            font_size,
            bg: Brush::new(theme.bg),
            surface: Brush::new(theme.surface),
            buttons: Vec::new(),
            controls: Vec::new(),
            editors: Vec::new(),
            tooltip: null_mut(),
            title: title.into(),
            top,
            resizable: !matches!(role, 3 | 5 | 9 | 10),
            min_size: match role {
                1 => (300., 220.),
                2 => (240., 160.),
                3 => (1., 1.),
                4 => (540., 360.),
                5 => (180., 30.),
                10 => (300., 145.),
                _ => (380., 220.),
            },
            small: None,
            large: None,
        };
        let width = host.px(size.0);
        let height = host.px(size.1);
        let window = Rc::new(RefCell::new(Window { host, content }));
        let raw = Rc::into_raw(window.clone());
        let hwnd = CreateWindowExW(
            (if top || matches!(role, 5 | 9) {
                WS_EX_TOPMOST
            } else {
                0
            }) | (if role == 9 { WS_EX_NOACTIVATE } else { 0 })
                | (if role == 3 { WS_EX_LAYERED } else { 0 })
                | if matches!(role, 5 | 9) {
                    WS_EX_TOOLWINDOW | WS_EX_TOPMOST
                } else {
                    0
                },
            platform::wide("QingquOCR.Native.Window").as_ptr(),
            platform::wide(title).as_ptr(),
            WS_POPUP
                | WS_CLIPCHILDREN
                | if role == 3 {
                    WS_THICKFRAME
                } else if !matches!(role, 5 | 9 | 10) {
                    WS_THICKFRAME | WS_MINIMIZEBOX | WS_MAXIMIZEBOX
                } else {
                    0
                },
            CW_USEDEFAULT,
            CW_USEDEFAULT,
            width,
            height,
            null_mut(),
            null_mut(),
            GetModuleHandleW(null()),
            raw.cast(),
        );
        if hwnd.is_null() {
            drop(Rc::from_raw(raw));
            return Err("无法创建原生窗口。".into());
        }
        SetPropW(hwnd, platform::wide("QingquRole").as_ptr(), role as HANDLE);
        custom_frame(hwnd);
        if role == 1 {
            // Toolbar creation happens during Main::init, so position its
            // owner first instead of anchoring it to the default origin.
            place_beside(hwnd, null_mut());
        }
        {
            let mut window = window.borrow_mut();
            let Window { host, content } = &mut *window;
            host.hwnd = hwnd;
            if host.root == 0 {
                host.root = hwnd as usize;
            }
            host.tooltip = CreateWindowExW(
                WS_EX_TOPMOST,
                TOOLTIPS_CLASSW,
                null(),
                WS_POPUP | TTS_ALWAYSTIP,
                0,
                0,
                0,
                0,
                hwnd,
                null_mut(),
                GetModuleHandleW(null()),
                null(),
            );
            SendMessageW(host.tooltip, TTM_SETMAXTIPWIDTH, 0, host.px(360.) as isize);
            SetPropW(
                hwnd,
                platform::wide("QingquTooltip").as_ptr(),
                host.tooltip as HANDLE,
            );
            SendMessageW(host.tooltip, TTM_SETDELAYTIME, TTDT_INITIAL as usize, 450);
            SendMessageW(host.tooltip, TTM_SETDELAYTIME, TTDT_RESHOW as usize, 100);
            SendMessageW(host.tooltip, TTM_SETDELAYTIME, TTDT_AUTOPOP as usize, 12000);
            host.change_dpi(GetDpiForWindow(hwnd).max(96));
            icons::icon_to_window(hwnd);
            if let Err(e) = content.init(host) {
                drop(window);
                DestroyWindow(hwnd);
                return Err(e);
            }
            host.layout_title();
            content.layout(host);
            set_top(hwnd, host.top);
        }
        platform::square_owned_windows();
        if role != 1 && role != 9 {
            let root = window.borrow().host.root as HWND;
            place_beside(hwnd, root);
        } else if role == 1 {
            place_beside(hwnd, null_mut());
        }
        ShowWindow(
            hwnd,
            if role == 9 {
                SW_SHOWNOACTIVATE
            } else {
                SW_SHOW
            },
        );
        Ok(hwnd)
    }
}

/// Prefer the owner's right, then left, and keep every new window on its monitor.
unsafe fn place_beside(hwnd: HWND, owner: HWND) {
    let mut bounds = RECT::default();
    GetWindowRect(hwnd, &mut bounds);
    let mut monitor = MONITORINFO {
        cbSize: size_of::<MONITORINFO>() as u32,
        ..Default::default()
    };
    GetMonitorInfoW(
        MonitorFromWindow(
            if owner.is_null() { hwnd } else { owner },
            MONITOR_DEFAULTTONEAREST,
        ),
        &mut monitor,
    );
    let area = monitor.rcWork;
    let width = (bounds.right - bounds.left).min(area.right - area.left);
    let height = (bounds.bottom - bounds.top).min(area.bottom - area.top);
    let (x, y) = if !owner.is_null() && GetWindowRect(owner, &mut bounds) != 0 {
        let x = if bounds.right + 12 + width <= area.right {
            bounds.right + 12
        } else if bounds.left - 12 - width >= area.left {
            bounds.left - 12 - width
        } else {
            bounds.left + 32
        };
        (x, bounds.top + 24)
    } else {
        (
            area.left + (area.right - area.left - width) / 2,
            area.top + (area.bottom - area.top - height) / 2,
        )
    };
    SetWindowPos(
        hwnd,
        null_mut(),
        x.clamp(area.left, area.right - width),
        y.clamp(area.top, area.bottom - height),
        width,
        height,
        SWP_NOZORDER | SWP_NOACTIVATE,
    );
}

unsafe extern "system" fn window_proc(hwnd: HWND, msg: u32, w: WPARAM, l: LPARAM) -> LRESULT {
    if !GetPropW(hwnd, platform::wide("QingquServiceHelp").as_ptr()).is_null() {
        if msg == WM_MOUSEMOVE {
            let mut tracking = TRACKMOUSEEVENT {
                cbSize: size_of::<TRACKMOUSEEVENT>() as u32,
                dwFlags: TME_LEAVE,
                hwndTrack: hwnd,
                dwHoverTime: 0,
            };
            TrackMouseEvent(&mut tracking);
        } else if msg == WM_MOUSELEAVE {
            PostMessageW(hwnd, service_help::DISMISS, 0, 0);
        }
    }
    // Native menus draw synchronously while the main command is borrowed.
    if let Some(result) = paragraph_menu::message(hwnd, msg, l) {
        return result;
    }
    if msg == WM_NCCREATE {
        let cs = &*(l as *const CREATESTRUCTW);
        SetWindowLongPtrW(hwnd, GWLP_USERDATA, cs.lpCreateParams as isize);
        let cell = &*(cs.lpCreateParams as *const RefCell<Window>);
        if let Ok(mut s) = cell.try_borrow_mut() {
            s.host.hwnd = hwnd;
        }
    }
    let ptr = GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *const RefCell<Window>;
    if ptr.is_null() {
        return DefWindowProcW(hwnd, msg, w, l);
    }
    if msg == WM_NCCALCSIZE {
        let r = if w != 0 {
            &mut (*(l as *mut NCCALCSIZE_PARAMS)).rgrc[0]
        } else {
            &mut *(l as *mut RECT)
        };
        let frame = window_frame_width(hwnd);
        InflateRect(r, -frame, -frame);
        return 0;
    }
    if let Some(result) = picture::paint_prepared(hwnd, msg, w) {
        return result;
    }
    if msg == WM_WINDOWPOSCHANGED && !picture::is_prepared(hwnd) {
        // Also runs while content is borrowed during drag, zoom and hide/show.
        picture_shadow::sync(hwnd);
    }
    // These messages also arrive reentrantly while content is borrowed. Never
    // let DefWindowProc restore the native recessed frame on that path.
    if msg == WM_NCACTIVATE {
        SetPropW(
            hwnd,
            platform::wide("QingquActive").as_ptr(),
            usize::from(w != 0) as HANDLE,
        );
        window_frame(hwnd);
        return 1;
    }
    if msg == WM_NCPAINT {
        window_frame(hwnd);
        return 0;
    }
    if msg == WM_ERASEBKGND {
        return 1;
    }
    if msg == WM_NOTIFY {
        scintilla::notification(l);
    }
    // Text notifications arrive even while a command borrows the parent.
    // Keep edit metadata current on this reentrant path too.
    if msg == WM_COMMAND
        && (w >> 16) as u16 == EN_CHANGE as u16
        && l != 0
        && scintilla::is_editor(l as HWND)
    {
        scintilla::changed(l as HWND);
    }
    // A minimized window reports its icon dimensions, not the editor's
    // reading area. Keep child geometry until the restored size arrives.
    if (msg == WM_SIZE && w == SIZE_MINIMIZED as usize) || (msg == LAYOUT && IsIconic(hwnd) != 0) {
        return 0;
    }
    // Keep a strong reference across synchronous, reentrant Windows messages.
    Rc::increment_strong_count(ptr);
    let cell = Rc::from_raw(ptr);
    if msg == WM_NCDESTROY {
        picture_shadow::forget(hwnd);
        SetWindowLongPtrW(hwnd, GWLP_USERDATA, 0);
        RemovePropW(hwnd, platform::wide("QingquRole").as_ptr());
        if let Ok(s) = cell.try_borrow() {
            if let Some(p) = &s.host.proxy {
                if s.host.root != hwnd as usize {
                    p.send(Notice::ChildClosed(hwnd as usize));
                }
            }
        }
        drop(Rc::from_raw(ptr));
        return DefWindowProcW(hwnd, msg, w, l);
    }
    if msg == WM_DESTROY {
        if GetPropW(hwnd, platform::wide("QingquRole").as_ptr()) as usize == 1 {
            PostQuitMessage(0);
        }
        return 0;
    }
    if msg == WM_PRINT {
        if picture::print_full(hwnd, w as HDC) {
            return 0;
        }
        let result = DefWindowProcW(hwnd, msg, w, l);
        if l as i32 & PRF_NONCLIENT != 0 {
            paint_window_frame(hwnd, w as HDC);
        }
        return result;
    }
    let Ok(mut window) = cell.try_borrow_mut() else {
        if msg == WM_SIZE && !picture::is_prepared(hwnd) {
            PostMessageW(hwnd, LAYOUT, 0, 0);
        }
        return DefWindowProcW(hwnd, msg, w, l);
    };
    let Window { host, content } = &mut *window;
    match msg {
        WM_NCCALCSIZE => return 0,
        WM_ERASEBKGND => return 1,
        WM_GETMINMAXINFO => {
            let info = &mut *(l as *mut MINMAXINFO);
            info.ptMinTrackSize = POINT {
                x: host.px(host.min_size.0),
                y: host.px(host.min_size.1),
            };
            let monitor = MonitorFromWindow(hwnd, MONITOR_DEFAULTTONEAREST);
            let mut bounds = MONITORINFO {
                cbSize: size_of::<MONITORINFO>() as u32,
                ..Default::default()
            };
            if GetMonitorInfoW(monitor, &mut bounds) != 0 {
                info.ptMaxPosition = POINT {
                    x: bounds.rcWork.left - bounds.rcMonitor.left,
                    y: bounds.rcWork.top - bounds.rcMonitor.top,
                };
                info.ptMaxSize = POINT {
                    x: bounds.rcWork.right - bounds.rcWork.left,
                    y: bounds.rcWork.bottom - bounds.rcWork.top,
                };
            }
            return 0;
        }
        WM_NCHITTEST => {
            let mut p = POINT {
                x: (l as u32 & 65535) as i16 as i32,
                y: ((l as u32 >> 16) & 65535) as i16 as i32,
            };
            if matches!(content, Content::Picture(_)) {
                let bounds = picture::visible_bounds(hwnd);
                if PtInRect(&bounds, p) == 0 {
                    return HTTRANSPARENT as isize;
                }
            }
            ScreenToClient(hwnd, &mut p);
            let (width, height) = dimensions(hwnd);
            let edge = host.px(4.);
            if host.resizable && IsZoomed(hwnd) == 0 {
                let (left, right, top, bottom) = (
                    p.x < edge,
                    p.x >= width - edge,
                    p.y < edge,
                    p.y >= height - edge,
                );
                let hit = match (left, right, top, bottom) {
                    (true, _, true, _) => HTTOPLEFT,
                    (_, true, true, _) => HTTOPRIGHT,
                    (true, _, _, true) => HTBOTTOMLEFT,
                    (_, true, _, true) => HTBOTTOMRIGHT,
                    (true, _, _, _) => HTLEFT,
                    (_, true, _, _) => HTRIGHT,
                    (_, _, true, _) => HTTOP,
                    (_, _, _, true) => HTBOTTOM,
                    _ => HTCLIENT,
                };
                if hit != HTCLIENT {
                    return hit as isize;
                }
            }
            if matches!(content, Content::Toolbar) {
                return if p.x < host.px(22.) {
                    HTCAPTION
                } else {
                    HTCLIENT
                } as isize;
            }
            if matches!(content, Content::Picture(_)) {
                // Picture dragging uses capture, avoiding Windows' caption/top
                // work-area clamp and snap behavior on borderless image pins.
                return HTCLIENT as isize;
            }
            if p.y < host.px(22.)
                && !host.buttons.iter().filter(|b| b.id < 100).any(|b| {
                    let mut r = RECT::default();
                    GetWindowRect(b.hwnd, &mut r);
                    let mut point = p;
                    ClientToScreen(hwnd, &mut point);
                    PtInRect(&r, point) != 0
                })
            {
                return HTCAPTION as isize;
            }
            return HTCLIENT as isize;
        }
        WM_SIZE | LAYOUT => {
            host.layout_title();
            content.layout(host);
            host.invalidate();
            return 0;
        }
        WM_DPICHANGED => {
            host.change_dpi(w as u32 & 65535);
            let r = &*(l as *const RECT);
            SetWindowPos(
                hwnd,
                null_mut(),
                r.left,
                r.top,
                r.right - r.left,
                r.bottom - r.top,
                SWP_NOZORDER | SWP_NOACTIVATE,
            );
            host.layout_title();
            content.layout(host);
            return 0;
        }
        WM_PAINT => {
            let mut ps = PAINTSTRUCT::default();
            let target = BeginPaint(hwnd, &mut ps);
            let buffer = if matches!(content, Content::Main(_) | Content::Note(_)) {
                PaintBuffer::new(ps.rcPaint)
            } else {
                None
            };
            let dc = buffer.as_ref().map_or(target, PaintBuffer::dc);
            let (width, height) = dimensions(hwnd);
            if !matches!(content, Content::Picture(_)) {
                fill(dc, &rect(0, 0, width, height), host.theme.bg);
            }
            if !matches!(content, Content::Toolbar | Content::Picture(_)) {
                fill(dc, &rect(0, 0, width, host.px(22.)), host.theme.surface);
                let controls =
                    host.buttons.iter().filter(|b| b.id < 100).count() as i32 * host.px(22.);
                label(
                    dc,
                    &rect(
                        host.px(7.),
                        0,
                        (width - controls - host.px(12.)).max(0),
                        host.px(22.),
                    ),
                    &host.title,
                    host.theme.muted,
                    host.font.0,
                    DT_SINGLELINE | DT_VCENTER | DT_END_ELLIPSIS,
                );
            }
            content.paint(host, dc);
            if let Some(buffer) = buffer {
                buffer.present(target);
            }
            EndPaint(hwnd, &ps);
            return 0;
        }
        WM_PRINTCLIENT => {
            let dc = w as HDC;
            let (width, height) = dimensions(hwnd);
            if !matches!(content, Content::Picture(_)) {
                fill(dc, &rect(0, 0, width, height), host.theme.bg);
            }
            if !matches!(content, Content::Toolbar | Content::Picture(_)) {
                fill(dc, &rect(0, 0, width, host.px(22.)), host.theme.surface);
                let count = host.buttons.iter().filter(|b| b.id < 100).count() as i32;
                label(
                    dc,
                    &rect(
                        host.px(7.),
                        0,
                        (width - count * host.px(22.) - host.px(12.)).max(0),
                        host.px(22.),
                    ),
                    &host.title,
                    host.theme.muted,
                    host.font.0,
                    DT_SINGLELINE | DT_VCENTER | DT_END_ELLIPSIS,
                );
            }
            content.paint(host, dc);
            return 0;
        }
        WM_DRAWITEM => {
            let draw = &*(l as *const DRAWITEMSTRUCT);
            if controls::draw_item(host, draw) {
                return 1;
            }
            if let Some(b) = host.buttons.iter().find(|b| b.id as u32 == draw.CtlID) {
                let saved = SaveDC(draw.hDC);
                IntersectClipRect(
                    draw.hDC,
                    draw.rcItem.left,
                    draw.rcItem.top,
                    draw.rcItem.right,
                    draw.rcItem.bottom,
                );
                let mut p = POINT::default();
                GetCursorPos(&mut p);
                ScreenToClient(draw.hwndItem, &mut p);
                let hot = PtInRect(&draw.rcItem, p) != 0;
                let primary = matches!(b.id, 240 | 501);
                let base =
                    if b.id < 100 || matches!(content, Content::Main(_) | Content::Picture(_)) {
                        host.theme.surface
                    } else {
                        host.theme.bg
                    };
                fill(draw.hDC, &draw.rcItem, base);
                let mut face = draw.rcItem;
                if b.icon.is_some() {
                    InflateRect(&mut face, -host.px(2.), -host.px(3.));
                }
                if primary || hot || b.active || draw.itemState & ODS_SELECTED != 0 {
                    fill(
                        draw.hDC,
                        &face,
                        if primary {
                            host.theme.accent
                        } else {
                            host.theme.hover
                        },
                    );
                }
                if b.active {
                    let marker = rect(
                        face.left,
                        face.bottom - host.px(2.),
                        face.right - face.left,
                        host.px(2.),
                    );
                    fill(draw.hDC, &marker, host.theme.accent);
                }
                if let Some(icon) = b.icon {
                    if let Some(atlas) = if b.id < 100 { &host.small } else { &host.large } {
                        atlas.paint(draw.hDC, draw.rcItem, icon);
                    }
                } else {
                    let ink = if primary {
                        if host.dark && host.palette == "catppuccin" {
                            host.theme.bg
                        } else {
                            color(0xffffff)
                        }
                    } else {
                        host.theme.text
                    };
                    label(
                        draw.hDC,
                        &draw.rcItem,
                        &b.text,
                        ink,
                        host.font.0,
                        DT_SINGLELINE | DT_VCENTER | DT_CENTER | DT_END_ELLIPSIS,
                    );
                    if !primary {
                        let brush = Brush::new(host.theme.border);
                        FrameRect(draw.hDC, &draw.rcItem, brush.0);
                    }
                }
                if draw.itemState & (ODS_FOCUS | ODS_NOFOCUSRECT) == ODS_FOCUS {
                    let brush = Brush::new(host.theme.accent);
                    FrameRect(draw.hDC, &face, brush.0);
                }
                RestoreDC(draw.hDC, saved);
                return 1;
            }
        }
        WM_CTLCOLORSTATIC | WM_CTLCOLOREDIT | WM_CTLCOLORLISTBOX | WM_CTLCOLORBTN => {
            SetTextColor(w as HDC, host.theme.text);
            if msg != WM_CTLCOLORLISTBOX {
                SetBkColor(w as HDC, host.theme.bg);
                return host.bg.0 as isize;
            }
            SetBkColor(w as HDC, host.theme.surface);
            return host.surface.0 as isize;
        }
        WM_COMMAND | COMMAND => {
            let id = w as u16;
            let code = (w >> 16) as u16;
            let from = l as HWND;
            match id {
                2 => {
                    if matches!(content, Content::Main(_)) {
                        // Esc on the focused main window follows Tianruo's
                        // workflow: hide the window while keeping the current
                        // OCR result and any running job intact. Capture
                        // selection windows still handle Esc as cancel.
                        ShowWindow(hwnd, SW_MINIMIZE);
                    } else {
                        PostMessageW(hwnd, WM_CLOSE, 0, 0);
                    }
                    return 0;
                }
                CLOSE => {
                    drop(window);
                    PostMessageW(hwnd, WM_CLOSE, 0, 0);
                    return 0;
                }
                MIN => {
                    ShowWindow(hwnd, SW_MINIMIZE);
                    return 0;
                }
                MAX => {
                    ShowWindow(
                        hwnd,
                        if IsZoomed(hwnd) != 0 {
                            SW_RESTORE
                        } else {
                            SW_MAXIMIZE
                        },
                    );
                    return 0;
                }
                PIN => {
                    host.top = !host.top;
                    set_top(hwnd, host.top);
                    if let Some(b) = host.buttons.iter_mut().find(|b| b.id == PIN) {
                        b.icon = Some(if host.top { Icon::Pin } else { Icon::Unpin });
                        InvalidateRect(b.hwnd, null(), 0);
                    }
                    return 0;
                }
                _ => content.command(host, id, code, from),
            }
            return 0;
        }
        WM_NOTIFY => {
            if let Content::Settings(s) = content {
                if s.reference_notice(host, l) {
                    return 1;
                }
            }
        }
        WM_CLOSE => {
            let destroy = content.close(host);
            drop(window);
            if destroy {
                DestroyWindow(hwnd);
            }
            return 0;
        }
        EXIT => {
            if let Content::Main(s) = content {
                s.exiting = true;
            }
            drop(window);
            PostMessageW(hwnd, WM_CLOSE, 0, 0);
            return 0;
        }
        WAKE => {
            if let Content::Main(s) = content {
                s.pump(host);
            }
            return 0;
        }
        WM_TIMER => {
            match content {
                Content::Main(s) => s.timer(host, w),
                Content::Picture(s) if w == picture::ZOOM_TIMER => s.timer(host),
                Content::Settings(s) => s.timer(host),
                Content::ServiceHelp(s) if w == service_help::CLOSE_TIMER => s.timer(host),
                _ => {}
            }
            return 0;
        }
        service_help::DISMISS => {
            match content {
                Content::Settings(s) => s.dismiss_help(),
                Content::ServiceHelp(s) => s.schedule_close(host),
                _ => {}
            }
            return 0;
        }
        WM_DROPFILES => {
            let count = DragQueryFileW(w as _, 0xffffffff, null_mut(), 0);
            if count > 0 {
                let n = DragQueryFileW(w as _, 0, null_mut(), 0);
                let mut path = vec![0u16; n as usize + 1];
                DragQueryFileW(w as _, 0, path.as_mut_ptr(), path.len() as u32);
                if let Content::Main(s) = content {
                    s.load(
                        host,
                        std::path::PathBuf::from(String::from_utf16_lossy(&path[..n as usize])),
                    );
                }
            }
            DragFinish(w as _);
            return 0;
        }
        WM_CONTEXTMENU | WM_NCRBUTTONUP => {
            if let Content::Main(s) = content {
                s.popup(host, w as HWND);
            } else if let Content::Note(s) = content {
                s.popup(host);
            } else if let Content::Picture(s) = content {
                s.popup(host);
            }
            return 0;
        }
        WM_LBUTTONDOWN | WM_LBUTTONUP | WM_MOUSEMOVE | WM_CAPTURECHANGED | WM_CANCELMODE => {
            if let Content::Picture(s) = content {
                s.mouse(host, msg, l);
                return 0;
            }
            if let Content::Main(s) = content {
                if s.mouse(host, msg, l) {
                    return 0;
                }
            }
        }
        WM_MOUSEWHEEL => {
            if let Content::Picture(s) = content {
                s.wheel(host, w, l);
            } else if let Content::Settings(s) = content {
                s.wheel(host, w);
            }
            return 0;
        }
        PICTURE_ZOOM => {
            if let Content::Picture(s) = content {
                s.set_zoom(host, w as f32 / 100.);
            }
            return 0;
        }
        PICTURE_INFO => {
            if let Content::Picture(s) = content {
                s.show_info(host, w != 0);
            }
            return 0;
        }
        WM_HSCROLL | WM_VSCROLL => {
            if let Content::Picture(s) = content {
                s.scroll(host, msg, w);
            } else if let Content::Settings(s) = content {
                s.scroll(host, w);
            }
            return 0;
        }
        WM_KEYDOWN => {
            if w == VK_ESCAPE as usize {
                if matches!(content, Content::Main(_)) {
                    // Esc minimizes the selected main window. It must not
                    // cancel an OCR request or erase the current selection.
                    ShowWindow(hwnd, SW_MINIMIZE);
                } else {
                    PostMessageW(hwnd, WM_CLOSE, 0, 0);
                }
                return 0;
            }
            if let Content::Main(s) = content {
                s.table_key(host, w);
            } else if let Content::Picture(s) = content {
                if w == b'C' as usize && GetKeyState(VK_CONTROL as i32) < 0 {
                    s.command(host, COPY);
                }
            } else if let Content::Tables(s) = content {
                if w == b'C' as usize && GetKeyState(VK_CONTROL as i32) < 0 {
                    s.command(host, 261, 0);
                }
            }
        }
        APPLY_THEME => {
            let index = w;
            host.change_theme(
                if index >= 2 { "catppuccin" } else { "vscode" },
                index % 2 == 1,
            );
            content.theme(host);
            return 0;
        }
        _ => {}
    }
    drop(window);
    DefWindowProcW(hwnd, msg, w, l)
}
unsafe extern "system" fn control_proc(
    hwnd: HWND,
    msg: u32,
    w: WPARAM,
    l: LPARAM,
    _id: usize,
    parent: usize,
) -> LRESULT {
    if !GetPropW(hwnd, platform::wide(table_layout::COPY_TABLE_PROP).as_ptr()).is_null() {
        if msg == WM_COPY
            || (msg == WM_KEYDOWN && w == b'C' as usize && GetKeyState(VK_CONTROL as i32) < 0)
        {
            PostMessageW(
                parent as HWND,
                COMMAND,
                table_layout::COPY_SELECTION as usize,
                hwnd as isize,
            );
            return 0;
        }
        if msg == WM_CHAR && w == 3 {
            return 0;
        }
    }
    // A rich math control's default plain-text clipboard representation loses
    // fractions, root ranges and exponents. Only math preview controls opt in;
    // copy their stored source instead of flattening the rendered equation.
    if (msg == WM_COPY || (msg == WM_KEYDOWN && w == 0x43 && GetKeyState(VK_CONTROL as i32) < 0))
        && !GetPropW(hwnd, platform::wide(rendered::COPY_SOURCE_PROP).as_ptr()).is_null()
    {
        PostMessageW(parent as HWND, COMMAND, COPY as usize, hwnd as isize);
        return 0;
    }
    if msg == WM_CHAR
        && w == 3
        && !GetPropW(hwnd, platform::wide(rendered::COPY_SOURCE_PROP).as_ptr()).is_null()
    {
        return 0;
    }
    // Owner-drawn buttons erase only their own surface during WM_DRAWITEM.
    // Avoid the system button background flashing through on hover/focus.
    if msg == WM_ERASEBKGND
        && GetWindowLongPtrW(hwnd, GWL_STYLE) as u32 & BS_TYPEMASK as u32 == BS_OWNERDRAW as u32
    {
        return 1;
    }
    if msg == WM_CHAR
        && GetKeyState(VK_CONTROL as i32) < 0
        && matches!(w, 0x2b | 0x2d | 0x3d | 0x30)
    {
        return 0;
    }
    if msg == WM_KEYDOWN && GetKeyState(VK_CONTROL as i32) < 0 {
        let command = match w as u32 {
            0xBB | 0x6B => Some(ZOOM_IN),
            0xBD | 0x6D => Some(ZOOM_OUT),
            0x30 | 0x60 => Some(ZOOM_RESET),
            0x46 => Some(SEARCH),
            0x5A if GetKeyState(VK_SHIFT as i32) < 0 => Some(REDO),
            0x5A => Some(UNDO),
            0x59 => Some(REDO),
            _ => None,
        };
        if let Some(command) = command {
            PostMessageW(parent as HWND, COMMAND, command as usize, hwnd as isize);
            return 0;
        }
    }
    if msg == WM_KEYDOWN && w == VK_ESCAPE as usize {
        PostMessageW(parent as HWND, WM_KEYDOWN, w, l);
        return 0;
    }
    // Wheel messages carry the modifier state at the time of the event.
    // Reading the later keyboard state can turn queued scrolling into zoom.
    if msg == WM_MOUSEWHEEL && w & 8 != 0 {
        // MK_CONTROL
        PostMessageW(
            parent as HWND,
            COMMAND,
            if (w >> 16) as u16 as i16 > 0 {
                ZOOM_IN
            } else {
                ZOOM_OUT
            } as usize,
            hwnd as isize,
        );
        return 0;
    }
    if msg == WM_CONTEXTMENU {
        PostMessageW(parent as HWND, WM_CONTEXTMENU, hwnd as usize, l);
        return 0;
    }
    if msg == WM_MOUSEMOVE {
        let mut event = TRACKMOUSEEVENT {
            cbSize: size_of::<TRACKMOUSEEVENT>() as u32,
            dwFlags: TME_LEAVE
                | if service_help::is_button(_id) {
                    TME_HOVER
                } else {
                    0
                },
            hwndTrack: hwnd,
            dwHoverTime: 450,
        };
        TrackMouseEvent(&mut event);
        if GetPropW(hwnd, platform::wide("QingquHot").as_ptr()).is_null() {
            SetPropW(hwnd, platform::wide("QingquHot").as_ptr(), 1 as HANDLE);
            InvalidateRect(hwnd, null(), 0);
        }
    }
    if msg == WM_MOUSELEAVE {
        if service_help::is_button(_id)
            || !GetPropW(parent as HWND, platform::wide("QingquServiceHelp").as_ptr()).is_null()
        {
            PostMessageW(parent as HWND, service_help::DISMISS, 0, 0);
        }
        RemovePropW(hwnd, platform::wide("QingquHot").as_ptr());
        InvalidateRect(hwnd, null(), 0);
    }
    if msg == WM_MOUSEHOVER && service_help::is_button(_id) {
        PostMessageW(parent as HWND, COMMAND, _id, 0);
    }
    DefSubclassProc(hwnd, msg, w, l)
}

pub fn run() -> Result<(), String> {
    let _apartment = editor_view::Apartment::new()?;
    unsafe {
        scintilla::register()?;
        // Load once; Windows owns the library until all Rich Edit windows have closed.
        let rich = LoadLibraryW(platform::wide("Msftedit.dll").as_ptr());
        if rich.is_null() {
            return Err("Windows Rich Edit 组件不可用。".into());
        }
        let init = INITCOMMONCONTROLSEX {
            dwSize: size_of::<INITCOMMONCONTROLSEX>() as u32,
            dwICC: ICC_STANDARD_CLASSES | ICC_WIN95_CLASSES,
        };
        InitCommonControlsEx(&init);
        let class = platform::wide("QingquOCR.Native.Window");
        let wc = WNDCLASSEXW {
            cbSize: size_of::<WNDCLASSEXW>() as u32,
            style: CS_DBLCLKS,
            lpfnWndProc: Some(window_proc),
            hInstance: GetModuleHandleW(null()),
            hCursor: LoadCursorW(null_mut(), IDC_ARROW),
            hIcon: icons::application_icon(),
            hIconSm: icons::application_icon(),
            lpszClassName: class.as_ptr(),
            ..Default::default()
        };
        if RegisterClassExW(&wc) == 0 {
            return Err("无法注册应用窗口。".into());
        }
        let (mut settings, keys, warning) = config::load();
        if std::env::args().any(|s| s == "--dark") {
            settings.dark = true;
        }
        if std::env::args().any(|s| s == "--catppuccin") {
            settings.palette = "catppuccin".into();
        }
        let palette = settings.palette.clone();
        let dark = settings.dark;
        let font_size = settings.editor_font_size;
        let work = platform::primary_work_size();
        let size = (820f32.min(work.0 - 20.), 360f32.min(work.1 - 30.));
        let hwnd = create(
            "轻取 OCR",
            size,
            Content::Main(Box::new(Main::new(settings, keys, warning))),
            None,
            &palette,
            dark,
            font_size,
        )?;
        let mut msg = MSG::default();
        loop {
            let status = GetMessageW(&mut msg, null_mut(), 0, 0);
            if status <= 0 {
                break;
            }
            // Dialog navigation gives native fields and buttons Tab/Shift+Tab behavior.
            let root = GetAncestor(msg.hwnd, GA_ROOT);
            if (WM_MOUSEFIRST..=WM_MOUSELAST).contains(&msg.message) {
                let tooltip = GetPropW(root, platform::wide("QingquTooltip").as_ptr()) as HWND;
                if !tooltip.is_null() {
                    SendMessageW(tooltip, TTM_RELAYEVENT, 0, (&msg as *const MSG) as isize);
                }
            }
            if !root.is_null() && IsDialogMessageW(root, &msg) != 0 {
                continue;
            }
            TranslateMessage(&msg);
            DispatchMessageW(&msg);
        }
        if IsWindow(hwnd) != 0 {
            DestroyWindow(hwnd);
        }
        windows_sys::Win32::Foundation::FreeLibrary(rich);
        Ok(())
    }
}
