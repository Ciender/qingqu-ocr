use super::*;
use crate::ocr::Table;

pub struct Note {
    initial: String,
    pub edit: HWND,
    lines: bool,
}
impl Note {
    pub fn new(body: String) -> Self {
        Self {
            initial: body,
            edit: null_mut(),
            lines: true,
        }
    }
    pub fn init(&mut self, h: &mut Host) -> Result<(), String> {
        unsafe {
            h.title_buttons(false, true);
            self.edit = h.rich(220, false);
            scintilla::line_numbers(self.edit, self.lines);
            set_text(self.edit, &self.initial);
            self.initial.clear();
            self.initial.shrink_to_fit();
        }
        Ok(())
    }
    pub fn layout(&mut self, h: &mut Host) {
        let (w, height) = dimensions(h.hwnd);
        let left = h.px(5.);
        unsafe {
            move_to(
                self.edit,
                rect(left, h.px(26.), w - left - h.px(5.), height - h.px(31.)),
            );
        }
    }
    pub fn command(&mut self, h: &mut Host, id: u16, code: u16) {
        unsafe {
            if id == 220 && code == EN_CHANGE as u16 {
                InvalidateRect(h.hwnd, &rect(0, 0, h.px(32.), dimensions(h.hwnd).1), 0);
                return;
            }
            match id {
                LINES => {
                    self.lines = !self.lines;
                    scintilla::line_numbers(self.edit, self.lines);
                    self.layout(h);
                    h.invalidate();
                }
                COPY => {
                    let (_, s) = selected(self.edit);
                    let _ = platform::copy_text(&s);
                }
                CUT => {
                    SendMessageW(self.edit, WM_CUT, 0, 0);
                }
                PASTE => {
                    SendMessageW(self.edit, WM_PASTE, 0, 0);
                }
                UNDO => {
                    SendMessageW(self.edit, EM_UNDO, 0, 0);
                }
                REDO => {
                    SendMessageW(self.edit, win::REDO, 0, 0);
                }
                ZOOM_IN | ZOOM_OUT | ZOOM_RESET => {
                    h.set_font_size(if id == ZOOM_RESET {
                        14.
                    } else {
                        (h.font_size + if id == ZOOM_IN { 1. } else { -1. }).clamp(8., 48.)
                    });
                    edit_style(self.edit, h.theme, h.font_size, h.dpi);
                    h.invalidate();
                }
                SEARCH => {
                    let _ = create(
                        "便签 · 查找 / 替换",
                        (440., 200.),
                        Content::Find(Find::new(self.edit)),
                        h.proxy.clone(),
                        &h.palette,
                        h.dark,
                        h.font_size,
                    );
                }
                _ => {}
            }
        }
    }
    pub fn theme(&mut self, h: &mut Host) {
        unsafe {
            edit_style(self.edit, h.theme, h.font_size, h.dpi);
        }
    }
    pub fn popup(&mut self, h: &mut Host) {
        unsafe {
            let id = menu(
                h.hwnd,
                &[
                    (COPY, "复制"),
                    (CUT, "剪切"),
                    (PASTE, "粘贴"),
                    (UNDO, "撤销"),
                    (REDO, "重做"),
                    (SEARCH, "查找 / 替换"),
                    (LINES, "显示 / 隐藏行号"),
                ],
            );
            self.command(h, id, 0);
        }
    }
}

pub struct Preview {
    pub target: usize,
    pub range: Range,
    pub before: String,
    pub after: String,
    pub left: HWND,
    pub right: HWND,
}
impl Preview {
    pub fn init(&mut self, h: &mut Host) -> Result<(), String> {
        unsafe {
            h.title_buttons(false, false);
            self.left = h.rich(230, true);
            self.right = h.rich(231, true);
            set_text(self.left, &self.before);
            set_text(self.right, &self.after);
            h.button(240, None, "应用");
            h.button(241, None, "取消");
        }
        Ok(())
    }
    pub fn layout(&mut self, h: &mut Host) {
        let (w, height) = dimensions(h.hwnd);
        unsafe {
            move_to(
                self.left,
                rect(h.px(8.), h.px(46.), w / 2 - h.px(12.), height - h.px(93.)),
            );
            move_to(
                self.right,
                rect(
                    w / 2 + h.px(4.),
                    h.px(46.),
                    w / 2 - h.px(12.),
                    height - h.px(93.),
                ),
            );
            move_to(
                GetDlgItem(h.hwnd, 240),
                rect(w - h.px(180.), height - h.px(38.), h.px(78.), h.px(28.)),
            );
            move_to(
                GetDlgItem(h.hwnd, 241),
                rect(w - h.px(92.), height - h.px(38.), h.px(78.), h.px(28.)),
            );
        }
    }
    pub fn command(&mut self, h: &mut Host, id: u16) {
        if id == 240 {
            if let Some(p) = &h.proxy {
                p.send(Notice::Apply(
                    self.target,
                    self.range,
                    self.before.clone(),
                    self.after.clone(),
                ));
            }
        }
        if id == 240 || id == 241 {
            unsafe {
                PostMessageW(h.hwnd, WM_CLOSE, 0, 0);
            }
        }
    }
    pub fn theme(&mut self, h: &mut Host) {
        unsafe {
            edit_style(self.left, h.theme, h.font_size, h.dpi);
            edit_style(self.right, h.theme, h.font_size, h.dpi);
        }
    }
    pub unsafe fn paint(&self, h: &Host, dc: HDC) {
        let (w, _) = dimensions(h.hwnd);
        label(
            dc,
            &rect(h.px(9.), h.px(24.), w / 2, h.px(20.)),
            "处理前",
            h.theme.muted,
            h.font.0,
            DT_SINGLELINE | DT_VCENTER,
        );
        label(
            dc,
            &rect(w / 2 + h.px(5.), h.px(24.), w / 2, h.px(20.)),
            "处理后",
            h.theme.muted,
            h.font.0,
            DT_SINGLELINE | DT_VCENTER,
        );
    }
}

pub struct Find {
    target: HWND,
    needle: HWND,
    replacement: HWND,
}
impl Find {
    pub fn new(target: HWND) -> Self {
        Self {
            target,
            needle: null_mut(),
            replacement: null_mut(),
        }
    }
    pub fn init(&mut self, h: &mut Host) -> Result<(), String> {
        unsafe {
            h.title_buttons(false, false);
            h.control("STATIC", "查找", 0, 250);
            h.control("STATIC", "替换为", 0, 251);
            self.needle = h.control(
                "EDIT",
                "",
                WS_TABSTOP | WS_BORDER | ES_AUTOHSCROLL as u32,
                252,
            );
            self.replacement = h.control(
                "EDIT",
                "",
                WS_TABSTOP | WS_BORDER | ES_AUTOHSCROLL as u32,
                253,
            );
            h.button(254, None, "查找下一个");
            h.button(255, None, "替换当前");
            h.button(256, None, "全部替换");
            SetFocus(self.needle);
        }
        Ok(())
    }
    pub fn layout(&mut self, h: &mut Host) {
        let (w, _) = dimensions(h.hwnd);
        let width = w as f32 * 96. / h.dpi as f32;
        unsafe {
            h.place(250, 10., 42., 65., 24.);
            h.place(251, 10., 78., 65., 24.);
            h.place(252, 80., 40., width - 95., 26.);
            h.place(253, 80., 76., width - 95., 26.);
            h.place(254, 10., 125., 120., 28.);
            h.place(255, 140., 125., 110., 28.);
            h.place(256, 260., 125., 110., 28.);
        }
    }
    pub fn command(&mut self, h: &mut Host, id: u16) {
        unsafe {
            if IsWindow(self.target) == 0 {
                PostMessageW(h.hwnd, WM_CLOSE, 0, 0);
                return;
            }
            let needle = text(self.needle);
            if needle.is_empty() {
                return;
            }
            let replacement = text(self.replacement);
            if id == 256 {
                let body = text(self.target);
                let after = body.replace(&needle, &replacement);
                replace(self.target, Range { start: 0, end: -1 }, &after);
                return;
            }
            if id == 255 {
                let (r, s) = selected(self.target);
                if s == needle {
                    replace(self.target, r, &replacement);
                }
            }
            if id == 254 || id == 255 {
                let mut r = Range::default();
                SendMessageW(self.target, EXGETSEL, 0, (&mut r as *mut Range) as isize);
                let body = text(self.target);
                let start = body
                    .char_indices()
                    .scan(0usize, |units, (i, c)| {
                        let before = *units;
                        *units += c.len_utf16();
                        Some((i, before))
                    })
                    .find(|&(_, units)| units >= r.end.max(0) as usize)
                    .map_or(body.len(), |(i, _)| i);
                let found = body[start..]
                    .find(&needle)
                    .map(|i| start + i)
                    .or_else(|| body[..start].find(&needle));
                if let Some(index) = found {
                    let cp = body[..index].encode_utf16().count() as i32;
                    let range = Range {
                        start: cp,
                        end: cp + needle.encode_utf16().count() as i32,
                    };
                    SendMessageW(self.target, EXSETSEL, 0, (&range as *const Range) as isize);
                    SendMessageW(self.target, EM_SCROLLCARET, 0, 0);
                } else {
                    win::error(h.hwnd, "没有找到该文字。");
                }
            }
        }
    }
}

pub struct Tables {
    tables: Vec<Table>,
    index: usize,
    view: Option<table_layout::TableView>,
}
impl Tables {
    pub fn new(tables: Vec<Table>) -> Self {
        Self {
            tables,
            index: 0,
            view: None,
        }
    }
    pub fn init(&mut self, h: &mut Host) -> Result<(), String> {
        unsafe {
            h.title_buttons(false, false);
            h.button(260, None, "复制整表");
            h.button(261, None, "复制选中内容");
            let combo = h.control(
                "COMBOBOX",
                "",
                CBS_DROPDOWNLIST as u32 | WS_VSCROLL | WS_TABSTOP,
                262,
            );
            for i in 0..self.tables.len() {
                SendMessageW(
                    combo,
                    CB_ADDSTRING,
                    0,
                    platform::wide(&format!("表格 {}", i + 1)).as_ptr() as isize,
                );
            }
            SendMessageW(combo, CB_SETCURSEL, 0, 0);
        }
        self.view = Some(table_layout::TableView::create(h));
        Ok(())
    }
    pub fn layout(&mut self, h: &mut Host) {
        let (w, height) = dimensions(h.hwnd);
        unsafe {
            h.place(260, 8., 29., 100., 28.);
            h.place(261, 116., 29., 110., 28.);
            h.place(262, 238., 30., 130., 180.);
        }
        if let (Some(view), Some(table)) = (&mut self.view, self.tables.get(self.index)) {
            view.layout(
                h,
                table,
                rect(
                    h.px(8.),
                    h.px(66.),
                    (w - h.px(16.)).max(1),
                    (height - h.px(74.)).max(1),
                ),
            );
        }
    }
    pub fn command(&mut self, h: &mut Host, id: u16, code: u16) {
        if id == 262 && code == CBN_SELCHANGE as u16 {
            self.index = unsafe {
                SendMessageW(GetDlgItem(h.hwnd, 262), CB_GETCURSEL, 0, 0).max(0) as usize
            };
            self.layout(h);
            h.invalidate();
            return;
        }
        let result = if id == 260 {
            self.tables
                .get(self.index)
                .ok_or("没有表格结果。".into())
                .and_then(table_layout::copy_table)
        } else if id == 261 || id == table_layout::COPY_SELECTION {
            self.view
                .as_ref()
                .ok_or("没有表格结果。".into())
                .and_then(|v| v.copy_selection())
        } else {
            return;
        };
        if let Err(e) = result {
            unsafe {
                MessageBoxW(
                    h.hwnd,
                    platform::wide(&e).as_ptr(),
                    platform::wide("表格").as_ptr(),
                    MB_OK | MB_ICONWARNING,
                );
            }
        }
    }
    pub fn theme(&mut self, h: &mut Host) {
        self.layout(h);
    }
}
