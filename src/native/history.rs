//! Session-wide undo/redo, with changed text on disk instead of full snapshots in RAM.
use super::win::*;
use std::{
    cell::RefCell,
    fs::File,
    io::{self, Read, Seek, SeekFrom, Write},
};

const NONE: u64 = u64::MAX;
const HEADER: u64 = 80;
struct Journal {
    file: File,
    cursor: u64,
    redo: u64,
    current: String,
    pasted: bool,
    paragraph_source: u64,
    group: Option<(String, Range, bool, u64)>,
}
impl Journal {
    fn new(current: String) -> io::Result<Self> {
        Ok(Self {
            file: crate::session_file::create("undo")?,
            cursor: NONE,
            redo: NONE,
            current,
            pasted: false,
            paragraph_source: NONE,
            group: None,
        })
    }
    fn record(
        &mut self,
        next: String,
        before: Range,
        after: Range,
        pasted: bool,
    ) -> io::Result<()> {
        let source = if self.group.is_some() {
            self.paragraph_source
        } else {
            NONE
        };
        self.record_with_source(next, before, after, pasted, source)
    }
    fn record_with_source(
        &mut self,
        next: String,
        before: Range,
        after: Range,
        pasted: bool,
        source: u64,
    ) -> io::Result<()> {
        if self.current == next && self.pasted == pasted && self.paragraph_source == source {
            return Ok(());
        }
        if self.group.is_some() {
            self.current = next;
            self.pasted = pasted;
            self.paragraph_source = source;
            return Ok(());
        }
        let a = self.current.as_bytes();
        let b = next.as_bytes();
        let mut start = a.iter().zip(b).take_while(|(a, b)| a == b).count();
        while !self.current.is_char_boundary(start) || !next.is_char_boundary(start) {
            start -= 1;
        }
        let mut suffix = a[start..]
            .iter()
            .rev()
            .zip(b[start..].iter().rev())
            .take_while(|(a, b)| a == b)
            .count();
        while !self.current.is_char_boundary(a.len() - suffix)
            || !next.is_char_boundary(b.len() - suffix)
        {
            suffix -= 1;
        }
        let removed = &a[start..a.len() - suffix];
        let inserted = &b[start..b.len() - suffix];
        let offset = self.file.seek(SeekFrom::End(0))?;
        // Linked metadata also lives on disk; RAM does not grow per undo step.
        for value in [
            self.cursor,
            NONE,
            start as u64,
            removed.len() as u64,
            inserted.len() as u64,
        ] {
            self.file.write_all(&value.to_le_bytes())?;
        }
        for value in [before.start, before.end, after.start, after.end] {
            self.file.write_all(&value.to_le_bytes())?;
        }
        // Restore paste behavior along with text when undo crosses a paste or
        // a fresh OCR result. This costs eight disk bytes per history record.
        self.file
            .write_all(&[self.pasted as u8, pasted as u8, 0, 0, 0, 0, 0, 0])?;
        for source in [self.paragraph_source, source] {
            self.file.write_all(&source.to_le_bytes())?;
        }
        self.file.write_all(removed)?;
        self.file.write_all(inserted)?;
        if self.cursor != NONE {
            self.file.seek(SeekFrom::Start(self.cursor + 8))?;
            self.file.write_all(&offset.to_le_bytes())?;
        }
        self.cursor = offset;
        self.redo = NONE;
        self.current = next;
        self.pasted = pasted;
        self.paragraph_source = source;
        Ok(())
    }
    fn end_group(&mut self, after: Range) -> io::Result<()> {
        if let Some((before, selection, pasted, source)) = self.group.take() {
            let next = std::mem::replace(&mut self.current, before);
            let next_pasted = std::mem::replace(&mut self.pasted, pasted);
            let next_source = std::mem::replace(&mut self.paragraph_source, source);
            self.record_with_source(next, selection, after, next_pasted, next_source)?;
        }
        Ok(())
    }
    fn travel(&mut self, redo: bool) -> io::Result<Option<Range>> {
        let node = if redo { self.redo } else { self.cursor };
        if node == NONE {
            return Ok(None);
        }
        self.file.seek(SeekFrom::Start(node))?;
        let mut header = [0u8; HEADER as usize];
        self.file.read_exact(&mut header)?;
        let word = |i: usize| u64::from_le_bytes(header[i * 8..i * 8 + 8].try_into().unwrap());
        let (previous, next, start, removed, inserted) = (
            word(0),
            word(1),
            word(2) as usize,
            word(3) as usize,
            word(4) as usize,
        );
        let range = |i: usize| Range {
            start: i32::from_le_bytes(header[i..i + 4].try_into().unwrap()),
            end: i32::from_le_bytes(header[i + 4..i + 8].try_into().unwrap()),
        };
        let (offset, len, replace_len, selection) = if redo {
            (node + HEADER + removed as u64, inserted, removed, range(48))
        } else {
            (node + HEADER, removed, inserted, range(40))
        };
        let mut bytes = vec![0; len];
        self.file.seek(SeekFrom::Start(offset))?;
        self.file.read_exact(&mut bytes)?;
        let change = std::str::from_utf8(&bytes).map_err(io::Error::other)?;
        self.current
            .replace_range(start..start + replace_len, change);
        self.pasted = header[if redo { 57 } else { 56 }] != 0;
        self.paragraph_source = word(if redo { 9 } else { 8 });
        if redo {
            self.cursor = node;
            self.redo = next;
        } else {
            self.cursor = previous;
            self.redo = node;
        }
        Ok(Some(selection))
    }

    fn source(&mut self) -> io::Result<Option<String>> {
        if self.paragraph_source == NONE {
            return Ok(None);
        }
        self.file.seek(SeekFrom::Start(self.paragraph_source))?;
        let mut size = [0; 8];
        self.file.read_exact(&mut size)?;
        let size = usize::try_from(u64::from_le_bytes(size)).map_err(io::Error::other)?;
        let mut bytes = vec![0; size];
        self.file.read_exact(&mut bytes)?;
        String::from_utf8(bytes).map(Some).map_err(io::Error::other)
    }

    fn begin_paragraphs(&mut self, base: &str, selection: Range) -> io::Result<()> {
        self.end_group(selection)?;
        let previous = self.paragraph_source;
        let source = if previous != NONE {
            previous
        } else {
            let offset = self.file.seek(SeekFrom::End(0))?;
            self.file.write_all(&(base.len() as u64).to_le_bytes())?;
            self.file.write_all(base.as_bytes())?;
            offset
        };
        self.group = Some((self.current.clone(), selection, self.pasted, previous));
        self.paragraph_source = source;
        Ok(())
    }
}

const SUBCLASS: usize = 0x5148;
const PROP: &str = "QingquSessionHistory";
unsafe fn selection(hwnd: HWND) -> Range {
    let mut r = Range::default();
    SendMessageW(hwnd, EXGETSEL, 0, (&mut r as *mut Range) as isize);
    r
}
pub unsafe fn install(hwnd: HWND) -> Result<(), String> {
    let state = Box::new(RefCell::new(
        Journal::new(text(hwnd)).map_err(|_| "无法创建文本撤销日志，请检查临时目录。")?,
    ));
    let ptr = Box::into_raw(state);
    if SetWindowSubclass(hwnd, Some(proc), SUBCLASS, ptr as usize) == 0 {
        drop(Box::from_raw(ptr));
        return Err("无法启用会话撤销历史。".into());
    }
    SetPropW(hwnd, crate::platform::wide(PROP).as_ptr(), ptr.cast());
    // This journal owns history; don't retain a second Rich Edit undo buffer.
    SendMessageW(hwnd, SETUNDOLIMIT, 0, 0);
    Ok(())
}
pub unsafe fn begin_group(hwnd: HWND) {
    let ptr = GetPropW(hwnd, crate::platform::wide(PROP).as_ptr()) as *mut RefCell<Journal>;
    if let Some(cell) = ptr.as_ref() {
        if let Ok(mut state) = cell.try_borrow_mut() {
            if state.group.is_none() {
                state.group = Some((
                    state.current.clone(),
                    selection(hwnd),
                    state.pasted,
                    state.paragraph_source,
                ));
            }
        }
    }
}

/// A paragraph command's original pasted text, tied to this exact undo state.
/// Normal edits invalidate it; undo/redo restores its disk offset with the text.
pub unsafe fn paragraph_source(hwnd: HWND) -> Result<Option<String>, String> {
    let ptr = GetPropW(hwnd, crate::platform::wide(PROP).as_ptr()) as *mut RefCell<Journal>;
    let cell = ptr.as_ref().ok_or("无法读取文本处理记录。")?;
    let mut state = cell
        .try_borrow_mut()
        .map_err(|_| "文本正在处理，请稍后重试。")?;
    state
        .source()
        .map_err(|_| "无法读取合并前的段落记录。".into())
}

pub unsafe fn begin_paragraph_change(hwnd: HWND, base: &str) -> Result<(), String> {
    let ptr = GetPropW(hwnd, crate::platform::wide(PROP).as_ptr()) as *mut RefCell<Journal>;
    let cell = ptr.as_ref().ok_or("无法保存文本处理记录。")?;
    let mut state = cell
        .try_borrow_mut()
        .map_err(|_| "文本正在处理，请稍后重试。")?;
    state
        .begin_paragraphs(base, selection(hwnd))
        .map_err(|_| "无法保存合并前的段落记录，请检查临时目录。".into())
}
pub unsafe fn end_group(hwnd: HWND) {
    let ptr = GetPropW(hwnd, crate::platform::wide(PROP).as_ptr()) as *mut RefCell<Journal>;
    if let Some(cell) = ptr.as_ref() {
        if let Ok(mut state) = cell.try_borrow_mut() {
            if state.end_group(selection(hwnd)).is_err() {
                report(hwnd);
            }
        }
    }
}
unsafe fn report(hwnd: HWND) {
    // No silent truncation of the session history if the disk is full.
    MessageBoxW(
        GetParent(hwnd),
        crate::platform::wide("文本撤销日志写入失败，请检查临时目录和磁盘空间。当前文本仍保留。")
            .as_ptr(),
        crate::platform::wide("轻取 OCR").as_ptr(),
        MB_OK | MB_ICONERROR,
    );
}
unsafe extern "system" fn proc(
    hwnd: HWND,
    msg: u32,
    w: WPARAM,
    l: LPARAM,
    _: usize,
    data: usize,
) -> LRESULT {
    let cell = &*(data as *mut RefCell<Journal>);
    if msg == WM_NCDESTROY {
        RemovePropW(hwnd, crate::platform::wide(PROP).as_ptr());
        let result = DefSubclassProc(hwnd, msg, w, l);
        drop(Box::from_raw(data as *mut RefCell<Journal>));
        return result;
    }
    let Ok(mut state) = cell.try_borrow_mut() else {
        return DefSubclassProc(hwnd, msg, w, l);
    };
    if msg == EM_CANUNDO {
        return (state.cursor != NONE || state.group.is_some()) as isize;
    }
    if msg == WM_USER + 85 {
        return (state.redo != NONE) as isize;
    }
    if msg == EM_UNDO || msg == WM_UNDO || msg == REDO {
        let result = state
            .end_group(selection(hwnd))
            .and_then(|_| state.travel(msg == REDO));
        return match result {
            Ok(Some(r)) => {
                set_text(hwnd, &state.current);
                super::scintilla::set_pasted_document(hwnd, state.pasted);
                SendMessageW(hwnd, EXSETSEL, 0, (&r as *const Range) as isize);
                SendMessageW(hwnd, EM_SCROLLCARET, 0, 0);
                1
            }
            Ok(None) => 0,
            Err(_) => {
                report(hwnd);
                0
            }
        };
    }
    // control_proc routes keydown; suppress the corresponding control character too.
    if msg == WM_CHAR && matches!(w, 25 | 26) {
        return 0;
    }
    let user_change = matches!(
        msg,
        WM_CHAR
            | WM_KEYDOWN
            | WM_CUT
            | WM_PASTE
            | super::scintilla::api::SCI_PASTE
            | WM_CLEAR
            | 0x010f
            | 0x010e
            | 0x0109
    );
    let mutation = user_change || matches!(msg, WM_SETTEXT | EM_REPLACESEL);
    if !mutation {
        drop(state);
        return DefSubclassProc(hwnd, msg, w, l);
    }
    let before = selection(hwnd);
    if user_change && state.end_group(before).is_err() {
        report(hwnd);
    }
    // Keydown, caret navigation and IME updates often change no text. Keep
    // the caller's dirty flag, but avoid reading/diffing the whole document
    // unless this message actually modified it. WM_SETTEXT resets the flag.
    let modified = SendMessageW(hwnd, EM_GETMODIFY, 0, 0);
    SendMessageW(hwnd, EM_SETMODIFY, 0, 0);
    let result = DefSubclassProc(hwnd, msg, w, l);
    let changed = SendMessageW(hwnd, EM_GETMODIFY, 0, 0) != 0;
    if modified != 0 && !changed {
        SendMessageW(hwnd, EM_SETMODIFY, 1, 0);
    }
    if !changed && !matches!(msg, WM_SETTEXT | EM_REPLACESEL) {
        return result;
    }
    if state
        .record(
            text(hwnd),
            before,
            selection(hwnd),
            super::scintilla::pasted_document(hwnd),
        )
        .is_err()
    {
        report(hwnd);
    }
    result
}
