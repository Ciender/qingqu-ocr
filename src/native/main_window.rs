use super::*;
use crate::{
    config::{Credentials, Settings},
    ocr::Table,
    text,
};
use backend::{Job, Output, Work, Worker};
use image::RgbaImage;
use std::{
    path::PathBuf,
    sync::{
        atomic::{AtomicBool, Ordering},
        mpsc, Arc,
    },
    time::Instant,
};
mod commands;
mod document_pane;
mod jobs;
mod recognition;
mod table_pane;
mod translation_controls;
use table_pane::{TablePane, TABLE_INDEX};

const SOURCE: u16 = 200;
const TRANSLATION: u16 = 201;
const CALL_SETTINGS: u16 = 206;
const LANGUAGE: u16 = 203;
const TARGET_LANGUAGE: u16 = 205;
const R_COPY: u16 = 310;
const R_CLEAR: u16 = 311;
const R_UNDO: u16 = 312;
const R_REDO: u16 = 313;
const R_SEARCH: u16 = 314;
const R_PASTE: u16 = 315;
const R_CUT: u16 = 316;
const R_MENU: u16 = 317;
const DOCUMENT_VIEW: u16 = 318;
const SOURCE_TOOLS: &[u16] = &[
    CAPTURE,
    TABLE_CAPTURE,
    VERTICAL_CAPTURE,
    FORMULA_CAPTURE,
    SEARCH,
    PARAGRAPH,
    FORMAT,
    PUNCTUATION,
    CHINESE,
    DOCUMENT_VIEW,
];
pub struct Pending {
    pub id: u64,
    pub cancel: Arc<AtomicBool>,
    pub started: Instant,
    pub label: String,
    pub purpose: Purpose,
    pub hidden: Vec<usize>,
}
pub enum Purpose {
    Ocr {
        formula: bool,
        vertical: bool,
    },
    Translate,
    Capture(String),
    Clipboard,
    Convert {
        target: usize,
        range: Range,
        before: String,
    },
}
pub struct Main {
    pub settings: Settings,
    pub keys: Credentials,
    pub source: HWND,
    pub translated: HWND,
    pub worker: Option<Worker>,
    desktop: Option<platform::Desktop>,
    events: Option<mpsc::Receiver<platform::Event>>,
    pub pending: Option<Pending>,
    pub(super) next_id: u64,
    pub children: Vec<usize>,
    settings_hwnd: usize,
    toolbar: usize,
    pub(super) document_view: Option<rendered::DocumentView>,
    rendered_active: bool,
    rendered_markdown: bool,
    rendered_body: String,
    rendered_origin: HWND,
    pub exiting: bool,
    pub status: String,
    original: String,
    processing_base: String,
    processing_result: String,
    layout: Option<String>,
    pub(super) tables: Vec<Table>,
    table_view: TablePane,
    pub(super) table_active: bool,
    focused: HWND,
    source_fraction: f32,
    dragging: bool,
    translation_width: i32,
    preview: bool,
}
impl Main {
    pub fn new(mut settings: Settings, keys: Credentials, warning: Option<String>) -> Self {
        if !settings.editor_font_size.is_finite() {
            settings.editor_font_size = 14.;
        }
        settings.editor_font_size = settings.editor_font_size.clamp(8., 48.);
        settings.startup = platform::startup_enabled();
        settings.normalize_ocr_modes();
        settings.normalize_ocr_language();
        settings.normalize_processing_modes();
        if settings.translation_ui_version == 0 {
            settings.auto_translate = settings.show_translation;
            settings.translation_ui_version = 1;
        }
        // Older preferences used an empty layout as the implicit default.
        // Treat that legacy value as Tianruo-style automatic paragraph
        // splitting when the main window starts, so a fresh OCR result is
        // readable without requiring a second toolbar click.
        if settings.processing.layout.is_empty() {
            settings.processing.layout = "smart".into();
        }
        if settings.processing.punctuation.is_empty() {
            settings.processing.punctuation = "chinese".into();
        }
        let preview = false;
        Self {
            settings,
            keys,
            source: null_mut(),
            translated: null_mut(),
            worker: None,
            desktop: None,
            events: None,
            pending: None,
            next_id: 1,
            children: Vec::new(),
            settings_hwnd: 0,
            toolbar: 0,
            document_view: None,
            rendered_active: false,
            rendered_markdown: false,
            rendered_body: String::new(),
            rendered_origin: null_mut(),
            exiting: false,
            status: warning.unwrap_or_else(|| "就绪 · Alt+V 读取当前剪贴板".into()),
            original: String::new(),
            processing_base: String::new(),
            processing_result: String::new(),
            layout: None,
            tables: Vec::new(),
            table_view: TablePane::new(),
            table_active: false,
            focused: null_mut(),
            source_fraction: 0.5,
            dragging: false,
            translation_width: 410,
            preview,
        }
    }
    pub fn init(&mut self, h: &mut Host) -> Result<(), String> {
        unsafe {
            h.title_buttons(true, false);
            self.source = h.rich(SOURCE, false);
            self.translated = h.rich(TRANSLATION, false);
            scintilla::line_numbers(self.source, true);
            history::install(self.source)?;
            history::install(self.translated)?;
            self.focused = self.source;
            if self.source.is_null() || self.translated.is_null() {
                return Err("无法创建原生文本编辑器。".into());
            }
            for (id, icon, tip) in [
                (CAPTURE, Icon::Text, "区域截图识别"),
                (TABLE_CAPTURE, Icon::Table, "表格识别 · 框选区域"),
                (VERTICAL_CAPTURE, Icon::Vertical, "竖排文字识别 · 框选区域"),
                (
                    FORMULA_CAPTURE,
                    Icon::Formula,
                    "文字＋公式识别 · 混排结果在主窗口右栏预览",
                ),
                (SEARCH, Icon::Search, "查找 / 替换原文 · Ctrl+F"),
                (
                    PARAGRAPH,
                    Icon::AutoParagraph,
                    "自动分段 · 点击选择分段模式",
                ),
                (FORMAT, Icon::Layout, "智能排版 · 仅处理当前文本"),
                (
                    PUNCTUATION,
                    Icon::ChinesePunctuation,
                    "中文标点 · 点击切换为英文标点",
                ),
                (CHINESE, Icon::Chinese, "简体 / 繁体转换"),
                (DOCUMENT_VIEW, Icon::Eye, "右栏显示公式／Markdown预览或译文"),
                (TRANSLATE, Icon::Translate, "翻译原文或选区"),
                (R_COPY, Icon::Copy, "复制译文"),
                (R_CLEAR, Icon::Clear, "清空译文"),
                (R_SEARCH, Icon::Search, "查找 / 替换译文"),
                (TOGGLE, Icon::Close, "收起翻译栏"),
            ] {
                h.button(id, Some(icon), tip);
            }
            self.sync_modes(h);
            h.control(
                "BUTTON",
                "自动翻译",
                WS_TABSTOP | BS_AUTOCHECKBOX as u32,
                AUTO_TRANSLATE,
            );
            h.control(
                "COMBOBOX",
                "",
                WS_TABSTOP | CBS_DROPDOWNLIST as u32 | WS_VSCROLL,
                TABLE_INDEX,
            );
            h.button(CALL_SETTINGS, None, "调用配置");
            h.control(
                "COMBOBOX",
                "",
                WS_TABSTOP | CBS_DROPDOWNLIST as u32 | WS_VSCROLL,
                LANGUAGE,
            );
            self.sync_ocr_language_selector(h);
            h.control(
                "COMBOBOX",
                "",
                WS_TABSTOP | CBS_DROPDOWNLIST as u32 | WS_VSCROLL,
                TARGET_LANGUAGE,
            );
            self.sync_translation_target(h);
            self.worker = Some(Worker::new(platform::Notify {
                hwnd: h.hwnd as usize,
                message: WAKE,
            }));
            h.proxy = Some(self.worker.as_ref().unwrap().proxy.clone());
            self.sync_picture_menu(h);
            let preview = std::env::args().any(|s| s == "--preview-document");
            let mut bindings = self.settings.hotkeys.clone();
            if preview {
                for b in &mut bindings {
                    b.key.clear();
                }
            }
            match platform::Desktop::start(
                platform::Notify {
                    hwnd: h.hwnd as usize,
                    message: WAKE,
                },
                bindings,
                self.settings.floating_button,
            ) {
                Ok((desktop, rx)) => {
                    if !desktop.has_tray {
                        self.settings.close_to_tray = false;
                    }
                    self.desktop = Some(desktop);
                    self.events = Some(rx);
                }
                Err(e) => {
                    self.status = e;
                    self.settings.close_to_tray = false;
                }
            }
            DragAcceptFiles(h.hwnd, 1);
            if self.settings.floating_button {
                if let Err(error) = self.sync_toolbar(h) {
                    self.set_status(h, error);
                }
            }
            h.min_size.0 = if self.settings.show_translation {
                600.
            } else {
                300.
            };
            if !self.settings.show_translation {
                let mut r = RECT::default();
                GetWindowRect(h.hwnd, &mut r);
                SetWindowPos(
                    h.hwnd,
                    null_mut(),
                    r.left,
                    r.top,
                    h.px(410.),
                    h.px(360.),
                    SWP_NOZORDER | SWP_NOACTIVATE,
                );
            }
        }
        self.load_document_argument(h)?;
        Ok(())
    }
    pub fn layout(&mut self, h: &mut Host) {
        unsafe {
            h.button_tip(
                TOGGLE,
                Icon::Close,
                if self.rendered_active {
                    "收起文档预览"
                } else {
                    "收起翻译栏"
                },
            );
            h.button_tip(
                TRANSLATE,
                Icon::Translate,
                if self.settings.show_translation {
                    "翻译原文或选区"
                } else {
                    "打开翻译栏并翻译原文或选区"
                },
            );
            let (w, height) = dimensions(h.hwnd);
            let padding = h.px(7.);
            let title = h.px(22.);
            let tools = h.px(38.);
            let footer = h.px(26.);
            let split = if self.settings.show_translation {
                (w as f32 * self.source_fraction).round() as i32
            } else {
                w
            };
            let content_y = title + tools;
            let content_height = (height - content_y - footer).max(1);
            move_to(
                self.source,
                rect(
                    padding,
                    content_y,
                    (split - 2 * padding).max(1),
                    content_height,
                ),
            );
            move_to(
                self.translated,
                rect(
                    split + padding,
                    content_y,
                    (w - split - 2 * padding).max(1),
                    content_height,
                ),
            );
            self.layout_document(
                h,
                rect(
                    split + padding,
                    content_y,
                    (w - split - 2 * padding).max(1),
                    content_height,
                ),
            );
            ShowWindow(
                self.translated,
                if self.settings.show_translation && !self.rendered_active && !self.table_active {
                    SW_SHOW
                } else {
                    SW_HIDE
                },
            );
            let step = h.px(34.);
            let source_end = split
                - padding
                - if self.settings.show_translation {
                    0
                } else {
                    step
                };
            let mut x = padding;
            for &id in SOURCE_TOOLS {
                let visible = x + step <= source_end;
                let b = GetDlgItem(h.hwnd, id as i32);
                ShowWindow(b, if visible { SW_SHOW } else { SW_HIDE });
                if visible {
                    move_to(b, rect(x, title, step, tools));
                    x += step;
                    if matches!(id, FORMULA_CAPTURE | SEARCH | FORMAT | CHINESE) {
                        x += h.px(5.);
                    }
                }
            }
            let right = [TOGGLE, TRANSLATE, R_COPY, R_CLEAR, R_SEARCH];
            let mut right_x = split + padding;
            for id in right {
                let b = GetDlgItem(h.hwnd, id as i32);
                let visible = if !self.settings.show_translation {
                    id == TRANSLATE
                } else if id == TOGGLE {
                    true
                } else {
                    self.settings.show_translation
                        && (!(self.rendered_active || self.table_active)
                            || !matches!(id, R_CLEAR | R_SEARCH))
                        && (id == R_MENU || right_x + step <= w - padding - 2 * step)
                };
                ShowWindow(b, if visible { SW_SHOW } else { SW_HIDE });
                let x = if id == TRANSLATE && !self.settings.show_translation {
                    source_end
                } else if id == R_MENU {
                    w - padding - 2 * step
                } else {
                    right_x
                };
                move_to(b, rect(x, title, step, tools));
                if visible && id != R_MENU {
                    right_x += step;
                }
            }
            let auto = GetDlgItem(h.hwnd, AUTO_TRANSLATE as i32);
            SendMessageW(
                auto,
                BM_SETCHECK,
                usize::from(self.settings.auto_translate),
                0,
            );
            self.layout_translation_controls(h, split, w, height, padding, footer);
            let language_width = ((split - 2 * padding) * 35 / 100).clamp(h.px(65.), h.px(110.));
            move_to(
                GetDlgItem(h.hwnd, CALL_SETTINGS as i32),
                rect(
                    split - padding - language_width - h.px(79.),
                    height - footer,
                    h.px(74.),
                    h.px(24.),
                ),
            );
            move_to(
                GetDlgItem(h.hwnd, LANGUAGE as i32),
                rect(
                    split - padding - language_width,
                    height - footer,
                    language_width,
                    h.px(230.),
                ),
            );
            h.button_tip(
                R_COPY,
                Icon::Copy,
                if self.table_active {
                    "复制整表（保留行列与合并单元格）"
                } else if self.rendered_active {
                    "复制文档源码（保留公式结构）"
                } else {
                    "复制译文"
                },
            );
            if let Some(button) = h.buttons.iter_mut().find(|b| b.id == DOCUMENT_VIEW) {
                button.active = self.rendered_active || self.table_active;
            }
            ShowWindow(GetDlgItem(h.hwnd, TABLE_INDEX as i32), SW_HIDE);
            ShowWindow(
                GetDlgItem(h.hwnd, table_layout::TABLE_EDITOR as i32),
                SW_HIDE,
            );
            if self.table_active && self.settings.show_translation {
                self.table_view.layout(
                    h,
                    rect(
                        split + padding,
                        content_y,
                        (w - split - 2 * padding).max(1),
                        content_height,
                    ),
                    &self.tables,
                );
            }
        }
    }
    pub(super) unsafe fn invalidate_editor_view(&self, h: &Host, _editor: HWND, changed: bool) {
        let (w, height) = dimensions(h.hwnd);
        if changed {
            InvalidateRect(h.hwnd, &rect(0, height - h.px(26.), w, h.px(26.)), 0);
        }
    }
    pub unsafe fn paint(&self, h: &Host, dc: HDC) {
        let (w, height) = dimensions(h.hwnd);
        let split = if self.settings.show_translation {
            (w as f32 * self.source_fraction).round() as i32
        } else {
            w
        };
        fill(dc, &rect(0, h.px(22.), w, h.px(38.)), h.theme.surface);
        fill(
            dc,
            &rect(0, height - h.px(26.), w, h.px(26.)),
            h.theme.surface,
        );
        if self.settings.show_translation {
            fill(
                dc,
                &rect(split - 1, h.px(22.), 1, height - h.px(22.)),
                h.theme.border,
            );
        }
        if RectVisible(dc, &rect(0, height - h.px(26.), w, h.px(26.))) != 0 {
            let original_len = SendMessageW(self.source, WM_GETTEXTLENGTH, 0, 0);
            label(
                dc,
                &rect(
                    h.px(7.),
                    height - h.px(24.),
                    (split - h.px(260.)).max(h.px(70.)),
                    h.px(22.),
                ),
                &if self.rendered_active {
                    format!(
                        "{original_len} 字 · {} 行 · 源码",
                        editor_view::line_count(self.source)
                    )
                } else {
                    format!(
                        "{original_len} 字 · {} 行",
                        editor_view::line_count(self.source)
                    )
                },
                h.theme.muted,
                h.font.0,
                DT_SINGLELINE | DT_VCENTER,
            );
            if self.settings.show_translation && !self.table_active {
                let n = SendMessageW(self.translated, WM_GETTEXTLENGTH, 0, 0);
                label(
                    dc,
                    &rect(
                        split + h.px(7.),
                        height - h.px(24.),
                        (w - split - h.px(160.)).max(0),
                        h.px(22.),
                    ),
                    &if self.rendered_active {
                        "文档预览 · 复制保留源码".into()
                    } else {
                        format!("{n} 字 · {} 号", h.font_size as i32)
                    },
                    h.theme.muted,
                    h.font.0,
                    DT_SINGLELINE | DT_VCENTER,
                );
            }
        }
    }
    pub fn theme(&mut self, h: &mut Host) {
        h.change_theme(&self.settings.palette, self.settings.dark);
        self.refresh_document(h);
    }
    pub fn set_status(&mut self, h: &mut Host, s: impl Into<String>) {
        self.status = s.into();
        h.title = self.status.clone();
        h.invalidate();
    }
    fn sync_toolbar(&mut self, h: &mut Host) -> Result<(), String> {
        let visible = self.settings.floating_button;
        if visible && (self.toolbar == 0 || unsafe { IsWindow(self.toolbar as HWND) } == 0) {
            let window = create(
                "轻取悬浮工具条",
                (198., 34.),
                Content::Toolbar,
                h.proxy.clone(),
                &self.settings.palette,
                self.settings.dark,
                h.font_size,
            )?;
            self.toolbar = window as usize;
            self.children.push(window as usize);
        }
        if self.toolbar != 0 {
            unsafe {
                ShowWindow(
                    self.toolbar as HWND,
                    if visible { SW_SHOWNOACTIVATE } else { SW_HIDE },
                );
            }
        }
        if let Some(desktop) = &self.desktop {
            desktop.toolbar_visible(visible);
        }
        Ok(())
    }
    fn sync_desktop_option(&self, command: u16, enabled: bool) {
        unsafe {
            if self.settings_hwnd != 0 && IsWindow(self.settings_hwnd as HWND) != 0 {
                PostMessageW(
                    self.settings_hwnd as HWND,
                    COMMAND,
                    command as usize | (usize::from(enabled) << 16),
                    0,
                );
            }
        }
    }
    fn set_toolbar_visible(&mut self, h: &mut Host, visible: bool) {
        let previous = self.settings.floating_button;
        self.settings.floating_button = visible;
        if let Err(error) = self.sync_toolbar(h) {
            self.settings.floating_button = previous;
            self.set_status(h, error);
            return;
        }
        self.sync_desktop_option(settings::SYNC_TOOLBAR, visible);
        match config::save_preferences(&self.settings) {
            Ok(()) => self.set_status(
                h,
                if visible {
                    "已显示工具悬浮窗。"
                } else {
                    "已关闭工具悬浮窗。"
                },
            ),
            Err(error) => self.set_status(h, error),
        }
    }
    fn toggle_startup(&mut self, h: &mut Host) {
        let enabled = !platform::startup_enabled();
        if let Err(error) = platform::set_startup(enabled) {
            self.set_status(h, error);
            return;
        }
        self.settings.startup = enabled;
        self.sync_desktop_option(settings::SYNC_STARTUP, enabled);
        match config::save_preferences(&self.settings) {
            Ok(()) => self.set_status(
                h,
                if enabled {
                    "已开启开机自启，下次登录后自动驻留托盘。"
                } else {
                    "已关闭开机自启。"
                },
            ),
            Err(error) => self.set_status(h, error),
        }
    }
    fn create_shortcut(&mut self, h: &mut Host, target: platform::Shortcut) {
        match platform::create_shortcut(target) {
            Ok(path) => self.set_status(h, format!("已创建快捷方式：{}", path.display())),
            Err(error) => self.set_status(h, error),
        }
    }
    pub fn open_settings(&mut self, h: &mut Host) {
        unsafe {
            if self.settings_hwnd != 0 && IsWindow(self.settings_hwnd as HWND) != 0 {
                show_main(self.settings_hwnd as HWND);
                return;
            }
        }
        self.settings.startup = platform::startup_enabled();
        let work = platform::primary_work_size();
        let state = SettingsEditor::new(self.settings.clone(), self.keys.clone());
        match create(
            "轻取 OCR · 设置",
            (840f32.min(work.0 - 30.), 530f32.min(work.1 - 40.)),
            Content::Settings(Box::new(state)),
            h.proxy.clone(),
            &self.settings.palette,
            self.settings.dark,
            h.font_size,
        ) {
            Ok(window) => {
                self.settings_hwnd = window as usize;
                self.children.push(window as usize);
            }
            Err(e) => self.set_status(h, e),
        }
    }
    pub fn note(&mut self, h: &mut Host, body: String) -> Result<HWND, String> {
        let hwnd = create(
            "便签",
            (360., 300.),
            Content::Note(Note::new(body)),
            h.proxy.clone(),
            &self.settings.palette,
            self.settings.dark,
            h.font_size,
        )?;
        self.children.push(hwnd as usize);
        Ok(hwnd)
    }

    fn picture_shared(
        &mut self,
        h: &mut Host,
        image: crate::image_store::ImageView,
    ) -> Result<HWND, String> {
        let dpi_scale = h.dpi as f32 / 96.;
        let scale = (h.px(850.) as f32 / image.width() as f32)
            .min(h.px(650.) as f32 / image.height() as f32)
            .min(1.);
        let size = (
            (image.width() as f32 * scale + (PICTURE_FRAME * 2) as f32) / dpi_scale,
            (image.height() as f32 * scale + (PICTURE_FRAME * 2) as f32) / dpi_scale
                + if self.settings.picture_info { 24. } else { 0. },
        );
        let hwnd = create(
            "图片剪贴板",
            size,
            Content::Picture(Picture::new(image).with_info(self.settings.picture_info)),
            h.proxy.clone(),
            &self.settings.palette,
            self.settings.dark,
            h.font_size,
        )?;
        self.children.push(hwnd as usize);
        Ok(hwnd)
    }
    fn reset_pins(&mut self, h: &mut Host) {
        unsafe {
            let mut cursor = POINT::default();
            GetCursorPos(&mut cursor);
            let monitor = MonitorFromPoint(cursor, MONITOR_DEFAULTTONEAREST);
            let mut count = 0;
            for &child in &self.children {
                let child = child as HWND;
                if matches!(
                    GetPropW(child, platform::wide("QingquRole").as_ptr()) as usize,
                    2 | 3
                ) && PostMessageW(child, RESET_PIN, count, monitor as LPARAM) != 0
                {
                    count += 1;
                }
            }
            self.set_status(h, format!("已复位 {count} 个 Alt+V 剪贴板窗口。"));
        }
    }
    fn set_picture_info(&mut self, h: &mut Host, visible: bool) {
        self.settings.picture_info = visible;
        if let Err(error) = config::save_preferences(&self.settings) {
            self.set_status(h, error);
        }
        unsafe {
            for &child in &self.children {
                let child = child as HWND;
                if GetPropW(child, platform::wide("QingquRole").as_ptr()) as usize == 3 {
                    PostMessageW(child, PICTURE_INFO, usize::from(visible), 0);
                }
            }
        }
    }
    fn sync_picture_menu(&self, h: &Host) {
        if let Some(proxy) = &h.proxy {
            if let Ok(mut label) = proxy.picture_menu.lock() {
                *label = self.settings.picture_menu_label();
            }
        }
    }
    pub fn toggle_translation(&mut self, h: &mut Host, visible: bool) {
        if self.settings.show_translation == visible {
            return;
        }
        if !visible {
            self.settings.auto_translate = false;
            if self
                .pending
                .as_ref()
                .is_some_and(|p| matches!(p.purpose, Purpose::Translate))
            {
                self.cancel(h);
            }
        }
        unsafe {
            let mut r = RECT::default();
            GetWindowRect(h.hwnd, &mut r);
            let width = r.right - r.left;
            self.settings.show_translation = visible;
            h.min_size.0 = if visible { 600. } else { 300. };
            let new_width = if visible {
                width + self.translation_width
            } else {
                let source = (width as f32 * self.source_fraction) as i32;
                self.translation_width = width - source;
                source.max(h.px(300.))
            };
            SetWindowPos(
                h.hwnd,
                null_mut(),
                r.left,
                r.top,
                new_width,
                r.bottom - r.top,
                SWP_NOZORDER | SWP_NOACTIVATE,
            );
        }
        self.layout(h);
        h.invalidate();
    }
    fn preview(&mut self, h: &mut Host, target: HWND, range: Range, before: String, after: String) {
        if before == after {
            self.set_status(h, "内容无需更改。");
            return;
        }
        if !self.preview {
            self.cancel_editor_job(h, target);
            unsafe {
                replace_preserving_view(target, range, &after);
            }
            return;
        }
        let state = Preview {
            target: target as usize,
            range,
            before,
            after,
            left: null_mut(),
            right: null_mut(),
        };
        if let Ok(window) = create(
            "排版预览 · 确认后应用",
            (760., 420.),
            Content::Preview(state),
            h.proxy.clone(),
            &h.palette,
            h.dark,
            h.font_size,
        ) {
            self.children.push(window as usize);
        }
    }
    fn open_tables(&mut self, h: &mut Host) {
        if self.tables.is_empty() {
            self.set_status(h, "当前没有表格识别结果。");
            return;
        }
        if let Ok(window) = create(
            "表格结果",
            (760., 460.),
            Content::Tables(Tables::new(self.tables.clone())),
            h.proxy.clone(),
            &h.palette,
            h.dark,
            h.font_size,
        ) {
            self.children.push(window as usize);
        }
    }
    pub fn mouse(&mut self, h: &mut Host, msg: u32, l: LPARAM) -> bool {
        if !self.settings.show_translation {
            return false;
        }
        let (x, y) = (
            (l as u32 & 65535) as i16 as i32,
            ((l as u32 >> 16) & 65535) as i16 as i32,
        );
        let (w, height) = dimensions(h.hwnd);
        let split = (w as f32 * self.source_fraction) as i32;
        unsafe {
            if msg == WM_LBUTTONDOWN
                && (x - split).abs() < h.px(5.)
                && y > h.px(22.)
                && y < height - h.px(26.)
            {
                self.dragging = true;
                SetCapture(h.hwnd);
                return true;
            }
            if msg == WM_LBUTTONUP && self.dragging {
                self.dragging = false;
                ReleaseCapture();
                return true;
            }
            if msg == WM_MOUSEMOVE && (self.dragging || (x - split).abs() < h.px(5.)) {
                SetCursor(LoadCursorW(null_mut(), IDC_SIZEWE));
                if self.dragging {
                    self.source_fraction = x.clamp(h.px(250.), (w - h.px(250.)).max(h.px(250.)))
                        as f32
                        / w.max(1) as f32;
                    self.layout(h);
                    h.invalidate();
                }
                return true;
            }
        }
        false
    }
    fn apply_theme(&mut self, h: &mut Host) {
        h.change_theme(&self.settings.palette, self.settings.dark);
        self.refresh_document(h);
        let theme = (if self.settings.palette == "catppuccin" {
            2
        } else {
            0
        }) + usize::from(self.settings.dark);
        for &child in &self.children {
            unsafe {
                PostMessageW(child as HWND, APPLY_THEME, theme, 0);
            }
        }
    }
    fn apply_settings(
        &mut self,
        h: &mut Host,
        mut settings: Settings,
        keys: Credentials,
        return_to_interfaces: bool,
    ) {
        // This preference belongs to the picture menu. An already-open
        // settings dialog must not replace it with its older snapshot.
        settings.picture_info = self.settings.picture_info;
        settings.normalize_ocr_modes();
        settings.normalize_ocr_language();
        settings.normalize_processing_modes();
        let result = (|| {
            platform::validate_bindings(&settings.hotkeys)?;
            if let Some(d) = &self.desktop {
                let bindings = settings.hotkeys.clone();
                d.rebind(bindings)?;
            }
            if settings.startup || platform::startup_enabled() {
                platform::set_startup(settings.startup)?;
            }
            config::save(&settings, &keys)
        })();
        if let Err(e) = result {
            if let Some(d) = &self.desktop {
                let _ = d.rebind(self.settings.hotkeys.clone());
            }
            unsafe {
                win::error(self.settings_hwnd as HWND, &e);
            }
            return;
        }
        let visible = settings.show_translation;
        let was_visible = self.settings.show_translation;
        let topmost_changed = self.settings.default_topmost != settings.default_topmost;
        let target_changed = self.settings.translation.target != settings.translation.target;
        if target_changed
            && self
                .pending
                .as_ref()
                .is_some_and(|p| matches!(p.purpose, Purpose::Translate))
        {
            self.cancel(h);
        }
        self.settings = settings;
        self.keys = keys;
        self.sync_picture_menu(h);
        if topmost_changed {
            h.top = self.settings.default_topmost;
            unsafe {
                win::set_top(h.hwnd, h.top);
                if let Some(button) = h.buttons.iter_mut().find(|button| button.id == PIN) {
                    button.icon = Some(if h.top { Icon::Pin } else { Icon::Unpin });
                    InvalidateRect(button.hwnd, null(), 0);
                }
            }
        }
        self.sync_ocr_language_selector(h);
        self.sync_translation_target(h);
        self.sync_modes(h);
        self.preview = false;
        self.settings.show_translation = was_visible;
        self.toggle_translation(h, visible);
        h.set_font_size(self.settings.editor_font_size);
        self.apply_theme(h);
        if let Err(error) = self.sync_toolbar(h) {
            self.set_status(h, error);
        }
        if self.settings_hwnd != 0 {
            unsafe {
                if return_to_interfaces {
                    PostMessageW(
                        self.settings_hwnd as HWND,
                        COMMAND,
                        settings::SAVED as usize,
                        0,
                    );
                } else {
                    PostMessageW(self.settings_hwnd as HWND, WM_CLOSE, 0, 0);
                }
            }
        }
        self.set_status(h, "设置已保存并应用。");
    }
    pub fn timer(&mut self, h: &mut Host, id: usize) {
        match id {
            1 => {
                if let Some(p) = &self.pending {
                    h.title = format!("{} · {:.1} s", p.label, p.started.elapsed().as_secs_f32());
                    h.invalidate();
                }
            }
            5 => {
                unsafe {
                    KillTimer(h.hwnd, 5);
                }
                self.refresh_document(h);
            }
            _ => {}
        }
    }

    pub(super) fn has_tray(&self) -> bool {
        self.desktop.as_ref().is_some_and(|d| d.has_tray)
    }
    pub fn close(&mut self, h: &mut Host) -> bool {
        if !self.exiting
            && self.settings.close_to_tray
            && self.desktop.as_ref().is_some_and(|d| d.has_tray)
        {
            unsafe {
                ShowWindow(h.hwnd, SW_HIDE);
            }
            return false;
        }
        self.cancel(h);
        for &window in &self.children {
            unsafe {
                if IsWindow(window as HWND) != 0 {
                    DestroyWindow(window as HWND);
                }
            }
        }
        self.children.clear();
        self.desktop.take();
        true
    }
}
