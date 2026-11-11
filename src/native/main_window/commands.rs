//! Main toolbar, editor commands and persistent text processing modes.
use super::*;

impl Main {
    pub(super) fn sync_modes(&self, h: &mut Host) {
        let modes = &self.settings.processing;
        let (icon, name) = match modes.layout.as_str() {
            "split" => (Icon::Split, "段落拆分"),
            "merge" => (Icon::Merge, "段落合并"),
            _ => (Icon::AutoParagraph, "自动分段"),
        };
        unsafe {
            h.button_tip(PARAGRAPH, icon, &format!("{name} · 点击选择分段模式"));
            h.button_tip(
                FORMAT,
                Icon::Layout,
                "智能排版 · 外部粘贴合成一行且不缩进；其他文字按排版设置处理",
            );
            let chinese = modes.punctuation == "chinese";
            h.button_tip(
                PUNCTUATION,
                if chinese {
                    Icon::ChinesePunctuation
                } else {
                    Icon::EnglishPunctuation
                },
                if chinese {
                    "中文标点 · 点击切换为英文标点"
                } else {
                    "英文标点 · 点击切换为中文标点"
                },
            );
            h.button_tip(
                CHINESE,
                match modes.chinese.as_str() {
                    "traditional" => Icon::ToTraditional,
                    "simplified" => Icon::ToSimplified,
                    _ => Icon::Chinese,
                },
                match modes.chinese.as_str() {
                    "traditional" => "转繁体已开启 · 点击切换为转简体",
                    "simplified" => "转简体已开启 · 点击关闭转换",
                    _ => "简繁转换已关闭 · 点击开启转繁体",
                },
            );
        }
        for button in &mut h.buttons {
            if matches!(button.id, PARAGRAPH | FORMAT | PUNCTUATION) {
                button.active = false;
            } else if button.id == CHINESE {
                button.active = !modes.chinese.is_empty();
            }
            unsafe {
                InvalidateRect(button.hwnd, null(), 0);
            }
        }
    }

    pub fn command(&mut self, h: &mut Host, id: u16, code: u16, from: HWND) {
        unsafe {
            let from_preview = self.rendered_active
                && self
                    .document_view
                    .as_ref()
                    .is_some_and(|view| view.editor == from);
            if from_preview && matches!(id, UNDO | REDO | CUT | PASTE | CLEAR) {
                self.set_status(h, "请在左栏编辑识别源码；右栏显示预览。");
                return;
            }
            if id == SOURCE || id == TRANSLATION {
                if code == EN_SETFOCUS as u16 {
                    self.focused = from;
                }
                if code == EN_CHANGE as u16 {
                    if self.rendered_active && self.rendered_origin == from {
                        SetTimer(h.hwnd, 5, 180, None);
                    }
                    self.cancel_editor_job(h, from);
                    self.invalidate_editor_view(h, from, true);
                }
                return;
            }
            if code == CBN_SELCHANGE as u16 {
                let index = SendMessageW(from, CB_GETCURSEL, 0, 0).max(0) as usize;
                match id {
                    TABLE_INDEX => self.table_view.select_table(index),
                    LANGUAGE => {
                        if let Some((s, _)) = self
                            .settings
                            .ocr_languages(&self.settings.text_ocr_mode)
                            .get(index)
                        {
                            self.settings.ocr_language = (*s).into();
                            if let Err(error) = config::save_preferences(&self.settings) {
                                self.set_status(h, error);
                            }
                        }
                    }
                    TARGET_LANGUAGE => self.select_translation_target(h, index),
                    _ => {}
                }
                self.layout(h);
                return;
            }
            if id == CALL_SETTINGS {
                self.open_settings(h);
                let dialog = self.settings_hwnd as HWND;
                let sidebar = GetDlgItem(dialog, 500);
                SendMessageW(sidebar, LB_SETCURSEL, 5, 0);
                SendMessageW(
                    dialog,
                    WM_COMMAND,
                    500 | ((LBN_SELCHANGE as usize) << 16),
                    sidebar as isize,
                );
                return;
            }
            let editor = if from == self.translated
                || matches!(
                    id,
                    R_COPY | R_CLEAR | R_UNDO | R_REDO | R_SEARCH | R_PASTE | R_CUT
                ) {
                self.translated
            } else if from == self.source
                || (SOURCE_TOOLS.contains(&id) && from == GetDlgItem(h.hwnd, id as i32))
            {
                self.source
            } else if matches!(
                id,
                SEARCH
                    | COPY
                    | CLEAR
                    | UNDO
                    | REDO
                    | PASTE
                    | CUT
                    | LATEX_PREVIEW
                    | MARKDOWN_PREVIEW
            ) {
                self.focused
            } else {
                self.source
            };
            // Commands run while Main is borrowed, so their synchronous
            // EN_CHANGE notifications cannot re-enter it. End the task (and
            // its streaming undo group) before applying an editor command.
            if matches!(
                id,
                PASTE | R_PASTE | CUT | R_CUT | CLEAR | R_CLEAR | UNDO | R_UNDO | REDO | R_REDO
            ) {
                self.cancel_editor_job(h, editor);
            }
            match id {
                SETTINGS => self.open_settings(h),
                CAPTURE => self.action(h, "capture"),
                WINDOW => self.action(h, "window"),
                TABLE_CAPTURE => self.action(h, "table"),
                VERTICAL_CAPTURE => self.action(h, "vertical"),
                FORMULA_CAPTURE => self.action(h, "formula"),
                CLIPBOARD => self.action(h, "clipboard"),
                OPEN => {
                    if let Some(path) = platform::open_image(h.hwnd as usize) {
                        self.load(h, path);
                    }
                }
                LATEX_PREVIEW | MARKDOWN_PREVIEW => {
                    self.open_rendered(h, editor, id == MARKDOWN_PREVIEW)
                }
                DOCUMENT_VIEW => {
                    if self.rendered_active || self.table_active {
                        self.close_document(h);
                        self.set_status(h, "已切换到译文视图；识别源码保持在左栏。");
                    } else if !self.tables.is_empty() {
                        self.table_active = true;
                        if !self.settings.show_translation {
                            self.toggle_translation(h, true);
                        }
                        self.layout(h);
                        h.invalidate();
                    } else {
                        self.open_rendered(h, self.source, false);
                    }
                }
                TOGGLE => {
                    if self.settings.show_translation {
                        self.toggle_translation(h, false);
                    } else {
                        self.command(h, TRANSLATE, 0, null_mut());
                    }
                }
                AUTO_TRANSLATE => {
                    self.settings.auto_translate =
                        SendMessageW(GetDlgItem(h.hwnd, AUTO_TRANSLATE as i32), BM_GETCHECK, 0, 0)
                            == BST_CHECKED as isize;
                    if self.automatic_translation_enabled() {
                        self.command(h, TRANSLATE, 0, null_mut());
                    }
                }
                TRANSLATE => {
                    let (_, body) = selected(self.source);
                    self.close_document(h);
                    if !self.settings.show_translation {
                        self.settings.auto_translate = true;
                        self.toggle_translation(h, true);
                    }
                    if body.trim().is_empty() {
                        self.set_status(h, "请先输入原文。");
                        return;
                    }
                    self.begin(
                        h,
                        Work::Translate(
                            body,
                            Box::new(self.settings.translation.clone()),
                            Box::new(self.keys.clone()),
                        ),
                        "翻译",
                        Purpose::Translate,
                        Vec::new(),
                    );
                }
                table_layout::COPY_SELECTION => {
                    let result = self.table_view.copy(&self.tables, true);
                    self.set_status(
                        h,
                        result
                            .map(|_| "已复制表格选区（Word / Excel）".into())
                            .unwrap_or_else(|e| e),
                    );
                }
                COPY | R_COPY => {
                    if self.table_active && id == R_COPY {
                        let result = self.table_view.copy(&self.tables, false);
                        self.set_status(
                            h,
                            result
                                .map(|_| "已复制整表（Word / Excel）".into())
                                .unwrap_or_else(|e| e),
                        );
                        return;
                    }
                    if self.rendered_active
                        && (id == R_COPY
                            || self
                                .document_view
                                .as_ref()
                                .is_some_and(|v| v.editor == from))
                    {
                        let result = platform::copy_text(&self.rendered_body);
                        self.set_status(
                            h,
                            result
                                .map(|_| "已复制文档源码（保留公式结构）".into())
                                .unwrap_or_else(|e| e),
                        );
                        return;
                    }
                    let (_, body) = selected(editor);
                    let result = platform::copy_text(&body);
                    self.set_status(h, result.map(|_| "已复制".into()).unwrap_or_else(|e| e));
                }
                PASTE | R_PASTE => {
                    SendMessageW(editor, WM_PASTE, 0, 0);
                }
                CUT | R_CUT => {
                    SendMessageW(editor, WM_CUT, 0, 0);
                }
                CLEAR | R_CLEAR => replace(editor, Range { start: 0, end: -1 }, ""),
                UNDO | R_UNDO => {
                    SendMessageW(editor, EM_UNDO, 0, 0);
                }
                REDO | R_REDO => {
                    SendMessageW(editor, win::REDO, 0, 0);
                }
                CANCEL => {
                    self.cancel(h);
                    self.set_status(h, "已取消；迟到结果不会覆盖编辑内容。");
                }
                SEARCH | R_SEARCH => {
                    if let Ok(window) = create(
                        "查找 / 替换",
                        (440., 200.),
                        Content::Find(Find::new(editor)),
                        h.proxy.clone(),
                        &h.palette,
                        h.dark,
                        h.font_size,
                    ) {
                        self.children.push(window as usize);
                    }
                }
                ZOOM_IN | ZOOM_OUT | ZOOM_RESET => {
                    h.set_font_size(if id == ZOOM_RESET {
                        14.
                    } else {
                        (h.font_size + if id == ZOOM_IN { 1. } else { -1. }).clamp(8., 48.)
                    });
                    self.settings.editor_font_size = h.font_size;
                    for &edit in &h.editors {
                        edit_style(edit, h.theme, h.font_size, h.dpi);
                    }
                    self.refresh_document(h);
                    h.invalidate();
                }
                PARAGRAPH => {
                    match paragraph_menu::choose(h, &self.settings.processing.layout) {
                        Ok(Some(choice)) => {
                            self.command(h, choice, 0, null_mut());
                            SetFocus(self.source);
                        }
                        Ok(None) => {}
                        Err(error) => self.set_status(h, error),
                    }
                    // A native menu owns a nested message loop; drain any
                    // worker notices that arrived while Main was borrowed.
                    PostMessageW(h.hwnd, WAKE, 0, 0);
                }
                FORMAT
                | AUTO_PARAGRAPH
                | SPLIT
                | MERGE
                | PUNCTUATION
                | CHINESE
                | RESTORE
                | 400..=408 => {
                    if matches!(id, AUTO_PARAGRAPH | SPLIT | MERGE | PUNCTUATION | CHINESE) {
                        let active = match id {
                            AUTO_PARAGRAPH | SPLIT | MERGE => {
                                let mode = match id {
                                    AUTO_PARAGRAPH => "smart",
                                    SPLIT => "split",
                                    _ => "merge",
                                };
                                self.settings.processing.layout = mode.into();
                                true
                            }
                            PUNCTUATION => {
                                self.settings.processing.punctuation =
                                    match self.settings.processing.punctuation.as_str() {
                                        "chinese" => "english",
                                        "english" => "chinese",
                                        _ => "chinese",
                                    }
                                    .into();
                                !self.settings.processing.punctuation.is_empty()
                            }
                            _ => {
                                self.settings.processing.chinese =
                                    match self.settings.processing.chinese.as_str() {
                                        "traditional" => "simplified",
                                        "simplified" => "",
                                        _ => "traditional",
                                    }
                                    .into();
                                !self.settings.processing.chinese.is_empty()
                            }
                        };
                        self.sync_modes(h);
                        self.set_status(
                            h,
                            if active {
                                match id {
                                    AUTO_PARAGRAPH | SPLIT | MERGE => match self.settings.processing.layout.as_str() {
                                        "split" => "已切换到段落拆分模式：当前原文已应用，之后 OCR 自动沿用。",
                                        "merge" => "已切换到段落合并模式：当前原文已应用，之后 OCR 自动沿用。",
                                        _ => "已切换到自动分段模式：当前原文已应用，之后 OCR 自动沿用。",
                                    },
                                    PUNCTUATION => match self.settings.processing.punctuation.as_str() {
                                        "chinese" => "已切换到中文标点模式：当前原文已应用，之后 OCR 自动沿用。",
                                        "english" => "已切换到英文标点模式：当前原文已应用，之后 OCR 自动沿用。",
                                        _ => "标点转换已关闭：之后 OCR 不再转换标点。",
                                    },
                                    CHINESE => match self.settings.processing.chinese.as_str() {
                                        "traditional" => "已切换到繁体模式：当前原文已应用，之后 OCR 自动沿用。",
                                        "simplified" => "已切换到简体模式：当前原文已应用，之后 OCR 自动沿用。",
                                        _ => "简繁转换已关闭：之后 OCR 不再转换简繁体。",
                                    },
                                    _ => "模式已切换：当前原文已应用，之后 OCR 自动沿用。",
                                }
                            } else {
                                match id {
                                    PUNCTUATION => "标点转换已关闭：之后 OCR 不再转换标点。",
                                    CHINESE => "简繁转换已关闭：之后 OCR 不再转换简繁体。",
                                    _ => "模式已切换：当前布局保持不变。",
                                }
                            },
                        );
                        if let Err(e) = config::save_preferences(&self.settings) {
                            self.set_status(h, e);
                        }
                        if !active && matches!(id, PUNCTUATION | CHINESE) {
                            return;
                        }
                    }
                    let (range, before) =
                        if matches!(id, AUTO_PARAGRAPH | SPLIT | MERGE | PUNCTUATION) {
                            (Range { start: 0, end: -1 }, text(self.source))
                        } else {
                            selected(self.source)
                        };
                    if before.is_empty() {
                        return;
                    }
                    let pasted = scintilla::pasted_document(self.source);
                    let pasted_base = if pasted
                        && matches!(id, FORMAT | AUTO_PARAGRAPH | SPLIT | MERGE | PUNCTUATION)
                    {
                        match history::paragraph_source(self.source) {
                            Ok(source) => Some(source.unwrap_or_else(|| text(self.source))),
                            Err(error) => {
                                self.set_status(h, error);
                                return;
                            }
                        }
                    } else {
                        None
                    };
                    if id == CHINESE {
                        self.begin(
                            h,
                            Work::Convert(
                                before.clone(),
                                self.settings.processing.chinese == "traditional",
                            ),
                            if self.settings.processing.chinese == "traditional" {
                                "转换为繁体"
                            } else {
                                "转换为简体"
                            },
                            Purpose::Convert {
                                target: self.source as usize,
                                range,
                                before,
                            },
                            Vec::new(),
                        );
                        return;
                    }
                    let after = match id {
                        FORMAT => {
                            if pasted {
                                text::format_pasted_text(&before, &self.settings.format_options)
                            } else {
                                let positioned = if range.start == 0
                                    && before == text(self.source)
                                    && (before == self.original || before == self.processing_result)
                                {
                                    self.layout.as_deref()
                                } else {
                                    None
                                };
                                text::intelligent_format_ocr(
                                    &before,
                                    positioned,
                                    &self.settings.processing,
                                    &self.settings.format_options,
                                    self.settings.first_line_indent,
                                )
                            }
                        }
                        AUTO_PARAGRAPH | SPLIT | MERGE => {
                            let entire = range.start == 0 && before == text(self.source);
                            let base = if let Some(source) = &pasted_base {
                                source.clone()
                            } else if entire && before == self.processing_result {
                                self.processing_base.clone()
                            } else {
                                before.clone()
                            };
                            let positioned = if !pasted && entire && base == self.original {
                                self.layout.as_deref()
                            } else {
                                None
                            };
                            let result = if pasted {
                                self.settings.processing.apply_pasted(
                                    &base,
                                    &self.settings.format_options,
                                    self.settings.first_line_indent,
                                )
                            } else {
                                self.settings.processing.apply(
                                    &base,
                                    positioned,
                                    &self.settings.format_options,
                                    self.settings.first_line_indent,
                                )
                            };
                            let after = match result {
                                Ok(after) => after,
                                Err(error) => {
                                    self.set_status(h, error);
                                    return;
                                }
                            };
                            if entire && !pasted {
                                self.processing_base = base;
                                self.processing_result = after.clone();
                            }
                            after
                        }
                        PUNCTUATION => text::punctuation(
                            &before,
                            self.settings.processing.punctuation == "chinese",
                        ),
                        RESTORE => self.original.clone(),
                        400 => text::merge(&before, true),
                        401 => text::tidy(&before),
                        402 => text::punctuation(&before, false),
                        403 => text::indent_paragraphs(
                            &before,
                            self.settings.first_line_indent.min(16),
                        ),
                        404 => text::wrap(&before, self.settings.wrap_columns.clamp(5, 300)),
                        405 => {
                            if self.settings.split_delimiter.is_empty() {
                                before.clone()
                            } else {
                                before.replace(&self.settings.split_delimiter, "\n\n")
                            }
                        }
                        _ => before.clone(),
                    };
                    let range = if id == RESTORE {
                        Range { start: 0, end: -1 }
                    } else {
                        range
                    };
                    let before = if id == RESTORE {
                        text(self.source)
                    } else {
                        before
                    };
                    if id == FORMAT {
                        self.set_status(
                            h,
                            "智能排版已应用到当前文本；默认分段模式保持不变，可撤销。",
                        );
                    }
                    if matches!(id, FORMAT | AUTO_PARAGRAPH | SPLIT | MERGE | PUNCTUATION) {
                        if id == PUNCTUATION && before == self.processing_result {
                            self.processing_result = after.clone();
                        }
                        if before != after {
                            self.cancel_editor_job(h, self.source);
                            if let Some(base) = &pasted_base {
                                if let Err(error) =
                                    history::begin_paragraph_change(self.source, base)
                                {
                                    self.set_status(h, error);
                                    return;
                                }
                            }
                            replace_preserving_view(self.source, range, &after);
                            if pasted_base.is_some() {
                                history::end_group(self.source);
                            }
                        }
                    } else {
                        self.preview(h, self.source, range, before, after);
                    }
                }
                TABLES => self.open_tables(h),
                NOTE => {
                    let (_, body) = selected(self.source);
                    let _ = self.note(h, body);
                }
                MENU => self.popup(h, self.source),
                R_MENU => {
                    let id = menu(
                        h.hwnd,
                        &[
                            (R_COPY, "复制译文"),
                            (R_CUT, "剪切译文选区"),
                            (R_PASTE, "粘贴到译文"),
                            (R_UNDO, "撤销译文编辑"),
                            (R_REDO, "重做译文编辑"),
                            (R_CLEAR, "清空译文"),
                            (R_SEARCH, "查找 / 替换译文"),
                        ],
                    );
                    if id != 0 {
                        self.command(h, id, 0, self.translated);
                    }
                }
                420 => {
                    self.preview = !self.preview;
                    self.settings.preview_before_apply = self.preview;
                    self.set_status(
                        h,
                        if self.preview {
                            "已开启处理前预览"
                        } else {
                            "处理结果将直接应用"
                        },
                    );
                }
                421 => {
                    self.settings.dark = !self.settings.dark;
                    self.apply_theme(h);
                }
                422 => {
                    self.settings.palette = if self.settings.palette == "catppuccin" {
                        "vscode"
                    } else {
                        "catppuccin"
                    }
                    .into();
                    self.apply_theme(h);
                }
                SHOW_TOOLBAR => self.set_toolbar_visible(h, true),
                HIDE_TOOLBAR => self.set_toolbar_visible(h, false),
                STARTUP => self.toggle_startup(h),
                DESKTOP_SHORTCUT => self.create_shortcut(h, platform::Shortcut::Desktop),
                START_MENU_SHORTCUT => self.create_shortcut(h, platform::Shortcut::StartMenu),
                424 => {
                    PostMessageW(h.hwnd, EXIT, 0, 0);
                }
                _ => {}
            }
            if matches!(
                id,
                PASTE
                    | CUT
                    | CLEAR
                    | UNDO
                    | REDO
                    | FORMAT
                    | AUTO_PARAGRAPH
                    | SPLIT
                    | MERGE
                    | PUNCTUATION
                    | RESTORE
                    | 400..=408
            ) {
                self.refresh_document(h);
            }
        }
    }
    pub fn popup(&mut self, h: &mut Host, target: HWND) {
        if self.table_popup(h) {
            return;
        }
        unsafe {
            if self.rendered_active
                && self
                    .document_view
                    .as_ref()
                    .is_some_and(|view| view.editor == target)
            {
                let id = menu(
                    h.hwnd,
                    &[
                        (COPY, "复制文档源码（保留公式结构）"),
                        (DOCUMENT_VIEW, "返回译文视图"),
                    ],
                );
                if id != 0 {
                    self.command(h, id, 0, target);
                }
                return;
            }
            if target == self.source || target == self.translated {
                self.focused = target;
            }
            let id = menu(
                h.hwnd,
                &[
                    (COPY, "复制"),
                    (CUT, "剪切"),
                    (PASTE, "粘贴"),
                    (UNDO, "撤销"),
                    (REDO, "重做"),
                    (SEARCH, "查找 / 替换"),
                    (LATEX_PREVIEW, "LaTeX 公式预览（选区 / 全文）"),
                    (MARKDOWN_PREVIEW, "Markdown 预览（选区 / 全文）"),
                    (CLEAR, "清空"),
                    (0, ""),
                    (CAPTURE, "区域截图识别"),
                    (TABLE_CAPTURE, "表格识别"),
                    (VERTICAL_CAPTURE, "竖排文字识别"),
                    (FORMULA_CAPTURE, "公式识别"),
                    (CLIPBOARD, "当前剪贴板 · Alt+V"),
                    (OPEN, "打开图片"),
                    (0, ""),
                    (PARAGRAPH, "分段模式…"),
                    (FORMAT, "智能排版（仅本次）"),
                    (PUNCTUATION, "切换中／英标点"),
                    (CHINESE, "简体 / 繁体转换"),
                    (400, "合并全部正文"),
                    (401, "清理空白"),
                    (403, "首行缩进"),
                    (404, "按设置列宽重排"),
                    (405, "按自定义分隔符拆分"),
                    (402, "转英文标点"),
                    (RESTORE, "恢复原始 OCR"),
                    (0, ""),
                    (TOGGLE, "显示 / 隐藏翻译栏"),
                    (TABLES, "表格结果"),
                    (NOTE, "原文打开便签"),
                    (421, "切换浅色 / 深色"),
                    (422, "切换配色系列"),
                    (SHOW_TOOLBAR, "显示工具悬浮窗"),
                    (HIDE_TOOLBAR, "关闭工具悬浮窗"),
                    (STARTUP, "开启 / 关闭开机自启"),
                    (DESKTOP_SHORTCUT, "发送到桌面快捷方式"),
                    (START_MENU_SHORTCUT, "发送到开始菜单"),
                    (SETTINGS, "设置"),
                    (CANCEL, "取消当前任务"),
                    (424, "彻底退出"),
                ],
            );
            if id != 0 {
                self.command(
                    h,
                    id,
                    0,
                    if matches!(id, COPY | CUT | PASTE | UNDO | REDO | SEARCH) {
                        self.focused
                    } else {
                        null_mut()
                    },
                );
            }
            PostMessageW(h.hwnd, WAKE, 0, 0);
        }
    }
}
