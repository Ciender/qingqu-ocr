mod api;
mod bindings;
mod reference;

use super::*;
use crate::config::{Credentials, Settings};
use serde_json::{json, Value};
use std::time::{Duration, Instant};

pub(super) const SAVED: u16 = 505;

#[derive(Clone)]
enum Input {
    Text,
    Secret,
    Check,
    Choice(Vec<(&'static str, &'static str)>),
    Preset(Vec<(&'static str, &'static str)>),
    Multiline,
    Number,
    Integer(usize, usize),
    Hotkey(usize),
    Heading,
}
struct Field {
    path: String,
    secret: bool,
    kind: Input,
    label: HWND,
    edit: HWND,
    record: HWND,
    height: f32,
}
pub struct SettingsEditor {
    draft: Value,
    keys: Value,
    category: usize,
    api: usize,
    service: usize,
    fields: Vec<Field>,
    offset: i32,
    total: i32,
    recording: Option<(usize, Instant)>,
    show_secrets: bool,
    api_overview: bool,
    help: HWND,
    reference: HWND,
    reference_urls: Vec<&'static str>,
    reference_actions: Vec<crate::service_info::Source>,
}
impl SettingsEditor {
    pub fn new(mut settings: Settings, mut keys: Credentials) -> Self {
        settings.normalize_ocr_modes();
        settings.normalize_processing_modes();
        settings.translation.ensure_profiles(&mut keys);
        Self {
            draft: serde_json::to_value(settings).unwrap(),
            keys: serde_json::to_value(keys).unwrap(),
            category: 3,
            api: 0,
            service: 0,
            fields: Vec::new(),
            offset: 0,
            total: 0,
            recording: None,
            show_secrets: true,
            api_overview: true,
            help: null_mut(),
            reference: null_mut(),
            reference_urls: Vec::new(),
            reference_actions: Vec::new(),
        }
    }
    pub fn init(&mut self, h: &mut Host) -> Result<(), String> {
        unsafe {
            h.title_buttons(false, false);
            h.min_size = (760., 480.);
            let list = h.control(
                "LISTBOX",
                "",
                WS_TABSTOP | LBS_NOTIFY as u32 | LBS_NOINTEGRALHEIGHT as u32,
                500,
            );
            for s in ["常规", "高级", "配置", "热键", "API 服务", "调用配置"] {
                SendMessageW(list, LB_ADDSTRING, 0, platform::wide(s).as_ptr() as isize);
            }
            SendMessageW(list, LB_SETCURSEL, 0, 0);
            h.button(501, None, "保存并应用");
            h.button(502, None, "取消");
            h.button(503, Some(Icon::Eye), "显示 / 隐藏密钥");
            h.button(504, None, "恢复默认快捷键");
            h.button(511, None, "返回接口列表");
            h.button(
                512,
                Some(Icon::Help),
                "免费额度与计费 · 悬停或点击查看官方资料",
            );
            self.init_reference(h)?;
            h.button(930, None, "打开官方密钥管理页，复制后填写上方凭据");
            h.button(931, None, "打开官方服务控制台，开通服务并查看调用量");
            self.init_bindings(h);
            self.build(h);
        }
        Ok(())
    }
    fn add(&mut self, h: &mut Host, label: &str, path: &str, kind: Input, secret: bool) {
        unsafe {
            let index = self.fields.len();
            let value = if secret {
                self.keys.pointer(path)
            } else {
                self.draft.pointer(path)
            };
            let value = value.cloned().unwrap_or(Value::Null);
            let label_hwnd = h.control("STATIC", label, 0, 700 + index as u16);
            let id = 520 + index as u16;
            let edit = match &kind {
                Input::Heading => null_mut(),
                Input::Check => {
                    let edit = h.control("BUTTON", label, WS_TABSTOP | BS_AUTOCHECKBOX as u32, id);
                    SendMessageW(
                        edit,
                        BM_SETCHECK,
                        usize::from(value.as_bool().unwrap_or(false)),
                        0,
                    );
                    edit
                }
                Input::Choice(options) | Input::Preset(options) => {
                    let edit = h.control(
                        "COMBOBOX",
                        "",
                        WS_TABSTOP | CBS_DROPDOWNLIST as u32 | WS_VSCROLL,
                        id,
                    );
                    for &(_, label) in options {
                        SendMessageW(
                            edit,
                            CB_ADDSTRING,
                            0,
                            platform::wide(label).as_ptr() as isize,
                        );
                    }
                    SendMessageW(
                        edit,
                        CB_SETCURSEL,
                        options
                            .iter()
                            .position(|(v, _)| Some(*v) == value.as_str())
                            .unwrap_or(0),
                        0,
                    );
                    edit
                }
                _ => {
                    let style = WS_BORDER
                        | WS_TABSTOP
                        | if matches!(kind, Input::Multiline) {
                            ES_MULTILINE as u32
                                | ES_AUTOVSCROLL as u32
                                | ES_WANTRETURN as u32
                                | WS_VSCROLL
                        } else {
                            ES_AUTOHSCROLL as u32
                        }
                        | if matches!(kind, Input::Secret) && !self.show_secrets {
                            ES_PASSWORD as u32
                        } else {
                            0
                        };
                    let edit = h.control("EDIT", "", style, id);
                    set_text(
                        edit,
                        &match &value {
                            Value::String(s) => s.clone(),
                            Value::Number(n) => n.to_string(),
                            _ => String::new(),
                        },
                    );
                    SendMessageW(
                        edit,
                        EM_SETLIMITTEXT,
                        if matches!(kind, Input::Multiline) {
                            64 * 1024
                        } else {
                            8192
                        },
                        0,
                    );
                    edit
                }
            };
            let record = if let Input::Hotkey(i) = kind {
                h.button(600 + i as u16, None, "录制")
            } else {
                null_mut()
            };
            self.fields.push(Field {
                path: path.into(),
                secret,
                kind: kind.clone(),
                label: label_hwnd,
                edit,
                record,
                height: if matches!(kind, Input::Heading) {
                    38.
                } else if matches!(kind, Input::Multiline) {
                    95.
                } else {
                    34.
                },
            });
        }
    }
    fn stop_recording(&mut self, h: &Host) {
        if let Some((i, _)) = self.recording.take() {
            unsafe {
                KillTimer(h.hwnd, 1);
                set_text(GetDlgItem(h.hwnd, 600 + i as i32), "录制");
            }
            if let Some(p) = &h.proxy {
                p.send(Notice::Record(false));
            }
        }
    }
    fn flush(&mut self) {
        unsafe {
            for field in &self.fields {
                if matches!(field.kind, Input::Heading | Input::Preset(_)) {
                    continue;
                }
                let value = match &field.kind {
                    Input::Check => Value::Bool(
                        SendMessageW(field.edit, BM_GETCHECK, 0, 0) == BST_CHECKED as isize,
                    ),
                    Input::Choice(options) => Value::String(
                        options
                            .get(SendMessageW(field.edit, CB_GETCURSEL, 0, 0).max(0) as usize)
                            .map_or("", |x| x.0)
                            .into(),
                    ),
                    Input::Number => json!(text(field.edit)
                        .parse::<f32>()
                        .unwrap_or(14.)
                        .clamp(8., 48.)),
                    Input::Integer(min, max) => json!(text(field.edit)
                        .parse::<usize>()
                        .unwrap_or(*min)
                        .clamp(*min, *max)),
                    _ => Value::String(text(field.edit)),
                };
                let object = if field.secret {
                    &mut self.keys
                } else {
                    &mut self.draft
                };
                if let Some(destination) = object.pointer_mut(&field.path) {
                    *destination = value;
                }
            }
        }
    }
    fn build(&mut self, h: &mut Host) {
        self.close_help();
        self.stop_recording(h);
        if let Ok(mut settings) = serde_json::from_value::<Settings>(self.draft.clone()) {
            settings.normalize_ocr_modes();
            self.draft = serde_json::to_value(settings).unwrap();
        }
        let handles = self
            .fields
            .drain(..)
            .flat_map(|f| [f.label, f.edit, f.record])
            .filter(|p| !p.is_null())
            .collect::<Vec<_>>();
        unsafe {
            h.button_label(
                501,
                if self.category == 0 && !self.api_overview {
                    "保存并返回"
                } else {
                    "保存并应用"
                },
            );
            for &handle in &handles {
                let tool = TTTOOLINFOW {
                    cbSize: size_of::<TTTOOLINFOW>() as u32,
                    hwnd: h.hwnd,
                    uId: handle as usize,
                    ..Default::default()
                };
                SendMessageW(
                    h.tooltip,
                    TTM_DELTOOLW,
                    0,
                    (&tool as *const TTTOOLINFOW) as isize,
                );
                DestroyWindow(handle);
            }
            h.controls.retain(|p| !handles.contains(p));
            h.buttons.retain(|b| !handles.contains(&b.hwnd));
            ShowWindow(
                GetDlgItem(h.hwnd, 503),
                if self.category == 5 { SW_SHOW } else { SW_HIDE },
            );
            ShowWindow(
                GetDlgItem(h.hwnd, 504),
                if self.category == 1 { SW_SHOW } else { SW_HIDE },
            );
            ShowWindow(
                GetDlgItem(h.hwnd, 511),
                if self.category == 0 && !self.api_overview {
                    SW_SHOW
                } else {
                    SW_HIDE
                },
            );
            ShowWindow(
                GetDlgItem(h.hwnd, 512),
                if self.category == 5 || (self.category == 0 && !self.api_overview) {
                    SW_SHOW
                } else {
                    SW_HIDE
                },
            );
            for i in 0..6 {
                for id in [800 + i, 810 + i, 820 + i, 830 + i, 850 + i] {
                    ShowWindow(
                        GetDlgItem(h.hwnd, id),
                        if self.category == 0 && self.api_overview {
                            SW_SHOW
                        } else {
                            SW_HIDE
                        },
                    );
                }
            }
            ShowWindow(
                GetDlgItem(h.hwnd, 840),
                if self.category == 5 { SW_SHOW } else { SW_HIDE },
            );
            ShowWindow(
                GetDlgItem(h.hwnd, 513),
                if self.category == 0 { SW_SHOW } else { SW_HIDE },
            );
            self.sync_bindings(h);
        }
        self.offset = 0;
        self.refresh_reference(h);
        if self.category == 0 && self.api_overview {
            self.layout(h);
            h.invalidate();
            return;
        }
        match self.category {
            0 => self.binding_fields(h),
            5 => self.service_fields(h),
            1 => {
                let bindings: Vec<config::Binding> =
                    serde_json::from_value(self.draft["hotkeys"].clone()).unwrap_or_default();
                for (i, b) in bindings.iter().enumerate() {
                    let name = config::ACTIONS
                        .iter()
                        .find(|(action, _)| *action == b.action)
                        .map_or(b.action.as_str(), |(_, name)| *name);
                    self.add(
                        h,
                        name,
                        &format!("/hotkeys/{i}/key"),
                        Input::Hotkey(i),
                        false,
                    );
                }
            }
            2 => {
                self.add(h, "主题与配色", "", Input::Heading, false);
                self.add(
                    h,
                    "配色系列",
                    "/palette",
                    Input::Choice(vec![
                        ("vscode", "VS Code Modern"),
                        ("catppuccin", "Catppuccin"),
                    ]),
                    false,
                );
                self.add(h, "深色模式", "/dark", Input::Check, false);
                self.add(
                    h,
                    "编辑字号（8–48）",
                    "/editor_font_size",
                    Input::Number,
                    false,
                );
            }
            3 => {
                self.add(h, "启动时", "", Input::Heading, false);
                for (label, path) in [
                    ("当前用户开机启动", "/startup"),
                    ("关闭主窗到托盘", "/close_to_tray"),
                    ("显示悬浮工具条", "/floating_button"),
                    ("主窗口默认置顶", "/default_topmost"),
                ] {
                    self.add(h, label, path, Input::Check, false);
                }
                self.add(h, "识别时", "", Input::Heading, false);
                self.add(
                    h,
                    "自动纠正文字方向",
                    "/correct_orientation",
                    Input::Check,
                    false,
                );
                self.add(h, "识别后", "", Input::Heading, false);
                for (label, path) in [
                    ("自动复制处理后的结果", "/auto_copy"),
                    ("自动翻译", "/auto_translate"),
                    ("显示翻译栏", "/show_translation"),
                ] {
                    self.add(h, label, path, Input::Check, false);
                }
            }
            _ => {
                self.add(
                    h,
                    "识别文本配置 · 工具栏切换后持续应用",
                    "",
                    Input::Heading,
                    false,
                );
                self.add(
                    h,
                    "默认分段模式",
                    "/processing/layout",
                    Input::Choice(vec![
                        ("smart", "自动分段"),
                        ("split", "段落拆分"),
                        ("merge", "段落合并"),
                    ]),
                    false,
                );
                self.add(
                    h,
                    "标点转换",
                    "/processing/punctuation",
                    Input::Choice(vec![("chinese", "中文标点"), ("english", "英文标点")]),
                    false,
                );
                self.add(
                    h,
                    "简繁转换",
                    "/processing/chinese",
                    Input::Choice(vec![
                        ("", "关闭"),
                        ("simplified", "转为简体"),
                        ("traditional", "转为繁体"),
                    ]),
                    false,
                );
                self.add(
                    h,
                    "智能排版选项 · 仅点击排版按钮时使用",
                    "",
                    Input::Heading,
                    false,
                );
                for (label, path) in [
                    ("修复段内折行", "/format_options/repair_wrap"),
                    ("识别段落边界", "/format_options/paragraphs"),
                    ("清理空白", "/format_options/trim"),
                    ("段间空一行", "/format_options/blank_lines"),
                    ("首行缩进", "/format_options/indent"),
                ] {
                    self.add(h, label, path, Input::Check, false);
                }
                self.add(
                    h,
                    "重排列宽",
                    "/wrap_columns",
                    Input::Integer(5, 300),
                    false,
                );
                self.add(
                    h,
                    "缩进空格数",
                    "/first_line_indent",
                    Input::Integer(0, 16),
                    false,
                );
                self.add(h, "自定义拆分符", "/split_delimiter", Input::Text, false);
            }
        }
        self.layout(h);
        h.invalidate();
    }
    pub fn layout(&mut self, h: &mut Host) {
        self.close_help();
        let (w, height) = dimensions(h.hwnd);
        let sidebar = h.px(116.);
        let top = h.px(65.);
        let bottom = height
            - h.px(54.)
            - if self.category == 0 && !self.api_overview {
                self.reference_height(h)
            } else {
                0
            };
        unsafe {
            move_to(
                GetDlgItem(h.hwnd, 500),
                rect(
                    h.px(8.),
                    h.px(40.),
                    sidebar - h.px(16.),
                    height - h.px(102.),
                ),
            );
            move_to(
                GetDlgItem(h.hwnd, 501),
                rect(w - h.px(217.), height - h.px(42.), h.px(120.), h.px(30.)),
            );
            move_to(
                GetDlgItem(h.hwnd, 502),
                rect(w - h.px(88.), height - h.px(42.), h.px(76.), h.px(30.)),
            );
            move_to(
                GetDlgItem(h.hwnd, 503),
                rect(h.px(11.), height - h.px(42.), h.px(30.), h.px(30.)),
            );
            move_to(
                GetDlgItem(h.hwnd, 504),
                rect(
                    sidebar + h.px(15.),
                    height - h.px(42.),
                    h.px(128.),
                    h.px(30.),
                ),
            );
            move_to(
                GetDlgItem(h.hwnd, 511),
                rect(
                    sidebar + h.px(15.),
                    height - h.px(42.),
                    h.px(116.),
                    h.px(30.),
                ),
            );
            self.layout_bindings(h, w);
            move_to(
                GetDlgItem(h.hwnd, 840),
                rect(
                    sidebar + h.px(15.),
                    h.px(66.),
                    w - sidebar - h.px(32.),
                    h.px(290.),
                ),
            );
            move_to(
                GetDlgItem(h.hwnd, 513),
                rect(
                    sidebar + h.px(if self.api_overview { 15. } else { 142. }),
                    height - h.px(42.),
                    h.px(116.),
                    h.px(30.),
                ),
            );
            move_to(
                GetDlgItem(h.hwnd, 512),
                rect(w - h.px(44.), h.px(28.), h.px(26.), h.px(26.)),
            );
            self.layout_reference(h);
            let mut y = if self.category == 5 { h.px(105.) } else { top } - self.offset;
            let label_width = h.px(137.);
            let edit_left = sidebar + h.px(15.) + label_width;
            for field in &self.fields {
                let item_height = h.px(field.height);
                let visible = y >= top && y + item_height <= bottom + h.px(5.);
                for handle in [field.label, field.edit, field.record] {
                    if !handle.is_null() {
                        ShowWindow(handle, if visible { SW_SHOW } else { SW_HIDE });
                    }
                }
                if matches!(field.kind, Input::Heading) {
                    move_to(
                        field.label,
                        rect(
                            sidebar + h.px(15.),
                            y + h.px(5.),
                            w - sidebar - h.px(30.),
                            h.px(27.),
                        ),
                    );
                    y += item_height;
                    continue;
                }
                if matches!(field.kind, Input::Check) {
                    ShowWindow(field.label, SW_HIDE);
                    move_to(
                        field.edit,
                        rect(
                            sidebar + h.px(15.),
                            y,
                            w - sidebar - h.px(30.),
                            item_height - h.px(7.),
                        ),
                    );
                    y += item_height;
                    continue;
                }
                let record_width = if field.record.is_null() { 0 } else { h.px(62.) };
                move_to(
                    field.label,
                    rect(
                        sidebar + h.px(15.),
                        y + h.px(4.),
                        label_width - h.px(6.),
                        h.px(27.),
                    ),
                );
                move_to(
                    field.edit,
                    rect(
                        edit_left,
                        y,
                        (w - edit_left - h.px(16.) - record_width).max(h.px(45.)),
                        if matches!(field.kind, Input::Choice(_) | Input::Preset(_)) {
                            h.px(230.)
                        } else {
                            item_height - h.px(7.)
                        },
                    ),
                );
                if !field.record.is_null() {
                    move_to(field.record, rect(w - h.px(73.), y, h.px(57.), h.px(27.)));
                }
                y += item_height;
            }
            self.total = y + self.offset - top;
            let info = SCROLLINFO {
                cbSize: size_of::<SCROLLINFO>() as u32,
                fMask: SIF_RANGE | SIF_PAGE | SIF_POS,
                nMin: 0,
                nMax: self.total.max(1) - 1,
                nPage: (bottom - top).max(1) as u32,
                nPos: self.offset,
                nTrackPos: 0,
            };
            SetScrollInfo(h.hwnd, SB_VERT, &info, 1);
        }
    }
    pub fn command(&mut self, h: &mut Host, id: u16, code: u16) {
        unsafe {
            if (930..932).contains(&id) {
                if let Some(link) = self.reference_actions.get((id - 930) as usize) {
                    reference::open_reference(h, link.url);
                }
            } else if service_help::is_button(id as usize) {
                self.flush();
                self.close_help();
                let index = if id == 512 {
                    self.api
                } else {
                    (id - 820) as usize
                };
                if self.category == 0 && self.api_overview {
                    self.api = index;
                    self.refresh_reference(h);
                    self.layout_reference(h);
                }
                if let Ok(settings) = serde_json::from_value::<Settings>(self.draft.clone()) {
                    let (settings, index) = self.reference_profile(settings, index);
                    let mut info =
                        crate::service_info::ServiceInfo::for_interface(&settings, index);
                    let mut links =
                        crate::service_info::ServiceInfo::setup_sources(&settings, index);
                    links.append(&mut info.sources);
                    info.sources = links;
                    SendMessageW(h.tooltip, TTM_POP, 0, 0);
                    match service_help::ServiceHelp::open(h, GetDlgItem(h.hwnd, id as i32), info) {
                        Ok(hwnd) => self.help = hwnd,
                        Err(message) => win::error(h.hwnd, &message),
                    }
                }
            } else if id == 500 && code == LBN_SELCHANGE as u16 {
                self.flush();
                self.category =
                    [3, 2, 4, 1, 5, 0][SendMessageW(GetDlgItem(h.hwnd, 500), LB_GETCURSEL, 0, 0)
                        .clamp(0, 5) as usize];
                if self.category == 0 {
                    self.api_overview = true;
                }
                self.build(h);
            } else if (800..806).contains(&id) && code == CBN_SELCHANGE as u16 {
                self.close_help();
                let index = (id - 800) as usize;
                let choice =
                    SendMessageW(GetDlgItem(h.hwnd, id as i32), CB_GETCURSEL, 0, 0).max(0) as usize;
                if let Some((value, _)) = config::interface_choices(index).get(choice) {
                    *self
                        .draft
                        .pointer_mut(config::INTERFACE_PATHS[index])
                        .unwrap() = json!(value);
                }
                self.api = index;
                self.refresh_reference(h);
                self.build(h);
            } else if (850..856).contains(&id) && code == CBN_SELCHANGE as u16 {
                let index = (id - 850) as usize;
                if let Ok(settings) = serde_json::from_value::<Settings>(self.draft.clone()) {
                    let provider = settings.interface_provider(index);
                    let selected = SendMessageW(GetDlgItem(h.hwnd, id as i32), CB_GETCURSEL, 0, 0)
                        .max(0) as usize;
                    if let (Some(path), Some(&(value, _))) = (
                        config::interface_mode_path(index, provider),
                        settings.interface_modes(index, provider).get(selected),
                    ) {
                        *self.draft.pointer_mut(path).unwrap() = json!(value);
                    }
                }
                self.api = index;
                self.build(h);
            } else if id == 840 && code == CBN_SELCHANGE as u16 {
                self.flush();
                self.service = SendMessageW(GetDlgItem(h.hwnd, 840), CB_GETCURSEL, 0, 0)
                    .clamp(0, config::API_SERVICES.len() as isize - 1)
                    as usize;
                self.build(h);
            } else if id == 513 {
                self.flush();
                if let Ok(settings) = serde_json::from_value::<Settings>(self.draft.clone()) {
                    self.service = config::API_SERVICES
                        .iter()
                        .position(|s| *s == settings.interface_provider(self.api))
                        .unwrap_or(0);
                }
                self.category = 5;
                SendMessageW(GetDlgItem(h.hwnd, 500), LB_SETCURSEL, 4, 0);
                SendMessageW(GetDlgItem(h.hwnd, 840), CB_SETCURSEL, self.service, 0);
                self.build(h);
            } else if (810..816).contains(&id) {
                self.flush();
                self.api = (id - 810) as usize;
                self.api_overview = false;
                self.build(h);
            } else if id == 511 {
                self.flush();
                self.api_overview = true;
                self.build(h);
            } else if code == CBN_SELCHANGE as u16
                && self
                    .fields
                    .iter()
                    .enumerate()
                    .any(|(i, f)| id == 520 + i as u16 && matches!(f.kind, Input::Preset(_)))
            {
                self.flush();
                if let Some(field) = self
                    .fields
                    .iter()
                    .enumerate()
                    .find(|(i, _)| id == 520 + *i as u16)
                    .map(|(_, f)| f)
                {
                    if let Input::Preset(options) = &field.kind {
                        let selected = SendMessageW(field.edit, CB_GETCURSEL, 0, 0).max(0) as usize;
                        if let Some(&(model, _)) =
                            options.get(selected).filter(|(model, _)| !model.is_empty())
                        {
                            let path = field.path.clone();
                            *self.draft.pointer_mut(&path).unwrap() = json!(model);
                            for f in &self.fields {
                                if f.path == path && matches!(f.kind, Input::Text) {
                                    set_text(f.edit, model);
                                }
                            }
                        }
                    }
                }
                self.refresh_reference(h);
            } else if code == CBN_SELCHANGE as u16
                && matches!(self.category, 0 | 5)
                && self.fields.iter().enumerate().any(|(i, f)| {
                    id == 520 + i as u16
                        && matches!(
                            f.path.as_str(),
                            "/translation/google_mode"
                                | "/translation/microsoft_mode"
                                | "/qwen_ocr/protocol"
                                | "/qwen_ocr/task"
                                | "/text_ocr_mode"
                                | "/picture_ocr_mode"
                        )
                })
            {
                self.flush();
                self.build(h);
            } else if id == SAVED {
                // Main acknowledges only after persistence and application
                // succeed; failed saves leave the credentials page intact.
                if self.category != 5 {
                    self.category = 0;
                    self.api_overview = true;
                    SendMessageW(GetDlgItem(h.hwnd, 500), LB_SETCURSEL, 5, 0);
                }
                self.build(h);
            } else if id == 501 {
                self.flush();
                self.stop_recording(h);
                let values = serde_json::from_value::<Settings>(self.draft.clone()).and_then(|s| {
                    serde_json::from_value::<Credentials>(self.keys.clone()).map(|k| (s, k))
                });
                match values {
                    Ok((settings, keys)) => {
                        if let Err(e) = platform::validate_bindings(&settings.hotkeys) {
                            win::error(h.hwnd, &e);
                        } else if let Some(p) = &h.proxy {
                            p.send(Notice::Settings {
                                settings: Box::new(settings),
                                keys: Box::new(keys),
                                return_to_interfaces: self.category == 5
                                    || (self.category == 0 && !self.api_overview),
                            });
                        }
                    }
                    Err(_) => win::error(h.hwnd, "设置内容格式无效。"),
                }
            } else if id == 502 {
                PostMessageW(h.hwnd, WM_CLOSE, 0, 0);
            } else if id == 503 {
                self.show_secrets = !self.show_secrets;
                for field in &self.fields {
                    if matches!(field.kind, Input::Secret) {
                        SendMessageW(
                            field.edit,
                            EM_SETPASSWORDCHAR,
                            if self.show_secrets { 0 } else { 0x25cf },
                            0,
                        );
                        InvalidateRect(field.edit, null(), 0);
                    }
                }
            } else if id == 504 {
                self.flush();
                self.draft["hotkeys"] = serde_json::to_value(Settings::default().hotkeys).unwrap();
                self.build(h);
            } else if (600..700).contains(&id) {
                let index = (id - 600) as usize;
                self.stop_recording(h);
                self.recording = Some((index, Instant::now()));
                set_text(GetDlgItem(h.hwnd, id as i32), "请按键");
                if let Some(p) = &h.proxy {
                    p.send(Notice::Record(true));
                }
                SetTimer(h.hwnd, 1, 40, None);
            } else if matches!(self.category, 0 | 5)
                && code == EN_CHANGE as u16
                && from_reference_field(self, id)
            {
                self.flush();
                self.refresh_reference(h);
            } else if matches!(self.category, 0 | 5) && code == CBN_SELCHANGE as u16 {
                self.flush();
                self.refresh_reference(h);
                self.layout_reference(h);
            }
        }
    }
    pub fn timer(&mut self, h: &mut Host) {
        unsafe {
            let Some((index, started)) = self.recording else {
                return;
            };
            if started.elapsed() < Duration::from_millis(200) {
                return;
            }
            if GetForegroundWindow() != h.hwnd {
                self.stop_recording(h);
                return;
            }
            if GetAsyncKeyState(VK_ESCAPE as i32) < 0 {
                self.stop_recording(h);
                return;
            }
            let key = (0x30..=0x5a)
                .chain(0x70..=0x87)
                .find(|&k| GetAsyncKeyState(k) < 0);
            if let Some(key) = key {
                let mut parts = Vec::new();
                if GetAsyncKeyState(VK_CONTROL as i32) < 0 {
                    parts.push("Ctrl".to_string());
                }
                if GetAsyncKeyState(VK_MENU as i32) < 0 {
                    parts.push("Alt".to_string());
                }
                if GetAsyncKeyState(VK_SHIFT as i32) < 0 {
                    parts.push("Shift".to_string());
                }
                if GetAsyncKeyState(VK_LWIN as i32) < 0 || GetAsyncKeyState(VK_RWIN as i32) < 0 {
                    parts.push("Win".to_string());
                }
                parts.push(if key >= 0x70 {
                    format!("F{}", key - 0x70 + 1)
                } else {
                    char::from_u32(key as u32).unwrap_or('?').to_string()
                });
                let text = parts.join("+");
                if platform::parse_hotkey(&text).is_ok() {
                    if let Some(field) = self
                        .fields
                        .iter()
                        .find(|f| matches!(f.kind,Input::Hotkey(i)if i==index))
                    {
                        set_text(field.edit, &text);
                    }
                    self.stop_recording(h);
                }
            }
        }
    }
    pub fn scroll(&mut self, h: &mut Host, w: usize) {
        let mut info = SCROLLINFO {
            cbSize: size_of::<SCROLLINFO>() as u32,
            fMask: SIF_ALL,
            ..Default::default()
        };
        unsafe {
            GetScrollInfo(h.hwnd, SB_VERT, &mut info);
        }
        let page = info.nPage as i32;
        match w as u16 as i32 {
            SB_LINEUP => self.offset -= h.px(34.),
            SB_LINEDOWN => self.offset += h.px(34.),
            SB_PAGEUP => self.offset -= page,
            SB_PAGEDOWN => self.offset += page,
            SB_THUMBTRACK | SB_THUMBPOSITION => self.offset = info.nTrackPos,
            _ => {}
        }
        self.offset = self.offset.clamp(0, (self.total - page).max(0));
        self.layout(h);
        h.invalidate();
    }
    pub fn wheel(&mut self, h: &mut Host, w: usize) {
        let delta = (w >> 16) as u16 as i16;
        for _ in 0..3 {
            self.scroll(h, if delta > 0 { SB_LINEUP } else { SB_LINEDOWN } as usize);
        }
    }
    pub fn theme(&mut self, h: &mut Host) {
        self.close_help();
        h.invalidate();
    }
    fn close_help(&mut self) {
        unsafe {
            if !self.help.is_null()
                && !GetPropW(self.help, platform::wide("QingquServiceHelp").as_ptr()).is_null()
            {
                DestroyWindow(self.help);
            }
        }
        self.help = null_mut();
    }
    pub(super) fn dismiss_help(&self) {
        if !self.help.is_null() {
            unsafe {
                PostMessageW(self.help, service_help::DISMISS, 0, 0);
            }
        }
    }
    pub unsafe fn paint(&self, h: &Host, dc: HDC) {
        let (w, height) = dimensions(h.hwnd);
        let sidebar = h.px(116.);
        fill(
            dc,
            &rect(0, h.px(22.), sidebar, height - h.px(22.)),
            h.theme.surface,
        );
        label(
            dc,
            &rect(
                sidebar + h.px(15.),
                h.px(28.),
                w - sidebar - h.px(30.),
                h.px(27.),
            ),
            ["调用配置", "热键", "高级", "常规", "配置", "API 服务"]
                .get(self.category)
                .unwrap_or(&"设置"),
            h.theme.text,
            h.font.0,
            DT_SINGLELINE | DT_VCENTER,
        );
        if self.category == 0 && self.api_overview {
            self.paint_bindings(h, dc, w);
        }
        self.paint_reference(h, dc);
        if self.category == 0 && !self.api_overview && self.api == 6 {
            label(
                dc,
                &rect(
                    sidebar + h.px(15.),
                    height - h.px(92.),
                    w - sidebar - h.px(30.),
                    h.px(30.),
                ),
                "仅保留配置，服务协议确认后启用。",
                h.theme.muted,
                h.font.0,
                DT_WORDBREAK,
            );
        }
    }
}

fn from_reference_field(editor: &SettingsEditor, id: u16) -> bool {
    editor.fields.iter().enumerate().any(|(i, field)| {
        id == 520 + i as u16
            && !field.secret
            && matches!(
                field.path.as_str(),
                "/qwen_ocr/model" | "/qwen_vl/model" | "/qwen_ocr/base_url" | "/qwen_vl/base_url"
            )
    })
}
