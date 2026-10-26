//! The right pane hosts the system Rich Edit table engine.
use super::*;
impl Main {
    pub(super) fn table_popup(&mut self, h: &mut Host) -> bool {
        if !self.table_active || !self.settings.show_translation {
            return false;
        }
        unsafe {
            let mut point = POINT::default();
            GetCursorPos(&mut point);
            ScreenToClient(h.hwnd, &mut point);
            if PtInRect(&self.table_view.area, point) == 0 {
                return false;
            }
            let choice = menu(
                h.hwnd,
                &[
                    (310, "复制整表（Word / Excel）"),
                    (261, "复制选中内容"),
                    (120, "在独立窗口打开表格"),
                ],
            );
            if choice == 261 {
                let result = self.table_view.copy(&self.tables, true);
                self.set_status(
                    h,
                    result
                        .map(|_| "已复制表格选区".into())
                        .unwrap_or_else(|e| e),
                );
            } else if choice != 0 {
                self.command(h, choice, 0, null_mut());
            }
        }
        true
    }
    pub fn table_key(&mut self, h: &mut Host, w: usize) {
        if self.table_active
            && self.settings.show_translation
            && w == b'C' as usize
            && unsafe { GetKeyState(VK_CONTROL as i32) } < 0
        {
            let result = self.table_view.copy(&self.tables, true);
            self.set_status(
                h,
                result
                    .map(|_| "已复制表格选区".into())
                    .unwrap_or_else(|e| e),
            );
        }
    }
}
pub(super) const TABLE_INDEX: u16 = 263;
pub(super) struct TablePane {
    pub index: usize,
    area: RECT,
    view: Option<table_layout::TableView>,
}
impl TablePane {
    pub fn new() -> Self {
        Self {
            index: 0,
            area: RECT::default(),
            view: None,
        }
    }
    pub fn layout(&mut self, h: &mut Host, area: RECT, tables: &[Table]) {
        self.area = area;
        self.index = self.index.min(tables.len().saturating_sub(1));
        unsafe {
            let combo = GetDlgItem(h.hwnd, TABLE_INDEX as i32);
            if SendMessageW(combo, CB_GETCOUNT, 0, 0) != tables.len() as isize {
                SendMessageW(combo, CB_RESETCONTENT, 0, 0);
                for i in 0..tables.len() {
                    SendMessageW(
                        combo,
                        CB_ADDSTRING,
                        0,
                        platform::wide(&format!("表格 {}", i + 1)).as_ptr() as isize,
                    );
                }
            }
            SendMessageW(combo, CB_SETCURSEL, self.index, 0);
            let (_, height) = dimensions(h.hwnd);
            move_to(
                combo,
                rect(area.left, height - h.px(26.), h.px(130.), h.px(200.)),
            );
            ShowWindow(combo, SW_SHOW);
        }
        if let Some(table) = tables.get(self.index) {
            let view = self
                .view
                .get_or_insert_with(|| table_layout::TableView::create(h));
            view.layout(h, table, area);
        }
    }
    pub fn select_table(&mut self, index: usize) {
        self.index = index;
    }
    pub fn copy(&self, tables: &[Table], selection: bool) -> Result<(), String> {
        let table = tables.get(self.index).ok_or("没有表格结果。")?;
        if selection {
            return self.view.as_ref().ok_or("没有表格结果。")?.copy_selection();
        }
        table_layout::copy_table(table)
    }
}
