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
