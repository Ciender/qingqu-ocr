//! Native three-column task connections and the independent API service selector.
use super::*;

impl SettingsEditor {
    pub(super) unsafe fn init_bindings(&mut self, h: &mut Host) {
        for i in 0..6u16 {
            for id in [800 + i, 830 + i, 850 + i] {
                h.control(
                    "COMBOBOX",
                    "",
                    WS_TABSTOP | CBS_DROPDOWNLIST as u32 | WS_VSCROLL,
                    id,
                );
            }
            let task = GetDlgItem(h.hwnd, 830 + i as i32);
            SendMessageW(
                task,
                CB_ADDSTRING,
                0,
                platform::wide(config::TASK_NAMES[i as usize]).as_ptr() as isize,
            );
            SendMessageW(task, CB_SETCURSEL, 0, 0);
            EnableWindow(task, 0);
            h.button(810 + i, Some(Icon::Settings), "识别参数 / 语言");
            h.button(820 + i, Some(Icon::Help), "当前接口 · 免费额度与计费");
        }
        let selector = h.control(
            "COMBOBOX",
            "",
            WS_TABSTOP | CBS_DROPDOWNLIST as u32 | WS_VSCROLL,
            840,
        );
        for service in config::API_SERVICES {
            SendMessageW(
                selector,
                CB_ADDSTRING,
                0,
                platform::wide(config::provider_label(service)).as_ptr() as isize,
            );
        }
        SendMessageW(selector, CB_SETCURSEL, self.service, 0);
        h.button(513, None, "配置 API 服务");
    }
    pub(super) unsafe fn sync_bindings(&self, h: &Host) {
        let Ok(settings) = serde_json::from_value::<Settings>(self.draft.clone()) else {
            return;
        };
        for i in 0..6usize {
            let provider = settings.interface_provider(i);
            let choices = config::interface_choices(i);
            let selector = GetDlgItem(h.hwnd, 800 + i as i32);
            SendMessageW(selector, CB_RESETCONTENT, 0, 0);
            for (_, name) in &choices {
                SendMessageW(
                    selector,
                    CB_ADDSTRING,
                    0,
                    platform::wide(name).as_ptr() as isize,
                );
            }
            let stored = self
                .draft
                .pointer(config::INTERFACE_PATHS[i])
                .and_then(Value::as_str)
                .unwrap_or("");
            SendMessageW(
                selector,
                CB_SETCURSEL,
                choices.iter().position(|v| v.0 == stored).unwrap_or(0),
                0,
            );
            let follow = i == 5 && settings.picture_ocr_provider.is_empty();
            let modes = if follow {
                &[("", "跟随文字整套配置")][..]
            } else {
                settings.interface_modes(i, provider)
            };
            let selector = GetDlgItem(h.hwnd, 850 + i as i32);
            SendMessageW(selector, CB_RESETCONTENT, 0, 0);
            for (_, name) in modes {
                SendMessageW(
                    selector,
                    CB_ADDSTRING,
                    0,
                    platform::wide(name).as_ptr() as isize,
                );
            }
            SendMessageW(
                selector,
                CB_SETCURSEL,
                modes
                    .iter()
                    .position(|v| v.0 == settings.interface_mode(i))
                    .unwrap_or(0),
                0,
            );
            EnableWindow(selector, i32::from(!follow && modes.len() > 1));
        }
    }
    pub(super) unsafe fn layout_bindings(&self, h: &Host, w: i32) {
        let left = h.px(131.);
        let task_width = h.px(80.);
        let arrow = h.px(22.);
        let right = w - h.px(16.);
        let actions = h.px(65.);
        let flexible = (right - left - task_width - arrow * 2 - actions).max(h.px(300.));
        let service_width = flexible * 52 / 100;
        let endpoint_width = flexible - service_width;
        for i in 0..6i32 {
            let y = h.px(96. + i as f32 * 47.);
            move_to(
                GetDlgItem(h.hwnd, 830 + i),
                rect(left, y, task_width, h.px(230.)),
            );
            move_to(
                GetDlgItem(h.hwnd, 800 + i),
                rect(left + task_width + arrow, y, service_width, h.px(290.)),
            );
            move_to(
                GetDlgItem(h.hwnd, 850 + i),
                rect(
                    left + task_width + arrow * 2 + service_width,
                    y,
                    endpoint_width,
                    h.px(260.),
                ),
            );
            move_to(
                GetDlgItem(h.hwnd, 810 + i),
                rect(right - actions + h.px(4.), y, h.px(28.), h.px(28.)),
            );
            move_to(
                GetDlgItem(h.hwnd, 820 + i),
                rect(right - h.px(28.), y, h.px(28.), h.px(28.)),
            );
        }
    }
    pub(super) unsafe fn paint_bindings(&self, h: &Host, dc: HDC, w: i32) {
        let left = h.px(131.);
        let service_left = left + h.px(102.);
        let flexible = (w - h.px(16.) - left - h.px(80. + 44. + 65.)).max(h.px(300.));
        let endpoint_left = service_left + flexible * 52 / 100 + h.px(22.);
        for (x, width, title) in [
            (left, h.px(80.), "功能"),
            (service_left, flexible * 52 / 100, "API 服务"),
            (endpoint_left, flexible * 48 / 100, "具体接口"),
        ] {
            label(
                dc,
                &rect(x, h.px(66.), width, h.px(24.)),
                title,
                h.theme.muted,
                h.font.0,
                DT_SINGLELINE | DT_VCENTER,
            );
        }
        for i in 0..6 {
            let y = h.px(96. + i as f32 * 47.);
            for x in [left + h.px(80.), endpoint_left - h.px(22.)] {
                label(
                    dc,
                    &rect(x, y, h.px(22.), h.px(27.)),
                    "→",
                    h.theme.muted,
                    h.font.0,
                    DT_SINGLELINE | DT_CENTER | DT_VCENTER,
                );
            }
        }
        label(dc, &rect(left, h.px(392.), w - left - h.px(20.), h.px(50.)), "每个功能按本行配置调用。右侧齿轮设置语言等参数，问号查看实际接口。\n密钥、地域和模型在“API 服务”中配置；图片可跟随整套文字配置。", h.theme.muted, h.font.0, DT_WORDBREAK);
    }
    pub(super) fn reference_profile(
        &self,
        mut settings: Settings,
        mut index: usize,
    ) -> (Settings, usize) {
        if self.category == 5 {
            let provider = config::API_SERVICES[self.service];
            index = if config::TRANSLATION_PROVIDERS.contains(&provider) {
                4
            } else if provider == "Mathpix" {
                3
            } else {
                0
            };
            match index {
                4 => settings.translation.provider = provider.into(),
                3 => settings.formula_provider = provider.into(),
                _ => settings.ocr_provider = provider.into(),
            }
            settings.normalize_ocr_modes();
        }
        (settings, index)
    }
}
