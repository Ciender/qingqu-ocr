//! Percentage entry for a picture; applying never modifies the original pixels.
use super::*;

pub struct Zoom {
    pub target: HWND,
    pub percent: u32,
}
impl Zoom {
    pub fn init(&mut self, h: &mut Host) -> Result<(), String> {
        unsafe {
            h.title_buttons(false, false);
            h.min_size = (300., 145.);
            h.control("STATIC", "缩放比例（1–800%）", 0, 269);
            let input = h.control(
                "EDIT",
                &self.percent.to_string(),
                WS_TABSTOP | ES_NUMBER as u32 | ES_AUTOHSCROLL as u32,
                270,
            );
            SetWindowSubclass(input, Some(input_proc), 1, 0);
            h.button(271, None, "应用");
            h.button_label(271, "应用");
            SendMessageW(input, EM_SETSEL, 0, -1);
            SetFocus(input);
        }
        Ok(())
    }
    pub fn layout(&self, h: &Host) {
        unsafe {
            h.place(269, 16., 32., 270., 24.);
            h.place(270, 16., 65., 170., 28.);
            h.place(271, 202., 65., 80., 28.);
        }
    }
    pub fn command(&self, h: &Host, id: u16) {
        if id != 271 {
            return;
        }
        unsafe {
            match text(GetDlgItem(h.hwnd, 270)).trim().parse::<u32>() {
                Ok(value) if (1..=800).contains(&value) => {
                    if IsWindow(self.target) != 0 {
                        PostMessageW(self.target, PICTURE_ZOOM, value as usize, 0);
                    }
                    PostMessageW(h.hwnd, WM_CLOSE, 0, 0);
                }
                _ => win::error(h.hwnd, "请输入 1 到 800 的百分比，例如 125 表示 125%。"),
            }
        }
    }
}
unsafe extern "system" fn input_proc(
    hwnd: HWND,
    msg: u32,
    w: WPARAM,
    l: LPARAM,
    _id: usize,
    _data: usize,
) -> LRESULT {
    if msg == WM_KEYDOWN && w == VK_RETURN as usize {
        PostMessageW(GetParent(hwnd), WM_COMMAND, 271, 0);
        return 0;
    }
    DefSubclassProc(hwnd, msg, w, l)
}
