//! Native tables through Microsoft's windows-rs TOM2 bindings. No RTF parser,
//! table drawing, header inference or spreadsheet-specific recognition rules.
use super::*;
use crate::ocr::Table;
use windows::{
    core::{IUnknown, Interface, BSTR},
    Win32::UI::Controls::RichEdit::*,
};

pub(super) const TABLE_EDITOR: u16 = 266;
pub(super) const COPY_SELECTION: u16 = 267;
pub(super) const COPY_TABLE_PROP: &str = "QingquCopyTable";

#[derive(Clone, Copy, Debug)]
struct Slot {
    start: i32,
    end: i32,
    row: usize,
    col: usize,
    colspan: usize,
}
pub(super) struct TableView {
    pub editor: HWND,
    rendered: String,
    table: Option<Table>,
    slots: Vec<Slot>,
}
impl TableView {
    pub fn create(h: &mut Host) -> Self {
        unsafe {
            let existing = GetDlgItem(h.hwnd, TABLE_EDITOR as i32);
            let editor = if existing.is_null() {
                h.rich(TABLE_EDITOR, true)
            } else {
                existing
            };
            SendMessageW(editor, SETTEXTMODE, 2 | 8, 0);
            SendMessageW(editor, SETUNDOLIMIT, 0, 0);
            SetPropW(
                editor,
                platform::wide(COPY_TABLE_PROP).as_ptr(),
                1 as HANDLE,
            );
            SetWindowLongPtrW(
                editor,
                GWL_STYLE,
                GetWindowLongPtrW(editor, GWL_STYLE) | WS_HSCROLL as isize,
            );
            Self {
                editor,
                rendered: String::new(),
                table: None,
                slots: Vec::new(),
            }
        }
    }
    pub fn layout(&mut self, h: &Host, table: &Table, area: RECT) {
        unsafe {
            move_to(self.editor, area);
            let actual = GetDpiForWindow(self.editor).max(96) as f32;
            let width = ((area.right - area.left - h.px(22.)).max(h.px(100.)) as f32 * 1440.
                / actual) as i32;
            let key = format!(
                "{width}:{:?}:{}:{}:{}",
                h.font_size, h.dpi, h.theme.text, h.theme.bg
            ) + &serde_json::to_string(table).unwrap_or_default();
            if key != self.rendered {
                SendMessageW(self.editor, WM_SETREDRAW, 0, 0);
                SendMessageW(self.editor, EM_SETREADONLY, 0, 0);
                self.table = None;
                self.slots.clear();
                let result = table.grid().and_then(|owners| {
                    render(
                        self.editor,
                        table,
                        &owners,
                        width,
                        h.font_size * h.dpi as f32 / actual,
                        h.theme,
                    )
                });
                match result {
                    Ok(slots) => {
                        self.slots = slots;
                        self.table = Some(table.clone());
                        self.rendered = key;
                    }
                    Err(error) => {
                        set_text(self.editor, &format!("无法显示表格：{error}"));
                    }
                }
                SendMessageW(self.editor, EM_SETREADONLY, 1, 0);
                SendMessageW(self.editor, EM_SETSEL, 0, 0);
                SendMessageW(self.editor, WM_VSCROLL, SB_TOP as usize, 0);
                SendMessageW(self.editor, WM_SETREDRAW, 1, 0);
            }
            ShowWindow(self.editor, SW_SHOW);
            InvalidateRect(self.editor, null(), 0);
        }
    }
    pub fn copy_selection(&self) -> Result<(), String> {
        let table = self.table.as_ref().ok_or("没有可复制的表格。")?;
        let mut range = Range::default();
        unsafe {
            SendMessageW(
                self.editor,
                EXGETSEL,
                0,
                (&mut range as *mut Range) as isize,
            );
        }
        if range.start == range.end {
            return Err("请先选择单元格，或使用复制整表。".into());
        }
        let selected = self
            .slots
            .iter()
            .filter(|s| s.start < range.end && s.end > range.start)
            .collect::<Vec<_>>();
        if selected.is_empty() {
            let (_, text) = unsafe { super::win::selected(self.editor) };
            return platform::copy_text(&text);
        }
        let top = selected.iter().map(|s| s.row).min().unwrap();
        let bottom = selected.iter().map(|s| s.row).max().unwrap();
        let left = selected.iter().map(|s| s.col).min().unwrap();
        let right = selected
            .iter()
            .map(|s| s.col + s.colspan - 1)
            .max()
            .unwrap();
        let selected = table.selection((top, left), (bottom, right))?;
        copy_table(&selected)
    }
}
/// Let the same system table engine produce Word RTF. Export contains only the
/// requested grid, without titles/notes; Excel receives XML Spreadsheet and TSV.
pub(super) fn copy_table(table: &Table) -> Result<(), String> {
    let owners = table.grid()?;
    unsafe {
        let parent = CreateWindowExW(
            0,
            platform::wide("STATIC").as_ptr(),
            platform::wide("").as_ptr(),
            WS_POPUP,
            0,
            0,
            800,
            600,
            null_mut(),
            null_mut(),
            GetModuleHandleW(null()),
            null(),
        );
        struct Window(HWND);
        impl Drop for Window {
            fn drop(&mut self) {
                unsafe {
                    DestroyWindow(self.0);
                }
            }
        }
        if parent.is_null() {
            return Err("无法创建表格导出控件。".into());
        }
        let _window = Window(parent);
        let editor = edit(parent, 0, null_mut(), false);
        if editor.is_null() {
            return Err("无法创建表格导出控件。".into());
        }
        SendMessageW(editor, SETTEXTMODE, 2 | 8, 0);
        let grid = Table {
            header: String::new(),
            footer: String::new(),
            ..table.clone()
        };
        render(
            editor,
            &grid,
            &owners,
            table.cols as i32 * 1500,
            11.,
            Theme::new("vscode", false),
        )?;
        let rtf = stream_rtf(editor);
        if !rtf.starts_with(b"{\\rtf") {
            return Err("系统没有返回表格文档。".into());
        }
        platform::copy_table(&table.spreadsheet_xml()?, &table.tsv(), &rtf)
    }
}
unsafe fn document(editor: HWND) -> windows::core::Result<ITextDocument2> {
    let mut raw: *mut std::ffi::c_void = null_mut();
    SendMessageW(
        editor,
        EM_GETOLEINTERFACE,
        0,
        (&mut raw as *mut *mut std::ffi::c_void) as isize,
    );
    if raw.is_null() {
        return Err(windows::core::Error::from_hresult(windows::core::HRESULT(
            0x80004002u32 as i32,
        )));
    }
    IUnknown::from_raw(raw).cast()
}
fn render(
    editor: HWND,
    table: &Table,
    owners: &[Option<usize>],
    width: i32,
    size: f32,
    theme: Theme,
) -> Result<Vec<Slot>, String> {
    unsafe { render_tom(editor, table, owners, width, size, theme) }
        .map_err(|e| format!("系统表格接口失败（{}）。", e.code()))
}
unsafe fn render_tom(
    editor: HWND,
    table: &Table,
    owners: &[Option<usize>],
    width: i32,
    size: f32,
    theme: Theme,
) -> windows::core::Result<Vec<Slot>> {
    let doc = document(editor)?;
    let prefix = if !table.header.is_empty() {
        format!("{}\r\r", table.header.replace('\n', "\r"))
    } else {
        String::new()
    };
    doc.Range2(0, i32::MAX)?
        .SetText(&BSTR::from(prefix.as_str()))?;
    let start = prefix.encode_utf16().count() as i32;
    let cursor = doc.Range2(start, start)?;
    let mut weights = vec![4usize; table.cols];
    for cell in &table.cells {
        if cell.colspan == 1 {
            weights[cell.col] = weights[cell.col].max(
                cell.text
                    .lines()
                    .map(|s| s.chars().map(|c| if c.is_ascii() { 1 } else { 2 }).sum())
                    .max()
                    .unwrap_or(0)
                    .min(32),
            );
        }
    }
    let total: usize = weights.iter().sum();
    let widths = weights
        .iter()
        .map(|v| (width * (*v as i32) / total.max(1) as i32).max((size * 95.) as i32))
        .collect::<Vec<_>>();
    let mut segments = Vec::new();
    for r in 0..table.rows {
        let mut cells = Vec::new();
        let mut col = 0;
        while col < table.cols {
            let owner = owners[r * table.cols + col].map(|i| &table.cells[i]);
            let span = owner.map_or(1, |c| c.colspan);
            cells.push((col, span, owner));
            col += span;
        }
        let position = if r == 0 {
            start
        } else {
            doc.Range2(0, i32::MAX)?.GetEnd()? - 1
        };
        cursor.SetRange(position, position)?;
        let props = cursor.GetRow()?;
        props.SetCellCount(cells.len() as i32)?;
        props.SetCellCountCache(cells.len() as i32)?;
        props.SetCellMargin(80)?;
        props.SetHeight((size * 30.) as i32)?;
        for (i, (col, span, owner)) in cells.iter().enumerate() {
            props.SetCellIndex(i as i32)?;
            props.SetCellWidth(widths[*col..col + span].iter().sum())?;
            props.SetCellMergeFlags(owner.filter(|c| c.rowspan > 1).map_or(0, |c| {
                if c.row == r {
                    tomVTopCell.0
                } else {
                    tomVLowCell.0
                }
            }))?;
            props.SetCellColorBack(theme.bg as i32)?;
            props.SetCellColorFore(theme.text as i32)?;
            props.SetCellBorderWidths(10, 10, 10, 10)?;
            props.SetCellBorderColors(
                theme.border as i32,
                theme.border as i32,
                theme.border as i32,
                theme.border as i32,
            )?;
        }
        props.Insert(1)?;
        for (col, span, owner) in cells {
            segments.push((
                r,
                col,
                span,
                owner
                    .filter(|c| c.row == r && c.col == col)
                    .map_or("", |c| c.text.as_str()),
            ));
        }
    }
    segments.sort_by_key(|s| (s.0, s.1));
    let positions = cell_ranges(&doc)?
        .into_iter()
        .map(|(start, _)| start)
        .collect::<Vec<_>>();
    if positions.len() != segments.len() {
        return Err(windows::core::Error::from_hresult(windows::core::HRESULT(
            0x80004005u32 as i32,
        )));
    }
    for (position, (_, _, _, text)) in positions.into_iter().zip(&segments).rev() {
        if !text.is_empty() {
            doc.Range2(position, position)?
                .SetText(&BSTR::from(*text))?;
        }
    }
    let mut slots = Vec::new();
    for ((start, end), (r, col, span, _)) in cell_ranges(&doc)?.into_iter().zip(segments) {
        slots.push(Slot {
            start,
            end,
            row: r,
            col,
            colspan: span,
        });
    }
    cursor.SetRange(
        doc.Range2(0, i32::MAX)?.GetEnd()? - 1,
        doc.Range2(0, i32::MAX)?.GetEnd()? - 1,
    )?;
    let tail = &table.footer;
    if !tail.is_empty() {
        cursor.SetText(&BSTR::from(format!("\r{tail}")))?;
    }
    let font = doc.Range2(0, i32::MAX)?.GetFont()?;
    font.SetName(&BSTR::from("Microsoft YaHei UI"))?;
    font.SetSize(size)?;
    font.SetForeColor(theme.text as i32)?;
    Ok(slots)
}

/// TOM2's documented row delimiters are U+FFF9 CR / U+FFFB CR; each physical
/// cell ends with U+0007. Count UTF-16 units, including surrogate pairs.
unsafe fn cell_ranges(doc: &ITextDocument2) -> windows::core::Result<Vec<(i32, i32)>> {
    let text = doc.Range2(0, i32::MAX)?.GetText2(0)?.to_string();
    let units = text.encode_utf16().collect::<Vec<_>>();
    let mut result = Vec::new();
    let mut i = 0;
    while i + 1 < units.len() {
        if units[i] == 0xfff9 && units[i + 1] == 13 {
            i += 2;
            let mut start = i;
            while i < units.len() && units[i] != 0xfffb {
                if units[i] == 7 {
                    result.push((start as i32, (i + 1) as i32));
                    start = i + 1;
                }
                i += 1;
            }
        }
        i += 1;
    }
    Ok(result)
}

unsafe fn stream_rtf(editor: HWND) -> Vec<u8> {
    unsafe extern "system" fn output(
        cookie: usize,
        buffer: *mut u8,
        size: i32,
        written: *mut i32,
    ) -> u32 {
        (*(cookie as *mut Vec<u8>))
            .extend_from_slice(std::slice::from_raw_parts(buffer, size as usize));
        *written = size;
        0
    }
    let mut bytes = Vec::<u8>::new();
    let mut stream = EDITSTREAM {
        dwCookie: (&mut bytes as *mut Vec<u8>) as usize,
        dwError: 0,
        pfnCallback: Some(output),
    };
    SendMessageW(
        editor,
        EM_STREAMOUT,
        SF_RTF as usize,
        (&mut stream as *mut EDITSTREAM) as isize,
    );
    bytes
}
