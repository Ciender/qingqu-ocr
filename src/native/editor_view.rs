//! Text apartment and logical-line metadata for native editors.
use super::win::*;
use std::ptr::null_mut;
use windows::{
    core::{IUnknown, Interface},
    Win32::{
        System::Ole::{OleInitialize, OleUninitialize},
        UI::Controls::RichEdit::{tomParagraph, ITextDocument, EM_GETOLEINTERFACE},
    },
};

// Keep OLE alive for the lifetime of the UI thread, including ordinary editors.
pub struct Apartment;
impl Apartment {
    pub fn new() -> Result<Self, String> {
        unsafe {
            OleInitialize(None).map_err(|_| "无法初始化系统文本组件。")?;
        }
        Ok(Self)
    }
}
impl Drop for Apartment {
    fn drop(&mut self) {
        unsafe {
            OleUninitialize();
        }
    }
}

unsafe fn document(editor: HWND) -> Option<ITextDocument> {
    let mut raw: *mut std::ffi::c_void = null_mut();
    if SendMessageW(editor, EM_GETOLEINTERFACE, 0, (&mut raw as *mut _) as isize) == 0
        || raw.is_null()
    {
        return None;
    }
    IUnknown::from_raw(raw).cast().ok()
}

pub unsafe fn line_count(editor: HWND) -> usize {
    if super::scintilla::is_editor(editor) {
        return super::scintilla::line_count(editor);
    }
    document(editor)
        .and_then(|doc| doc.Range(i32::MAX, i32::MAX).ok())
        .and_then(|range| range.GetIndex(tomParagraph.0).ok())
        .map(|count| count.max(1) as usize)
        .unwrap_or(1)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::platform::wide;
    use std::ptr::null;

    struct Editor {
        parent: HWND,
        hwnd: HWND,
        _apartment: Apartment,
    }
    impl Editor {
        unsafe fn new() -> Self {
            let apartment = Apartment::new().unwrap();
            super::super::scintilla::register().unwrap();
            let parent = CreateWindowExW(
                0,
                wide("STATIC").as_ptr(),
                wide("").as_ptr(),
                WS_POPUP,
                -2000,
                -2000,
                320,
                240,
                null_mut(),
                null_mut(),
                GetModuleHandleW(null()),
                null(),
            );
            assert!(!parent.is_null());
            let editor = super::super::scintilla::create(parent, 220, false);
            super::super::scintilla::line_numbers(editor, true);
            MoveWindow(editor, 0, 0, 200, 160, 0);
            Self {
                parent,
                hwnd: editor,
                _apartment: apartment,
            }
        }
    }
    impl Drop for Editor {
        fn drop(&mut self) {
            unsafe {
                DestroyWindow(self.parent);
            }
        }
    }

    #[test]
    fn ime_cancel_removes_tentative_pinyin_without_entering_session_history() {
        use super::super::{history, scintilla::api::*};
        unsafe {
            let owned = Editor::new();
            let editor = owned.hwnd;
            let body = "已有正文🙂";
            set_text(editor, body);
            history::install(editor).unwrap();
            SendMessageW(editor, EM_SETSEL, usize::MAX, -1);
            for pinyin in ["n", "ni", "nih", "niha", "nihao"] {
                SendMessageW(editor, WM_IME_STARTCOMPOSITION, 0, 0);
                // The same tentative marker and insertion used by Scintilla's
                // inline IME; END runs its real TentativeUndo without an IME
                // service or the user's active keyboard/desktop.
                let start = SendMessageW(editor, SCI_GETUNDOCURRENT, 0, 0);
                SendMessageW(editor, SCI_SETUNDOTENTATIVE, start as usize, 0);
                for unit in pinyin.encode_utf16() {
                    SendMessageW(editor, WM_CHAR, unit as usize, 0);
                }
                assert_eq!(text(editor), format!("{body}{pinyin}"));
                SendMessageW(editor, WM_IME_ENDCOMPOSITION, 0, 0);
                assert_eq!(text(editor), body, "cancel must remove preedit {pinyin}");
                assert_eq!(SendMessageW(editor, EM_CANUNDO, 0, 0), 0);
                assert_eq!(SendMessageW(editor, SCI_GETUNDOCOLLECTION, 0, 0), 0);
                assert_eq!(SendMessageW(editor, SCI_GETUNDOACTIONS, 0, 0), 0);
            }
        }
    }

    #[test]
    fn ime_commit_is_one_session_undo_step_and_never_restores_pinyin() {
        use super::super::{history, scintilla::api::*};
        unsafe {
            let owned = Editor::new();
            let editor = owned.hwnd;
            let body = "正文🙂：";
            set_text(editor, body);
            history::install(editor).unwrap();
            SendMessageW(editor, EM_SETSEL, usize::MAX, -1);
            SendMessageW(editor, WM_IME_STARTCOMPOSITION, 0, 0);
            for pinyin in ["n", "ni", "nih", "nihao"] {
                let start = SendMessageW(editor, SCI_GETUNDOCURRENT, 0, 0);
                SendMessageW(editor, SCI_SETUNDOTENTATIVE, start as usize, 0);
                for unit in pinyin.encode_utf16() {
                    SendMessageW(editor, WM_CHAR, unit as usize, 0);
                }
                assert_eq!(text(editor), format!("{body}{pinyin}"));
                // Replace the current preedit, as HandleCompositionInline does.
                SendMessageW(editor, SCI_UNDO, 0, 0);
                assert_eq!(text(editor), body);
                SendMessageW(editor, SCI_SETUNDOTENTATIVE, usize::MAX, 0);
            }
            for unit in "你好𠮷".encode_utf16() {
                SendMessageW(editor, WM_CHAR, unit as usize, 0);
            }
            SendMessageW(editor, WM_IME_ENDCOMPOSITION, 0, 0);
            assert_eq!(text(editor), format!("{body}你好𠮷"));
            assert_eq!(SendMessageW(editor, SCI_GETUNDOCOLLECTION, 0, 0), 0);
            assert_eq!(SendMessageW(editor, SCI_GETUNDOACTIONS, 0, 0), 0);
            SendMessageW(editor, EM_UNDO, 0, 0);
            assert_eq!(text(editor), body);
            assert_eq!(SendMessageW(editor, EM_CANUNDO, 0, 0), 0);
            SendMessageW(editor, REDO, 0, 0);
            assert_eq!(text(editor), format!("{body}你好𠮷"));
        }
    }
}
