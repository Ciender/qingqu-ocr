//! Recognition routing shared by the text selector, toolbar and image jobs.
//! Specialized tasks use an isolated snapshot and never mutate text settings.
use super::*;

impl Main {
    pub(super) fn sync_ocr_language_selector(&mut self, h: &Host) {
        let choices = self.settings.ocr_languages(&self.settings.text_ocr_mode);
        self.settings.normalize_ocr_language();
        unsafe {
            let selector = GetDlgItem(h.hwnd, LANGUAGE as i32);
            SendMessageW(selector, CB_RESETCONTENT, 0, 0);
            let displayed = if choices.is_empty() {
                &[("auto_detect", "接口默认")][..]
            } else {
                choices
            };
            for &(_, name) in displayed {
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
                displayed
                    .iter()
                    .position(|&(value, _)| value == self.settings.ocr_language)
                    .unwrap_or(0),
                0,
            );
            EnableWindow(selector, i32::from(!choices.is_empty()));
        }
    }
    pub(super) fn recognize_mode(&mut self, h: &mut Host, image: RgbaImage, mode: &str) {
        self.recognize_work(h, mode, |settings, keys| Work::Ocr(image, settings, keys));
    }
    pub(super) fn recognize_stored(&mut self, h: &mut Host, image: crate::image_store::ImageView) {
        let settings = self.settings.picture_settings();
        self.start_recognition(h, settings, |settings, keys| {
            Work::OcrStored(image, settings, keys)
        });
    }
    fn recognize_work(
        &mut self,
        h: &mut Host,
        mode: &str,
        work: impl FnOnce(Box<Settings>, Box<Credentials>) -> Work,
    ) {
        let mut settings = self.settings.clone();
        settings.ocr_mode = mode.into();
        self.start_recognition(h, settings, work);
    }
    fn start_recognition(
        &mut self,
        h: &mut Host,
        settings: Settings,
        work: impl FnOnce(Box<Settings>, Box<Credentials>) -> Work,
    ) {
        self.close_document(h);
        unsafe {
            show_main(h.hwnd);
        }
        if let Err(error) = crate::ocr_cloud::validate(&settings, &self.keys) {
            self.set_status(h, error);
            self.open_settings(h);
            return;
        }
        let mode = settings.ocr_mode.clone();
        let label = settings.ocr_route(&mode);
        self.begin(
            h,
            work(Box::new(settings), Box::new(self.keys.clone())),
            &label,
            Purpose::Ocr {
                formula: mode == "公式",
                vertical: mode == "竖排",
            },
            Vec::new(),
        );
    }
}
