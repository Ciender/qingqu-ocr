//! Target language is selected in the translation pane, independently of OCR.
use super::*;

impl Main {
    pub(super) fn automatic_translation_enabled(&self) -> bool {
        self.settings.auto_translate
            && self.settings.show_translation
            && !self.rendered_active
            && !self.table_active
    }

    pub(super) fn sync_translation_target(&self, h: &Host) {
        unsafe {
            let selector = GetDlgItem(h.hwnd, TARGET_LANGUAGE as i32);
            SendMessageW(selector, CB_RESETCONTENT, 0, 0);
            for &(_, label) in config::TRANSLATION_TARGETS {
                SendMessageW(
                    selector,
                    CB_ADDSTRING,
                    0,
                    platform::wide(label).as_ptr() as isize,
                );
            }
            SendMessageW(
                selector,
                CB_SETCURSEL,
                config::TRANSLATION_TARGETS
                    .iter()
                    .position(|&(value, _)| value == self.settings.translation.target)
                    .unwrap_or(0),
                0,
            );
        }
    }
    pub(super) fn select_translation_target(&mut self, h: &mut Host, index: usize) {
        self.settings.translation.target = config::TRANSLATION_TARGETS[index].0.into();
        if self
            .pending
            .as_ref()
            .is_some_and(|p| matches!(p.purpose, Purpose::Translate))
        {
            self.cancel(h);
        }
        if let Err(error) = config::save_preferences(&self.settings) {
            self.set_status(h, error);
        }
        if self.automatic_translation_enabled() && unsafe { !text(self.source).trim().is_empty() } {
            self.command(h, TRANSLATE, 0, null_mut());
        }
    }
    pub(super) unsafe fn layout_translation_controls(
        &self,
        h: &Host,
        split: i32,
        width: i32,
        height: i32,
        padding: i32,
        footer: i32,
    ) {
        let visible = self.settings.show_translation && !self.rendered_active && !self.table_active;
        let available = width - split - 2 * padding;
        let gap = h.px(4.);
        let show_auto = visible && available >= h.px(270.);
        let auto_width = if show_auto { h.px(88.) } else { 0 };
        let target_width = (available - auto_width - gap).min(h.px(110.));
        let target_right = width - padding;
        for (id, show, bounds) in [
            (
                AUTO_TRANSLATE,
                show_auto,
                rect(split + padding, height - footer, auto_width, footer),
            ),
            (
                TARGET_LANGUAGE,
                visible,
                rect(
                    target_right - target_width,
                    height - footer,
                    target_width,
                    h.px(260.),
                ),
            ),
        ] {
            let control = GetDlgItem(h.hwnd, id as i32);
            ShowWindow(control, if show { SW_SHOW } else { SW_HIDE });
            if show {
                move_to(control, bounds);
            }
        }
    }
}
