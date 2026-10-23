//! Native Scintilla plain-text editor. Its built-in margin and text share the
//! same layout and paint pass; the application never draws line numbers.
use super::win::*;
use crate::platform::wide;
use std::ptr::{null, null_mut};

#[allow(dead_code)]
pub mod api {
    include!(concat!(env!("OUT_DIR"), "/scintilla_constants.rs"));
}
use api::*;
const SUBCLASS: usize = 0x534349;
const KIND: &str = "QingquScintilla";
const DIRTY: &str = "QingquScintillaDirty";
const VERSION: &str = "QingquScintillaVersion";
const SAVED_VERSION: &str = "QingquScintillaSavedVersion";
const PASTED: &str = "QingquScintillaPastedDocument";

pub unsafe fn pasted_document(hwnd: HWND) -> bool {
    !GetPropW(hwnd, wide(PASTED).as_ptr()).is_null()
}

pub unsafe fn set_pasted_document(hwnd: HWND, pasted: bool) {
    if pasted {
        SetPropW(hwnd, wide(PASTED).as_ptr(), 1 as HANDLE);
    } else {
        RemovePropW(hwnd, wide(PASTED).as_ptr());
    }
}

unsafe extern "C" {
    fn Scintilla_RegisterClasses(instance: *mut std::ffi::c_void) -> i32;
}

pub unsafe fn register() -> Result<(), String> {
    static REGISTERED: std::sync::OnceLock<Result<(), String>> = std::sync::OnceLock::new();
    REGISTERED
        .get_or_init(|| {
            if Scintilla_RegisterClasses(GetModuleHandleW(null()).cast()) == 0 {
                Err("无法注册 Scintilla 编辑组件。".into())
            } else {
                Ok(())
            }
        })
        .clone()
}
pub unsafe fn is_editor(hwnd: HWND) -> bool {
    !GetPropW(hwnd, wide(KIND).as_ptr()).is_null()
}
pub unsafe fn call(hwnd: HWND, msg: u32, w: usize, l: isize) -> isize {
    SendMessageW(hwnd, msg, w, l)
}
fn utf8(s: &str) -> Vec<u8> {
    let mut bytes = s.replace("\r\n", "\n").replace('\r', "\n").into_bytes();
    bytes.push(0);
    bytes
}
unsafe fn wide_text(ptr: isize) -> String {
    if ptr == 0 {
        return String::new();
    }
    let ptr = ptr as *const u16;
    let mut length = 0;
    while *ptr.add(length) != 0 {
        length += 1;
    }
    String::from_utf16_lossy(std::slice::from_raw_parts(ptr, length))
}
pub unsafe fn text(hwnd: HWND) -> String {
    let length = call(hwnd, SCI_GETLENGTH, 0, 0).max(0) as usize;
    let mut bytes = vec![0u8; length + 1];
    call(hwnd, SCI_GETTEXT, bytes.len(), bytes.as_mut_ptr() as isize);
    bytes.truncate(length);
    String::from_utf8_lossy(&bytes).into_owned()
}

/// TSV rows stay on one visual line with shared pixel tab stops.
pub unsafe fn table_layout(hwnd: HWND, enabled: bool) {
    call(
        hwnd,
        SCI_SETWRAPMODE,
        if enabled { SC_WRAP_NONE } else { SC_WRAP_CHAR } as usize,
        0,
    );
    let body = text(hwnd);
    let mut widths: Vec<i32> = Vec::new();
    if enabled {
        for line in body.lines().filter(|s| s.contains('\t')) {
            for (col, value) in line.split('\t').enumerate() {
                if widths.len() <= col {
                    widths.resize(col + 1, 0);
                }
                let text = utf8(value);
                widths[col] = widths[col].max(
                    call(
                        hwnd,
                        SCI_TEXTWIDTH,
                        STYLE_DEFAULT as usize,
                        text.as_ptr() as isize,
                    ) as i32
                        + 20,
                );
            }
        }
    }
    for line in 0..line_count(hwnd) {
        call(hwnd, SCI_CLEARTABSTOPS, line, 0);
        let mut x = 0;
        for width in &widths {
            x += *width;
            call(hwnd, SCI_ADDTABSTOP, line, x as isize);
        }
    }
}
unsafe fn to_utf16(hwnd: HWND, position: isize) -> i32 {
    let line = call(hwnd, SCI_LINEFROMPOSITION, position.max(0) as usize, 0);
    let start = call(hwnd, SCI_POSITIONFROMLINE, line as usize, 0);
    (call(
        hwnd,
        SCI_INDEXPOSITIONFROMLINE,
        line as usize,
        SC_LINECHARACTERINDEX_UTF16 as isize,
    ) + call(hwnd, SCI_COUNTCODEUNITS, start as usize, position)) as i32
}
unsafe fn from_utf16(hwnd: HWND, units: i32) -> isize {
    let length = call(hwnd, SCI_GETLENGTH, 0, 0);
    if units < 0 {
        return length;
    }
    let line = call(
        hwnd,
        SCI_LINEFROMINDEXPOSITION,
        units as usize,
        SC_LINECHARACTERINDEX_UTF16 as isize,
    );
    let line = line.max(0);
    let start = call(hwnd, SCI_POSITIONFROMLINE, line as usize, 0).max(0);
    let base = call(
        hwnd,
        SCI_INDEXPOSITIONFROMLINE,
        line as usize,
        SC_LINECHARACTERINDEX_UTF16 as isize,
    );
    call(
        hwnd,
        SCI_POSITIONRELATIVECODEUNITS,
        start as usize,
        units as isize - base,
    )
    .clamp(0, length)
}
pub unsafe fn line_count(hwnd: HWND) -> usize {
    call(hwnd, SCI_GETLINECOUNT, 0, 0).max(1) as usize
}
pub unsafe fn line_numbers(hwnd: HWND, visible: bool) {
    SetPropW(
        hwnd,
        wide("QingquScintillaNumbers").as_ptr(),
        usize::from(visible) as HANDLE,
    );
    update_margin(hwnd);
}
pub unsafe fn update_margin(hwnd: HWND) {
    let visible = !GetPropW(hwnd, wide("QingquScintillaNumbers").as_ptr()).is_null();
    let width = if visible {
        let sample = format!("_{}", "9".repeat(line_count(hwnd).to_string().len().max(2)));
        call(
            hwnd,
            SCI_TEXTWIDTH,
            STYLE_LINENUMBER as usize,
            utf8(&sample).as_ptr() as isize,
        ) + 8
    } else {
        0
    };
    if call(hwnd, SCI_GETMARGINWIDTHN, 0, 0) != width {
        call(hwnd, SCI_SETMARGINWIDTHN, 0, width);
    }
}
pub unsafe fn style(hwnd: HWND, theme: Theme, size: f32, dpi: u32) {
    let actual = GetDpiForWindow(hwnd).max(96);
    let font = b"Microsoft YaHei UI\0";
    for style in [STYLE_DEFAULT, STYLE_LINENUMBER] {
        call(
            hwnd,
            SCI_STYLESETFONT,
            style as usize,
            font.as_ptr() as isize,
        );
        call(
            hwnd,
            SCI_STYLESETSIZEFRACTIONAL,
            style as usize,
            (size * 100. * dpi as f32 / actual as f32).round() as isize,
        );
        call(
            hwnd,
            SCI_STYLESETFORE,
            style as usize,
            if style == STYLE_LINENUMBER {
                theme.muted
            } else {
                theme.text
            } as isize,
        );
        call(
            hwnd,
            SCI_STYLESETBACK,
            style as usize,
            if style == STYLE_LINENUMBER {
                theme.surface
            } else {
                theme.bg
            } as isize,
        );
    }
    call(hwnd, SCI_STYLECLEARALL, 0, 0);
    // STYLECLEARALL also copies DEFAULT into the number style.
    call(
        hwnd,
        SCI_STYLESETFORE,
        STYLE_LINENUMBER as usize,
        theme.muted as isize,
    );
    call(
        hwnd,
        SCI_STYLESETBACK,
        STYLE_LINENUMBER as usize,
        theme.surface as isize,
    );
    call(hwnd, SCI_SETCARETFORE, theme.text as usize, 0);
    call(hwnd, SCI_SETSELBACK, 1, theme.accent as isize);
    call(hwnd, SCI_SETSELFORE, 1, 0xffffff);
    update_margin(hwnd);
}
pub unsafe fn create(parent: HWND, id: u16, readonly: bool) -> HWND {
    let hwnd = child(
        parent,
        "Scintilla",
        "",
        WS_TABSTOP | WS_VSCROLL | ES_MULTILINE as u32,
        id,
        null_mut(),
    );
    if hwnd.is_null() {
        return hwnd;
    }
    SetPropW(hwnd, wide(KIND).as_ptr(), 1 as HANDLE);
    call(hwnd, SCI_SETCODEPAGE, SC_CP_UTF8 as usize, 0);
    call(hwnd, SCI_SETEOLMODE, SC_EOL_LF as usize, 0);
    call(
        hwnd,
        SCI_ALLOCATELINECHARACTERINDEX,
        SC_LINECHARACTERINDEX_UTF16 as usize,
        0,
    );
    call(hwnd, SCI_SETWRAPMODE, SC_WRAP_CHAR as usize, 0);
    call(hwnd, SCI_SETLAYOUTCACHE, SC_CACHE_PAGE as usize, 0);
    call(hwnd, SCI_SETBUFFEREDDRAW, 1, 0);
    call(hwnd, SCI_SETPHASESDRAW, SC_PHASES_TWO as usize, 0);
    call(hwnd, SCI_SETMARGINTYPEN, 0, SC_MARGIN_NUMBER as isize);
    call(hwnd, SCI_SETMARGINWIDTHN, 1, 0);
    call(hwnd, SCI_SETMARGINLEFT, 0, 7);
    call(hwnd, SCI_SETMARGINRIGHT, 0, 7);
    call(hwnd, SCI_SETSCROLLWIDTHTRACKING, 1, 0);
    call(hwnd, SCI_SETENDATLASTLINE, 1, 0);
    call(hwnd, SCI_SETIMEINTERACTION, SC_IME_INLINE as usize, 0);
    call(hwnd, SCI_SETREADONLY, usize::from(readonly), 0);
    SetWindowSubclass(hwnd, Some(compatibility), SUBCLASS, 0);
    SetWindowSubclass(hwnd, Some(track_edits), SUBCLASS + 1, 0);
    hwnd
}

/// Keep the app's existing UTF-16 edit command contract at this boundary.
/// Scintilla itself stores UTF-8 and owns all text/margin rendering and scroll.
unsafe extern "system" fn compatibility(
    hwnd: HWND,
    msg: u32,
    w: usize,
    l: isize,
    _: usize,
    _: usize,
) -> isize {
    match msg {
        WM_SETTEXT | EM_REPLACESEL => {
            let body = utf8(&wide_text(l));
            let readonly = call(hwnd, SCI_GETREADONLY, 0, 0);
            if msg == WM_SETTEXT {
                call(hwnd, SCI_SETREADONLY, 0, 0);
            }
            call(
                hwnd,
                if msg == WM_SETTEXT {
                    SCI_SETTEXT
                } else {
                    SCI_REPLACESEL
                },
                0,
                body.as_ptr() as isize,
            );
            if msg == WM_SETTEXT {
                call(hwnd, SCI_SETREADONLY, readonly as usize, 0);
                set_pasted_document(hwnd, false);
            }
            update_margin(hwnd);
            return 1;
        }
        WM_GETTEXTLENGTH => {
            let length = call(hwnd, SCI_GETLENGTH, 0, 0);
            return to_utf16(hwnd, length) as isize + line_count(hwnd) as isize - 1;
        }
        WM_GETTEXT => {
            if l != 0 && w > 0 {
                let body: Vec<_> = text(hwnd).replace('\n', "\r\n").encode_utf16().collect();
                let n = body.len().min(w - 1);
                std::ptr::copy_nonoverlapping(body.as_ptr(), l as *mut u16, n);
                *(l as *mut u16).add(n) = 0;
                return n as isize;
            }
            return 0;
        }
        EXGETSEL => {
            if l != 0 {
                *(l as *mut Range) = Range {
                    start: to_utf16(hwnd, call(hwnd, SCI_GETSELECTIONSTART, 0, 0)),
                    end: to_utf16(hwnd, call(hwnd, SCI_GETSELECTIONEND, 0, 0)),
                };
            }
            return 0;
        }
        EM_SETSEL | EXSETSEL => {
            let r = if msg == EXSETSEL {
                if l == 0 {
                    return 0;
                }
                *(l as *const Range)
            } else {
                Range {
                    start: w as i32,
                    end: l as i32,
                }
            };
            let end = from_utf16(hwnd, r.end);
            let start = if r.start < 0 {
                end
            } else {
                from_utf16(hwnd, r.start)
            };
            call(hwnd, SCI_SETSEL, start as usize, end);
            return 0;
        }
        EM_GETMODIFY => {
            return (GetPropW(hwnd, wide(VERSION).as_ptr())
                != GetPropW(hwnd, wide(SAVED_VERSION).as_ptr())
                || !GetPropW(hwnd, wide(DIRTY).as_ptr()).is_null()) as isize
        }
        EM_SETMODIFY => {
            SetPropW(hwnd, wide(DIRTY).as_ptr(), usize::from(w != 0) as HANDLE);
            if w == 0 {
                call(hwnd, SCI_SETSAVEPOINT, 0, 0);
                SetPropW(
                    hwnd,
                    wide(SAVED_VERSION).as_ptr(),
                    GetPropW(hwnd, wide(VERSION).as_ptr()),
                );
            }
            return 0;
        }
        SETUNDOLIMIT => {
            call(hwnd, SCI_SETUNDOCOLLECTION, usize::from(w != 0), 0);
            return 0;
        }
        EM_UNDO | WM_UNDO => return call(hwnd, SCI_UNDO, 0, 0),
        REDO => return call(hwnd, SCI_REDO, 0, 0),
        GETSCROLLPOS | SETSCROLLPOS => {
            if l == 0 {
                return 0;
            }
            let height = call(hwnd, SCI_TEXTHEIGHT, 0, 0).max(1);
            if msg == GETSCROLLPOS {
                *(l as *mut POINT) = POINT {
                    x: call(hwnd, SCI_GETXOFFSET, 0, 0) as i32,
                    y: (call(hwnd, SCI_GETFIRSTVISIBLELINE, 0, 0) * height) as i32,
                };
            } else {
                let p = &*(l as *const POINT);
                call(
                    hwnd,
                    SCI_SETFIRSTVISIBLELINE,
                    (p.y as isize / height).max(0) as usize,
                    0,
                );
                call(hwnd, SCI_SETXOFFSET, p.x.max(0) as usize, 0);
            }
            return 1;
        }
        EM_LINEINDEX => return to_utf16(hwnd, call(hwnd, SCI_POSITIONFROMLINE, w, 0)) as isize,
        WM_SETFONT | SETZOOM | EXLIMITTEXT => return 0,
        WM_NCDESTROY => {
            RemovePropW(hwnd, wide(KIND).as_ptr());
            set_pasted_document(hwnd, false);
        }
        _ => {}
    }
    DefSubclassProc(hwnd, msg, w, l)
}

pub unsafe fn changed(hwnd: HWND) {
    SetPropW(hwnd, wide(DIRTY).as_ptr(), 1 as HANDLE);
    let key = wide(VERSION);
    let version = GetPropW(hwnd, key.as_ptr()) as usize;
    SetPropW(hwnd, key.as_ptr(), (version + 1) as HANDLE);
    update_margin(hwnd);
}

#[repr(C)]
struct Modification {
    header: NMHDR,
    position: isize,
    ch: i32,
    modifiers: i32,
    kind: i32,
}

/// Receive content changes even while a parent command borrows its state.
/// This also covers equal-byte-length replacements and IME commits.
pub unsafe fn notification(l: isize) {
    if l == 0 {
        return;
    }
    let header = &*(l as *const NMHDR);
    if header.code != 2008 || !is_editor(header.hwndFrom) {
        return;
    }
    let event = &*(l as *const Modification);
    if event.kind & (SC_MOD_INSERTTEXT | SC_MOD_DELETETEXT) as i32 != 0 {
        changed(event.header.hwndFrom);
    }
}

unsafe extern "system" fn track_edits(
    hwnd: HWND,
    msg: u32,
    w: usize,
    l: isize,
    _: usize,
    _: usize,
) -> isize {
    let paste = matches!(msg, WM_PASTE | SCI_PASTE)
        || (msg == WM_KEYDOWN
            && ((w == b'V' as usize && GetKeyState(VK_CONTROL as i32) < 0)
                || (w == VK_INSERT as usize && GetKeyState(VK_SHIFT as i32) < 0)));
    let version_before = if paste {
        GetPropW(hwnd, wide(VERSION).as_ptr())
    } else {
        null_mut()
    };
    let mutation = matches!(
        msg,
        WM_CHAR
            | WM_KEYDOWN
            | WM_SETTEXT
            | EM_REPLACESEL
            | WM_CUT
            | WM_PASTE
            | WM_CLEAR
            | WM_IME_COMPOSITION
            | SCI_PASTE
    );
    let before = if mutation {
        call(hwnd, SCI_GETLENGTH, 0, 0)
    } else {
        0
    };
    let result = DefSubclassProc(hwnd, msg, w, l);
    let after = if mutation {
        call(hwnd, SCI_GETLENGTH, 0, 0)
    } else {
        0
    };
    if mutation && (before != after || matches!(msg, WM_SETTEXT | EM_REPLACESEL)) {
        let key = wide(VERSION);
        let version = GetPropW(hwnd, key.as_ptr()) as usize;
        SetPropW(hwnd, key.as_ptr(), (version + 1) as HANDLE);
    }
    if paste && GetPropW(hwnd, wide(VERSION).as_ptr()) != version_before {
        set_pasted_document(hwnd, true);
    }
    if mutation && after == 0 {
        set_pasted_document(hwnd, false);
    }
    result
}
