//! Owned native information card. No network requests or idle polling.
use super::*;
use crate::service_info::{ServiceInfo, VERIFIED};

pub const DISMISS: u32 = WM_APP + 16;
pub const CLOSE_TIMER: usize = 9;
pub fn is_button(id: usize) -> bool {
    id == 512 || (820..826).contains(&id)
}
pub struct ServiceHelp {
    info: ServiceInfo,
    owner: HWND,
    anchor: HWND,
    body: HWND,
}
impl ServiceHelp {
    pub fn open(h: &Host, anchor: HWND, info: ServiceInfo) -> Result<HWND, String> {
        create(
            "免费额度与计费",
            (540., 480.),
            Content::ServiceHelp(Self {
                info,
                owner: h.hwnd,
                anchor,
                body: null_mut(),
            }),
            None,
            &h.palette,
            h.dark,
            14.,
        )
    }
    pub fn init(&mut self, h: &mut Host) -> Result<(), String> {
        unsafe {
            SetWindowLongPtrW(h.hwnd, GWLP_HWNDPARENT, self.owner as isize);
            h.button(CLOSE, Some(Icon::Close), "关闭额度说明");
            self.body = h.rich(900, true);
            if self.body.is_null() {
                return Err("无法显示额度说明。".into());
            }
            set_text(self.body, &self.info.body);
            for (i, link) in self.info.sources.iter().enumerate() {
                h.button(910 + i as u16, None, link.url);
                h.button_label(910 + i as u16, &format!("{} ↗", link.label));
            }
            SetPropW(
                h.hwnd,
                platform::wide("QingquServiceHelp").as_ptr(),
                1 as HANDLE,
            );
            let mut anchor = RECT::default();
            GetWindowRect(self.anchor, &mut anchor);
            let mut monitor = MONITORINFO {
                cbSize: size_of::<MONITORINFO>() as u32,
                ..Default::default()
            };
            GetMonitorInfoW(
                MonitorFromWindow(self.anchor, MONITOR_DEFAULTTONEAREST),
                &mut monitor,
            );
            let area = monitor.rcWork;
            let width = h.px(540.).min((area.right - area.left - 16).max(240));
            let height = h.px(480.).min((area.bottom - area.top - 16).max(200));
            let x = (anchor.right - width)
                .clamp(area.left + 8, (area.right - width - 8).max(area.left + 8));
            let y = if anchor.bottom + height + 8 <= area.bottom {
                anchor.bottom + 3
            } else {
                (anchor.top - height - 3).max(area.top + 8)
            };
            SetWindowPos(h.hwnd, HWND_TOPMOST, x, y, width, height, SWP_NOACTIVATE);
            self.schedule_close(h);
        }
        Ok(())
    }
    pub fn layout(&mut self, h: &Host) {
        let (w, height) = dimensions(h.hwnd);
        let footer = h.px(52.);
        let links = self.info.sources.len() as i32 * h.px(27.);
        let bottom = height - footer - links;
        unsafe {
            move_to(
                self.body,
                rect(
                    h.px(10.),
                    h.px(56.),
                    w - h.px(20.),
                    (bottom - h.px(62.)).max(h.px(30.)),
                ),
            );
            for (i, _) in self.info.sources.iter().enumerate() {
                move_to(
                    GetDlgItem(h.hwnd, 910 + i as i32),
                    rect(
                        h.px(10.),
                        bottom + i as i32 * h.px(27.),
                        w - h.px(20.),
                        h.px(24.),
                    ),
                );
            }
        }
    }
    pub fn command(&self, h: &Host, id: u16) {
        if let Some(link) = id
            .checked_sub(910)
            .and_then(|i| self.info.sources.get(i as usize))
        {
            unsafe {
                let result = windows_sys::Win32::UI::Shell::ShellExecuteW(
                    h.hwnd,
                    platform::wide("open").as_ptr(),
                    platform::wide(link.url).as_ptr(),
                    null(),
                    null(),
                    SW_SHOWNORMAL,
                );
                if result as isize <= 32 {
                    win::error(h.hwnd, "无法打开浏览器，请稍后重试。");
                }
            }
        }
    }
    pub fn schedule_close(&self, h: &Host) {
        unsafe {
            SetTimer(h.hwnd, CLOSE_TIMER, 650, None);
        }
    }
    pub fn timer(&self, h: &Host) {
        unsafe {
            KillTimer(h.hwnd, CLOSE_TIMER);
            let mut pointer = POINT::default();
            GetCursorPos(&mut pointer);
            let inside = [h.hwnd, self.anchor].iter().any(|&hwnd| {
                let mut bounds = RECT::default();
                IsWindowVisible(hwnd) != 0
                    && GetWindowRect(hwnd, &mut bounds) != 0
                    && PtInRect(&bounds, pointer) != 0
            });
            if !inside {
                PostMessageW(h.hwnd, WM_CLOSE, 0, 0);
            }
        }
    }
    pub unsafe fn paint(&self, h: &Host, dc: HDC) {
        let (w, height) = dimensions(h.hwnd);
        label(
            dc,
            &rect(h.px(12.), h.px(28.), w - h.px(24.), h.px(25.)),
            &self.info.title,
            h.theme.text,
            h.font.0,
            DT_SINGLELINE | DT_VCENTER | DT_END_ELLIPSIS,
        );
        label(
            dc,
            &rect(h.px(12.), height - h.px(47.), w - h.px(24.), h.px(20.)),
            &format!("资料核对：{VERIFIED} · 非账号实时剩余额度"),
            h.theme.muted,
            h.font.0,
            DT_SINGLELINE | DT_VCENTER,
        );
        label(
            dc,
            &rect(h.px(12.), height - h.px(27.), w - h.px(24.), h.px(20.)),
            "额度与价格可能调整，请以官方页面及账号控制台为准。",
            h.theme.muted,
            h.font.0,
            DT_SINGLELINE | DT_VCENTER | DT_END_ELLIPSIS,
        );
    }
}
