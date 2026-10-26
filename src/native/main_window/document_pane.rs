//! Inline result preview. OCR source, rendered content and translation keep
//! separate controls so switching views never rewrites source or translation.
use super::*;

impl Main {
    /// Open local text for visual review without OCR requests or clipboard writes.
    pub(super) fn load_document_argument(&mut self, h: &mut Host) -> Result<(), String> {
        let args: Vec<_> = std::env::args_os().collect();
        let Some(index) = args.iter().position(|s| s == "--preview-document") else {
            return Ok(());
        };
        let path = args.get(index + 1).ok_or("缺少文档预览路径。")?;
        if std::fs::metadata(path)
            .map_err(|_| "无法读取预览文件。")?
            .len()
            > 256 * 1024
        {
            return Err("预览文件超过 256 KiB。".into());
        }
        let source = std::fs::read_to_string(path).map_err(|_| "预览文件必须是 UTF-8 文本。")?;
        unsafe {
            set_text(self.source, &source);
        }
        self.original = source;
        self.open_rendered(h, self.source, false);
        Ok(())
    }
    pub(super) fn open_rendered(&mut self, h: &mut Host, editor: HWND, markdown: bool) {
        self.table_active = false;
        let (_, body) = unsafe { selected(editor) };
        if body.trim().is_empty() {
            self.set_status(h, "请先输入文字和公式，再打开预览。");
            return;
        }
        if self.document_view.is_none() {
            match rendered::DocumentView::create(h) {
                Ok(view) => self.document_view = Some(view),
                Err(error) => {
                    self.set_status(h, error);
                    return;
                }
            }
        }
        self.rendered_body = body;
        self.rendered_origin = editor;
        self.rendered_markdown = markdown;
        let result = self
            .document_view
            .as_ref()
            .unwrap()
            .rebuild(h, &self.rendered_body, markdown);
        match result {
            Ok(unsupported) => {
                self.rendered_active = true;
                if !self.settings.show_translation {
                    self.toggle_translation(h, true);
                }
                self.layout(h);
                self.set_status(
                    h,
                    if unsupported == 0 {
                        "已切换到右栏文档预览 · 复制保留公式源码；点击翻译返回译文。".into()
                    } else {
                        format!("右栏预览有 {unsupported} 处公式未完整转换，保留源码；请检查显示。")
                    },
                );
            }
            Err(error) => {
                self.close_document(h);
                self.set_status(h, error);
            }
        }
    }

    pub(in crate::native) fn close_document(&mut self, h: &mut Host) {
        self.table_active = false;
        self.rendered_active = false;
        self.rendered_body.clear();
        self.rendered_origin = null_mut();
        if let Some(view) = &self.document_view {
            view.hide_and_clear();
        }
        unsafe {
            KillTimer(h.hwnd, 5);
        }
        self.layout(h);
    }

    pub(super) fn refresh_document(&mut self, h: &mut Host) {
        if !self.rendered_active {
            return;
        }
        self.rendered_body = unsafe { text(self.rendered_origin) };
        if let Some(view) = &self.document_view {
            match view.rebuild(h, &self.rendered_body, self.rendered_markdown) {
                Ok(0) => {}
                Ok(n) => {
                    self.set_status(h, format!("文档预览有 {n} 处公式未完整转换，请检查源码。"))
                }
                Err(error) => {
                    self.close_document(h);
                    self.set_status(h, error);
                }
            }
        }
    }

    pub(super) fn layout_document(&self, h: &Host, area: RECT) {
        unsafe {
            if let Some(view) = &self.document_view {
                move_to(view.editor, area);
                ShowWindow(
                    view.editor,
                    if self.rendered_active && self.settings.show_translation {
                        SW_SHOW
                    } else {
                        SW_HIDE
                    },
                );
            }
            if let Some(button) = h.buttons.iter().find(|b| b.id == DOCUMENT_VIEW) {
                InvalidateRect(button.hwnd, null(), 0);
            }
        }
    }
}
