//! Windows integration runs on a message loop, with no idle polling.
mod clipboard_cache;
mod image_clipboard;
mod integration;
mod table_clipboard;
use crate::config::Binding;
use image::RgbaImage;
pub use integration::{create_shortcut, set_startup, startup_enabled, Shortcut};
use std::{
    ptr::{null, null_mut},
    sync::mpsc,
    thread,
};
use windows_sys::Win32::{
    Foundation::*,
    Graphics::Gdi::*,
    System::{
        LibraryLoader::GetModuleHandleW,
        Threading::{CreateMutexW, GetCurrentProcessId},
    },
    UI::{HiDpi::*, Input::KeyboardAndMouse::*, Shell::*, WindowsAndMessaging::*},
};

pub fn wide(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(Some(0)).collect()
}
pub fn enable_dpi_awareness() {
    unsafe {
        SetProcessDpiAwarenessContext(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2);
    }
}
pub fn primary_work_size() -> (f32, f32) {
    unsafe {
        let mut rect: RECT = std::mem::zeroed();
        if SystemParametersInfoW(SPI_GETWORKAREA, 0, (&mut rect as *mut RECT).cast(), 0) != 0 {
            let scale = (GetDpiForSystem() as f32 / 96.).max(1.);
            (
                (rect.right - rect.left) as f32 / scale,
                (rect.bottom - rect.top) as f32 / scale,
            )
        } else {
            (1920., 1080.)
        }
    }
}
pub fn square_owned_windows() {
    unsafe {
        unsafe extern "system" fn square(hwnd: HWND, _: LPARAM) -> i32 {
            let mut process = 0;
            GetWindowThreadProcessId(hwnd, &mut process);
            if process == GetCurrentProcessId() {
                let corner: u32 = 1;
                windows_sys::Win32::Graphics::Dwm::DwmSetWindowAttribute(
                    hwnd,
                    33,
                    (&corner as *const u32).cast(),
                    4,
                );
            }
            1
        }
        EnumWindows(Some(square), 0);
    }
}

pub struct Instance(HANDLE);
impl Instance {
    pub fn acquire() -> Option<Self> {
        unsafe {
            let h = CreateMutexW(null(), 0, wide("Local\\QingquOCR.Native.instance").as_ptr());
            if h.is_null() {
                return Some(Self(h));
            }
            if GetLastError() == ERROR_ALREADY_EXISTS {
                let hwnd = FindWindowW(wide("QingquOCR.Native.events").as_ptr(), null());
                if !hwnd.is_null() {
                    PostMessageW(hwnd, WM_APP + 2, 0, 0);
                }
                CloseHandle(h);
                None
            } else {
                Some(Self(h))
            }
        }
    }
}
impl Drop for Instance {
    fn drop(&mut self) {
        if !self.0.is_null() {
            unsafe {
                CloseHandle(self.0);
            }
        }
    }
}

#[derive(Debug)]
pub enum Event {
    Action(String),
    Status(String),
    Quit,
}
enum Command {
    Rebind(Vec<Binding>, mpsc::SyncSender<Result<(), String>>),
    Pause(bool, mpsc::SyncSender<Result<(), String>>),
    ToolbarVisible(bool),
}
pub struct Desktop {
    hwnd: usize,
    commands: mpsc::Sender<Command>,
    join: Option<thread::JoinHandle<()>>,
    pub has_tray: bool,
}
struct State {
    tx: mpsc::Sender<Event>,
    rx: mpsc::Receiver<Command>,
    ctx: Notify,
    bindings: Vec<Binding>,
    active: Vec<bool>,
    paused: bool,
    tray: NOTIFYICONDATAW,
    taskbar_created: u32,
    toolbar_visible: bool,
}
impl State {
    fn emit(&self, event: Event) {
        let _ = self.tx.send(event);
        self.ctx.request_repaint();
    }
}

#[derive(Clone, Copy, Default)]
pub struct Notify {
    pub hwnd: usize,
    pub message: u32,
}
impl Notify {
    pub fn request_repaint(&self) {
        if self.hwnd != 0 {
            unsafe {
                PostMessageW(self.hwnd as HWND, self.message, 0, 0);
            }
        }
    }
}

impl Desktop {
    pub fn start(
        ctx: Notify,
        bindings: Vec<Binding>,
        toolbar_visible: bool,
    ) -> Result<(Self, mpsc::Receiver<Event>), String> {
        let (tx, rx) = mpsc::channel();
        let (cmd_tx, cmd_rx) = mpsc::channel();
        let (ready_tx, ready_rx) = mpsc::channel::<Result<(usize, bool), String>>();
        let join = thread::spawn(move || unsafe {
            let class = wide("QingquOCR.Native.events");
            let module = GetModuleHandleW(null());
            let wc = WNDCLASSW {
                lpfnWndProc: Some(event_proc),
                hInstance: module,
                lpszClassName: class.as_ptr(),
                ..std::mem::zeroed()
            };
            RegisterClassW(&wc);
            let hwnd = CreateWindowExW(
                0,
                class.as_ptr(),
                class.as_ptr(),
                0,
                0,
                0,
                0,
                0,
                null_mut(),
                null_mut(),
                module,
                null(),
            );
            if hwnd.is_null() {
                let _ = ready_tx.send(Err("无法创建系统事件窗口。".into()));
                return;
            }
            let mut tray: NOTIFYICONDATAW = std::mem::zeroed();
            tray.cbSize = std::mem::size_of_val(&tray) as u32;
            tray.hWnd = hwnd;
            tray.uID = 1;
            tray.uFlags = NIF_ICON | NIF_MESSAGE | NIF_TIP;
            tray.uCallbackMessage = WM_APP + 3;
            // A Win32 integer resource identifier is encoded in the pointer value, not dereferenced.
            tray.hIcon = LoadIconW(module, std::ptr::without_provenance(1));
            let tip = wide("轻取 OCR · 当前剪贴板 / 截图识别");
            tray.szTip[..tip.len()].copy_from_slice(&tip);
            let tray_ok = Shell_NotifyIconW(NIM_ADD, &tray) != 0;
            let state = Box::new(State {
                tx,
                rx: cmd_rx,
                ctx,
                bindings: Vec::new(),
                active: Vec::new(),
                paused: false,
                tray,
                taskbar_created: RegisterWindowMessageW(wide("TaskbarCreated").as_ptr()),
                toolbar_visible,
            });
            let ptr = Box::into_raw(state);
            SetWindowLongPtrW(hwnd, GWLP_USERDATA, ptr as isize);
            if let Err(e) = rebind(hwnd, &mut *ptr, bindings, true) {
                (*ptr).emit(Event::Status(e));
            }
            if !tray_ok {
                (*ptr).emit(Event::Status(
                    "托盘图标创建失败；关闭窗口将直接退出。".into(),
                ));
            }
            let _ = ready_tx.send(Ok((hwnd as usize, tray_ok)));
            let mut msg: MSG = std::mem::zeroed();
            while GetMessageW(&mut msg, null_mut(), 0, 0) > 0 {
                TranslateMessage(&msg);
                DispatchMessageW(&msg);
            }
            let state = Box::from_raw(ptr);
            for i in 0..state.bindings.len() {
                UnregisterHotKey(hwnd, 100 + i as i32);
            }
            Shell_NotifyIconW(NIM_DELETE, &state.tray);
        });
        let (hwnd, has_tray) = ready_rx
            .recv()
            .map_err(|_| "系统事件线程无法启动。".to_string())??;
        Ok((
            Self {
                hwnd,
                commands: cmd_tx,
                join: Some(join),
                has_tray,
            },
            rx,
        ))
    }
    pub fn rebind(&self, bindings: Vec<Binding>) -> Result<(), String> {
        let (tx, rx) = mpsc::sync_channel(1);
        self.commands
            .send(Command::Rebind(bindings, tx))
            .map_err(|_| "快捷键线程已退出。")?;
        unsafe {
            PostMessageW(self.hwnd as HWND, WM_APP + 1, 0, 0);
        }
        rx.recv().map_err(|_| "快捷键更新没有完成。".to_string())?
    }
    pub fn pause_hotkeys(&self, paused: bool) -> Result<(), String> {
        let (tx, rx) = mpsc::sync_channel(1);
        self.commands
            .send(Command::Pause(paused, tx))
            .map_err(|_| "快捷键线程已退出。")?;
        unsafe {
            PostMessageW(self.hwnd as HWND, WM_APP + 1, 0, 0);
        }
        rx.recv()
            .map_err(|_| "快捷键状态更新没有完成。".to_string())?
    }
    pub fn toolbar_visible(&self, visible: bool) {
        if self.commands.send(Command::ToolbarVisible(visible)).is_ok() {
            unsafe {
                PostMessageW(self.hwnd as HWND, WM_APP + 1, 0, 0);
            }
        }
    }
}
impl Drop for Desktop {
    fn drop(&mut self) {
        unsafe {
            PostMessageW(self.hwnd as HWND, WM_CLOSE, 0, 0);
        }
        if let Some(join) = self.join.take() {
            let _ = join.join();
        }
    }
}

pub fn parse_hotkey(input: &str) -> Result<(u32, u32), String> {
    let mut modifiers = MOD_NOREPEAT;
    let mut key = None;
    for part in input.split('+').map(str::trim).filter(|s| !s.is_empty()) {
        let token = part.to_ascii_uppercase();
        let m = match token.as_str() {
            "CTRL" | "CONTROL" => MOD_CONTROL,
            "SHIFT" => MOD_SHIFT,
            "ALT" => MOD_ALT,
            "WIN" | "SUPER" => MOD_WIN,
            _ => 0,
        };
        if m != 0 {
            if modifiers & m != 0 {
                return Err("快捷键修饰键重复。".into());
            }
            modifiers |= m;
            continue;
        }
        if key.is_some() {
            return Err("快捷键只能包含一个主键。".into());
        }
        key = Some(
            if token.len() == 1 && token.as_bytes()[0].is_ascii_alphanumeric() {
                token.as_bytes()[0] as u32
            } else if let Some(f) = token
                .strip_prefix('F')
                .and_then(|s| s.parse::<u32>().ok())
                .filter(|v| (1..=24).contains(v))
            {
                VK_F1 as u32 + f - 1
            } else {
                match token.as_str() {
                    "SPACE" => VK_SPACE as u32,
                    "INSERT" => VK_INSERT as u32,
                    "PRINTSCREEN" => VK_SNAPSHOT as u32,
                    _ => return Err(format!("不支持的快捷键：{part}。")),
                }
            },
        );
    }
    let key = key.ok_or("快捷键缺少字母、数字或功能键。")?;
    if modifiers == MOD_NOREPEAT && !(VK_F1 as u32..=VK_F24 as u32).contains(&key) {
        return Err("全局字母快捷键至少需要 Ctrl、Alt、Shift 或 Win。".into());
    }
    Ok((modifiers, key))
}
pub fn validate_bindings(bindings: &[Binding]) -> Result<(), String> {
    let mut used = std::collections::HashSet::new();
    for b in bindings {
        if !b.key.trim().is_empty() {
            let key = parse_hotkey(&b.key)?;
            if !used.insert(key) {
                return Err(format!("快捷键 {} 重复。", b.key));
            }
        }
    }
    Ok(())
}
unsafe fn rebind(
    hwnd: HWND,
    state: &mut State,
    bindings: Vec<Binding>,
    initial: bool,
) -> Result<(), String> {
    validate_bindings(&bindings)?;
    let old = state.bindings.clone();
    let old_active = state.active.clone();
    state.paused = false;
    for i in 0..old.len() {
        UnregisterHotKey(hwnd, 100 + i as i32);
    }
    let mut failed = None;
    let mut active = vec![false; bindings.len()];
    for (i, b) in bindings.iter().enumerate() {
        if !b.key.trim().is_empty() {
            let (m, k) = parse_hotkey(&b.key).unwrap();
            if RegisterHotKey(hwnd, 100 + i as i32, m, k) == 0 {
                failed = Some(b.key.clone());
                break;
            }
            active[i] = true;
        }
    }
    if let Some(key) = failed {
        for i in 0..bindings.len() {
            UnregisterHotKey(hwnd, 100 + i as i32);
        }
        for (i, b) in old.iter().enumerate() {
            if old_active.get(i) == Some(&true) {
                if let Ok((m, k)) = parse_hotkey(&b.key) {
                    RegisterHotKey(hwnd, 100 + i as i32, m, k);
                }
            }
        }
        // On first launch keep other, available shortcuts working.
        if initial {
            active.fill(false);
            for (i, b) in bindings.iter().enumerate() {
                if let Ok((m, k)) = parse_hotkey(&b.key) {
                    active[i] = RegisterHotKey(hwnd, 100 + i as i32, m, k) != 0;
                }
            }
            state.bindings = bindings;
            state.active = active;
        }
        Err(format!("快捷键 {key} 被占用；请退出天若或在设置中换键。"))
    } else {
        state.bindings = bindings;
        state.active = active;
        Ok(())
    }
}
unsafe extern "system" fn event_proc(hwnd: HWND, msg: u32, w: WPARAM, l: LPARAM) -> LRESULT {
    let ptr = GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *mut State;
    if !ptr.is_null() {
        if msg == WM_APP + 3 && l as u32 == WM_RBUTTONUP {
            let tx = (*ptr).tx.clone();
            let notify = (*ptr).ctx;
            let menu = CreatePopupMenu();
            for (id, label) in [
                (1, "显示主窗口"),
                (2, "当前剪贴板"),
                (6, "Alt+V 全部剪贴板复位"),
                (3, "区域截图"),
                (0, ""),
                (10, "显示工具悬浮窗(&F)"),
                (11, "关闭工具悬浮窗(&H)"),
                (0, ""),
                (7, "开机自启(&S)"),
                (8, "发送到桌面快捷方式(&D)"),
                (9, "发送到开始菜单(&W)"),
                (0, ""),
                (5, "设置"),
                (4, "退出"),
            ] {
                let flags = if id == 0 {
                    MF_SEPARATOR
                } else if (id == 10 && (*ptr).toolbar_visible)
                    || (id == 11 && !(*ptr).toolbar_visible)
                {
                    MF_STRING | MF_GRAYED
                } else if id == 7 && startup_enabled() {
                    MF_STRING | MF_CHECKED
                } else {
                    MF_STRING
                };
                AppendMenuW(menu, flags, id, wide(label).as_ptr());
            }
            let mut point = POINT::default();
            GetCursorPos(&mut point);
            SetForegroundWindow(hwnd);
            let id = TrackPopupMenu(
                menu,
                TPM_RETURNCMD | TPM_RIGHTBUTTON,
                point.x,
                point.y,
                0,
                hwnd,
                null(),
            );
            DestroyMenu(menu);
            let event = match id {
                1 => Some(Event::Action("show".into())),
                2 => Some(Event::Action("clipboard".into())),
                3 => Some(Event::Action("capture".into())),
                4 => Some(Event::Quit),
                5 => Some(Event::Action("settings".into())),
                6 => Some(Event::Action("reset-pins".into())),
                7 => Some(Event::Action("toggle-startup".into())),
                8 => Some(Event::Action("desktop-shortcut".into())),
                9 => Some(Event::Action("start-menu-shortcut".into())),
                10 => Some(Event::Action("show-toolbar".into())),
                11 => Some(Event::Action("hide-toolbar".into())),
                _ => None,
            };
            if let Some(event) = event {
                let _ = tx.send(event);
                notify.request_repaint();
            }
            return 0;
        }
        let state = &mut *ptr;
        if msg == state.taskbar_created && msg != 0 {
            Shell_NotifyIconW(NIM_ADD, &state.tray);
            return 0;
        }
        match msg {
            WM_HOTKEY => {
                if state.paused {
                    return 0;
                }
                if let Some(b) = state.bindings.get(w.saturating_sub(100)) {
                    state.emit(Event::Action(b.action.clone()));
                }
                return 0;
            }
            x if x == WM_APP + 1 => {
                while let Ok(command) = state.rx.try_recv() {
                    match command {
                        Command::ToolbarVisible(visible) => state.toolbar_visible = visible,
                        Command::Rebind(bindings, reply) => {
                            let result = rebind(hwnd, state, bindings, false);
                            let _ = reply.send(result);
                        }
                        Command::Pause(paused, reply) => {
                            let mut failures = Vec::new();
                            if paused != state.paused {
                                for (i, b) in state.bindings.iter().enumerate() {
                                    if state.active.get(i) == Some(&true) {
                                        if paused {
                                            UnregisterHotKey(hwnd, 100 + i as i32);
                                        } else if let Ok((m, k)) = parse_hotkey(&b.key) {
                                            if RegisterHotKey(hwnd, 100 + i as i32, m, k) == 0 {
                                                failures.push(b.key.clone());
                                            }
                                        }
                                    }
                                }
                                state.paused = paused;
                            }
                            let _ = reply.send(if failures.is_empty() {
                                Ok(())
                            } else {
                                Err(format!(
                                    "快捷键 {} 在录制期间被其他程序占用，请重新配置。",
                                    failures.join("、")
                                ))
                            });
                        }
                    }
                }
                return 0;
            }
            x if x == WM_APP + 2 => {
                state.emit(Event::Action("show".into()));
                return 0;
            }
            x if x == WM_APP + 3 => {
                match l as u32 {
                    WM_LBUTTONUP | WM_LBUTTONDBLCLK => state.emit(Event::Action("show".into())),
                    _ => {}
                }
                return 0;
            }
            WM_CLOSE => {
                DestroyWindow(hwnd);
                return 0;
            }
            WM_DESTROY => {
                PostQuitMessage(0);
                return 0;
            }
            _ => {}
        }
    }
    DefWindowProcW(hwnd, msg, w, l)
}

pub enum ClipboardContent {
    Text(String),
    Image(RgbaImage, Option<usize>),
}
pub fn read_clipboard() -> Result<ClipboardContent, String> {
    let sequence =
        unsafe { windows_sys::Win32::System::DataExchange::GetClipboardSequenceNumber() };
    let mut clipboard = arboard::Clipboard::new().map_err(|_| "无法打开剪贴板，请稍后重试。")?;
    // Native image wins over the textual HTML/filename fallback created by many image applications.
    if let Ok(image) = clipboard.get_image() {
        let w = u32::try_from(image.width).map_err(|_| "剪贴板图片过大。")?;
        let h = u32::try_from(image.height).map_err(|_| "剪贴板图片过大。")?;
        if u64::from(w) * u64::from(h) > 64_000_000 {
            return Err("剪贴板图片超过 6400 万像素，请先缩小。".into());
        }
        let png_bytes = image_clipboard::png_size(sequence);
        return RgbaImage::from_raw(w, h, image.bytes.into_owned())
            .map(|image| ClipboardContent::Image(image, png_bytes))
            .ok_or("剪贴板图片格式不完整。".into());
    }
    if let Ok(text) = clipboard.get_text() {
        if !text.is_empty() {
            return Ok(ClipboardContent::Text(text));
        }
    }
    Err("剪贴板为空、正被占用或内容不是文字／图片。".into())
}
pub fn copy_text(text: &str) -> Result<(), String> {
    arboard::Clipboard::new()
        .and_then(|mut c| c.set_text(text))
        .map_err(|_| "无法写入剪贴板，请稍后重试。".into())
}
pub fn copy_image(owner: HWND, image: &RgbaImage) -> Result<(), String> {
    copy_image_pixels(owner, image.width(), image.height(), image.as_raw())
}
pub fn copy_image_pixels(
    owner: HWND,
    width: u32,
    height: u32,
    pixels: &[u8],
) -> Result<(), String> {
    image_clipboard::copy(owner, width, height, pixels)
}
pub fn copy_table(xml: &[u8], tsv: &str, rtf: &[u8]) -> Result<(), String> {
    table_clipboard::copy(xml, tsv, rtf)
}
pub fn hide_owned_windows() -> Vec<usize> {
    unsafe {
        unsafe extern "system" fn collect(hwnd: HWND, param: LPARAM) -> i32 {
            let mut process = 0;
            GetWindowThreadProcessId(hwnd, &mut process);
            if process == GetCurrentProcessId() && IsWindowVisible(hwnd) != 0 && IsIconic(hwnd) == 0
            {
                (*(param as *mut Vec<usize>)).push(hwnd as usize);
            }
            1
        }
        let mut windows = Vec::new();
        EnumWindows(Some(collect), (&mut windows as *mut Vec<usize>) as isize);
        for &hwnd in &windows {
            ShowWindow(hwnd as HWND, SW_HIDE);
        }
        windows
    }
}
pub fn restore_owned_windows(windows: &[usize]) {
    unsafe {
        for &hwnd in windows {
            if IsWindow(hwnd as HWND) != 0 {
                ShowWindow(hwnd as HWND, SW_SHOWNOACTIVATE);
            }
        }
    }
}
pub fn open_image(owner: usize) -> Option<std::path::PathBuf> {
    unsafe {
        use windows_sys::Win32::UI::Controls::Dialogs::*;
        let mut path = vec![0u16; 32768];
        let filter = wide("图片 (PNG/JPG/BMP)\0*.png;*.jpg;*.jpeg;*.bmp\0所有文件\0*.*\0");
        let mut ofn: OPENFILENAMEW = std::mem::zeroed();
        ofn.lStructSize = std::mem::size_of_val(&ofn) as u32;
        ofn.hwndOwner = owner as HWND;
        ofn.lpstrFilter = filter.as_ptr();
        ofn.lpstrFile = path.as_mut_ptr();
        ofn.nMaxFile = path.len() as u32;
        ofn.Flags = OFN_FILEMUSTEXIST | OFN_PATHMUSTEXIST | OFN_NOCHANGEDIR;
        if GetOpenFileNameW(&mut ofn) != 0 {
            let end = path.iter().position(|c| *c == 0).unwrap_or(path.len());
            Some(std::path::PathBuf::from(String::from_utf16_lossy(
                &path[..end],
            )))
        } else {
            None
        }
    }
}

pub fn capture_desktop() -> Result<(RgbaImage, i32, i32), String> {
    unsafe {
        let mut cursor = POINT::default();
        if GetCursorPos(&mut cursor) == 0 {
            return Err("无法确定鼠标所在显示器。".into());
        }
        let monitor = MonitorFromPoint(cursor, MONITOR_DEFAULTTONEAREST);
        let mut info = MONITORINFO {
            cbSize: std::mem::size_of::<MONITORINFO>() as u32,
            ..Default::default()
        };
        if GetMonitorInfoW(monitor, &mut info) == 0 {
            return Err("无法读取鼠标所在显示器范围。".into());
        }
        let x = info.rcMonitor.left;
        let y = info.rcMonitor.top;
        capture_rect(x, y, info.rcMonitor.right - x, info.rcMonitor.bottom - y)
            .map(|image| (image, x, y))
    }
}
fn capture_pixels(w: i32, h: i32) -> Result<Vec<u8>, String> {
    if w <= 0 || h <= 0 {
        return Err("截图区域无效。".into());
    }
    let bytes = (w as usize)
        .checked_mul(h as usize)
        .and_then(|pixels| pixels.checked_mul(4))
        .filter(|&bytes| bytes <= isize::MAX as usize)
        .ok_or("截图尺寸超出系统支持的内存范围。")?;
    // Keep the monitor's physical resolution. Allocation failure, rather than
    // an arbitrary pixel budget, decides whether the capture can proceed.
    let mut pixels = Vec::new();
    pixels.try_reserve_exact(bytes).map_err(|_| {
        format!(
            "可用内存不足，无法创建 {:.1} MiB 的截图缓冲。",
            bytes as f64 / (1024. * 1024.)
        )
    })?;
    pixels.resize(bytes, 0u8);
    Ok(pixels)
}

unsafe fn capture_rect(x: i32, y: i32, w: i32, h: i32) -> Result<RgbaImage, String> {
    let pixels = capture_pixels(w, h)?;
    let screen = GetDC(null_mut());
    let dc = CreateCompatibleDC(screen);
    let bitmap = CreateCompatibleBitmap(screen, w, h);
    if screen.is_null() || dc.is_null() || bitmap.is_null() {
        if !bitmap.is_null() {
            DeleteObject(bitmap);
        }
        if !dc.is_null() {
            DeleteDC(dc);
        }
        if !screen.is_null() {
            ReleaseDC(null_mut(), screen);
        }
        return Err("无法创建截图缓冲区。".into());
    }
    let old = SelectObject(dc, bitmap);
    if old.is_null() || old == -1isize as HGDIOBJ {
        DeleteObject(bitmap);
        DeleteDC(dc);
        ReleaseDC(null_mut(), screen);
        return Err("无法初始化截图位图。".into());
    }
    let ok = BitBlt(dc, 0, 0, w, h, screen, x, y, SRCCOPY | CAPTUREBLT);
    SelectObject(dc, old);
    let mut info: BITMAPINFO = std::mem::zeroed();
    info.bmiHeader = BITMAPINFOHEADER {
        biSize: std::mem::size_of::<BITMAPINFOHEADER>() as u32,
        biWidth: w,
        biHeight: -h,
        biPlanes: 1,
        biBitCount: 32,
        biCompression: BI_RGB,
        ..std::mem::zeroed()
    };
    let mut bytes = pixels;
    let lines = if ok != 0 {
        GetDIBits(
            screen,
            bitmap,
            0,
            h as u32,
            bytes.as_mut_ptr().cast(),
            &mut info,
            DIB_RGB_COLORS,
        )
    } else {
        0
    };
    DeleteObject(bitmap);
    DeleteDC(dc);
    ReleaseDC(null_mut(), screen);
    if lines != h {
        return Err("屏幕截图失败；受保护的窗口可能无法截图。".into());
    }
    for p in bytes.chunks_exact_mut(4) {
        p.swap(0, 2);
        p[3] = 255;
    }
    RgbaImage::from_raw(w as u32, h as u32, bytes).ok_or("截图数据不完整。".into())
}

mod selection;
pub use selection::{select_region, select_window};
