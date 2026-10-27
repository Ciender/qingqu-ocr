//! Persistent, scrollable service references with native Rich Edit URL links.
use super::*;
use crate::service_info::ServiceInfo;

const REFERENCE: i32 = 920;
const AUTO_URL: u32 = WM_USER + 91;
const GET_EVENT_MASK: u32 = WM_USER + 59;
const SET_EVENT_MASK: u32 = WM_USER + 69;
const LINK_EVENT: u32 = 0x0400_0000;
const LINK_NOTICE: u32 = 0x070b;

impl SettingsEditor {
    pub(super) unsafe fn init_reference(&mut self, h: &mut Host) -> Result<(), String> {
        self.reference = h.rich(REFERENCE as u16, true);
        if self.reference.is_null() {
            return Err("无法创建接口参考区。".into());
        }
        SendMessageW(self.reference, SETTEXTMODE, 2 | 8, 0);
        SendMessageW(self.reference, SETUNDOLIMIT, 0, 0);
        SendMessageW(self.reference, AUTO_URL, 1, 0);
        let mask = SendMessageW(self.reference, GET_EVENT_MASK, 0, 0);
        SendMessageW(
            self.reference,
            SET_EVENT_MASK,
            0,
            mask | LINK_EVENT as isize,
        );
        ShowWindow(self.reference, SW_HIDE);
        Ok(())
    }
    pub(super) fn refresh_reference(&mut self, h: &mut Host) {
        self.reference_urls.clear();
        self.reference_actions.clear();
        if !matches!(self.category, 0 | 5) || self.api >= config::INTERFACE_PATHS.len() {
            return;
        }
        if let Ok(settings) = serde_json::from_value::<Settings>(self.draft.clone()) {
            let (settings, index) = self.reference_profile(settings, self.api);
            let info = ServiceInfo::for_interface(&settings, index);
            self.reference_urls = info.sources.iter().map(|source| source.url).collect();
            self.reference_actions = ServiceInfo::setup_sources(&settings, index);
            unsafe {
                SendMessageW(self.reference, EM_SETREADONLY, 0, 0);
                set_text(self.reference, &info.reference_text());
                // Switching to rich-text mode resets the control's default ink.
                // Apply the current palette after inserting the reference text.
                SetWindowTheme(
                    self.reference,
                    platform::wide(if h.dark {
                        "DarkMode_Explorer"
                    } else {
                        "Explorer"
                    })
                    .as_ptr(),
                    null(),
                );
                SendMessageW(self.reference, WM_SETFONT, h.font.0 as usize, 1);
                edit_style(self.reference, h.theme, 12., h.dpi);
                SendMessageW(self.reference, EM_SETSEL, 0, 0);
                SendMessageW(self.reference, EM_SETREADONLY, 1, 0);
                for (i, link) in self.reference_actions.iter().enumerate().take(2) {
                    let name = if i == 0 {
                        "获取密钥 ↗"
                    } else {
                        "开通服务 / 用量 ↗"
                    };
                    h.button_label(930 + i as u16, name);
                    set_text(GetDlgItem(h.hwnd, 930 + i as i32), link.label);
                }
            }
        }
    }
    pub(super) fn reference_height(&self, _h: &Host) -> i32 {
        // References live exclusively in the question-mark card. Keep the
        // hidden Rich Edit for URL dispatch, without reserving editor space.
        0
    }
    pub(super) unsafe fn layout_reference(&self, h: &Host) {
        let (width, height) = dimensions(h.hwnd);
        let pane = self.reference_height(h);
        ShowWindow(
            self.reference,
            if pane >= h.px(70.) { SW_SHOW } else { SW_HIDE },
        );
        for id in 930..932 {
            ShowWindow(GetDlgItem(h.hwnd, id), SW_HIDE);
        }
        if pane < h.px(70.) {
            return;
        }
        let top = height - h.px(54.) - pane;
        let actions = if self.reference_actions.is_empty() {
            0
        } else {
            h.px(30.)
        };
        move_to(
            self.reference,
            rect(
                h.px(131.),
                top + h.px(24.) + actions,
                width - h.px(147.),
                (pane - h.px(28.) - actions).max(h.px(24.)),
            ),
        );
        for i in 0..2 {
            let button = GetDlgItem(h.hwnd, 930 + i);
            ShowWindow(
                button,
                if (i as usize) < self.reference_actions.len() {
                    SW_SHOW
                } else {
                    SW_HIDE
                },
            );
            move_to(
                button,
                rect(
                    h.px(131. + i as f32 * 155.),
                    top + h.px(24.),
                    h.px(148.),
                    h.px(25.),
                ),
            );
        }
    }
    pub(super) unsafe fn paint_reference(&self, h: &Host, dc: HDC) {
        let (width, height) = dimensions(h.hwnd);
        let pane = self.reference_height(h);
        if pane < h.px(70.) {
            return;
        }
        let top = height - h.px(54.) - pane;
        fill(
            dc,
            &rect(h.px(131.), top, width - h.px(147.), 1),
            h.theme.border,
        );
        label(
            dc,
            &rect(h.px(131.), top + h.px(2.), width - h.px(147.), h.px(20.)),
            &format!(
                "参考 · 密钥 / 计价 / 免费额度 · {}",
                crate::service_info::VERIFIED
            ),
            h.theme.muted,
            h.font.0,
            DT_SINGLELINE | DT_VCENTER | DT_END_ELLIPSIS,
        );
    }
    pub(in crate::native) unsafe fn reference_notice(&self, h: &Host, data: LPARAM) -> bool {
        if data == 0 {
            return false;
        }
        let header = std::ptr::read_unaligned(data as *const NMHDR);
        if header.hwndFrom != self.reference || header.code != LINK_NOTICE {
            return false;
        }
        let notice =
            std::ptr::read_unaligned(data as *const windows::Win32::UI::Controls::RichEdit::ENLINK);
        if notice.msg == WM_LBUTTONUP {
            let range = notice.chrg;
            let url = win::range_text(
                self.reference,
                Range {
                    start: range.cpMin,
                    end: range.cpMax,
                },
            );
            if self.reference_urls.iter().any(|&allowed| allowed == url) {
                open_reference(h, &url);
            }
        }
        true
    }
}

pub(super) unsafe fn open_reference(h: &Host, url: &str) {
    let result = windows_sys::Win32::UI::Shell::ShellExecuteW(
        h.hwnd,
        platform::wide("open").as_ptr(),
        platform::wide(url).as_ptr(),
        null(),
        null(),
        SW_SHOWNORMAL,
    );
    if result as isize <= 32 {
        win::error(h.hwnd, "无法打开浏览器，请复制参考区 URL 手动打开。");
    }
}
