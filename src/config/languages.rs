//! Language selectors follow the endpoint actually used by the text task.
use super::Settings;

const BAIDU_GENERAL: &[(&str, &str)] = &[
    ("CHN_ENG", "中英混合"),
    ("ENG", "英语"),
    ("JAP", "日语"),
    ("KOR", "韩语"),
    ("FRE", "法语"),
    ("SPA", "西班牙语"),
    ("POR", "葡萄牙语"),
    ("GER", "德语"),
    ("ITA", "意大利语"),
    ("RUS", "俄语"),
];
const BAIDU_ACCURATE: &[(&str, &str)] = &[
    ("auto_detect", "自动检测"),
    ("CHN_ENG", "中英混合"),
    ("ENG", "英语"),
    ("JAP", "日语"),
    ("KOR", "韩语"),
    ("FRE", "法语"),
    ("SPA", "西班牙语"),
    ("POR", "葡萄牙语"),
    ("GER", "德语"),
    ("ITA", "意大利语"),
    ("RUS", "俄语"),
    ("DAN", "丹麦语"),
    ("DUT", "荷兰语"),
    ("MAL", "马来语"),
    ("SWE", "瑞典语"),
    ("IND", "印尼语"),
    ("POL", "波兰语"),
    ("ROM", "罗马尼亚语"),
    ("TUR", "土耳其语"),
    ("GRE", "希腊语"),
    ("HUN", "匈牙利语"),
    ("THA", "泰语"),
    ("VIE", "越南语"),
    ("ARA", "阿拉伯语"),
    ("HIN", "印地语"),
];
const TENCENT_GENERAL: &[(&str, &str)] = &[
    ("auto_detect", "自动检测"),
    ("CHN_ENG", "中英混合"),
    ("MIX", "多语言混排"),
    ("JAP", "日语"),
    ("KOR", "韩语"),
    ("FRE", "法语"),
    ("SPA", "西班牙语"),
    ("POR", "葡萄牙语"),
    ("GER", "德语"),
    ("ITA", "意大利语"),
    ("RUS", "俄语"),
];
const TENCENT_ACCURATE: &[(&str, &str)] = &[("CHN_ENG", "通用文字"), ("MUL", "多语种")];

pub const TRANSLATION_LANGUAGES: &[(&str, &str)] = &[
    ("简体中文", "简体中文"),
    ("繁体中文", "繁体中文"),
    ("英语", "英语"),
    ("日语", "日语"),
    ("韩语", "韩语"),
    ("法语", "法语"),
    ("德语", "德语"),
    ("俄语", "俄语"),
    ("西班牙语", "西班牙语"),
];
pub const TRANSLATION_TARGETS: &[(&str, &str)] = &[
    ("auto", "自动选择"),
    ("简体中文", "简体中文"),
    ("繁体中文", "繁体中文"),
    ("英语", "英语"),
    ("日语", "日语"),
    ("韩语", "韩语"),
    ("法语", "法语"),
    ("德语", "德语"),
    ("俄语", "俄语"),
    ("西班牙语", "西班牙语"),
];

pub fn ocr_language_choices(provider: &str, mode: &str) -> &'static [(&'static str, &'static str)] {
    match (provider, mode) {
        ("百度 OCR", "高精度") => BAIDU_ACCURATE,
        ("百度 OCR", "通用多语言") => BAIDU_GENERAL,
        ("腾讯云 OCR", "高精度") => TENCENT_ACCURATE,
        ("腾讯云 OCR", "PP-OCRv6" | "通用多语言") => TENCENT_GENERAL,
        _ => &[],
    }
}
pub fn ocr_language_value<'a>(provider: &str, mode: &str, stored: &'a str) -> &'a str {
    // Vertical recognition has its own task profile, not the text selector's language.
    if mode == "竖排" {
        return "CHN_ENG";
    }
    let choices = ocr_language_choices(provider, mode);
    if choices.iter().any(|&(value, _)| value == stored) {
        return stored;
    }
    match (provider, mode, stored) {
        ("腾讯云 OCR", "高精度", "auto_detect" | "ENG") => "CHN_ENG",
        ("腾讯云 OCR", "高精度", _) => "MUL",
        ("腾讯云 OCR", _, "ENG") => "CHN_ENG",
        ("腾讯云 OCR", _, "MUL") => "MIX",
        _ => choices.first().map_or("auto_detect", |&(value, _)| value),
    }
}
impl Settings {
    pub fn ocr_languages(&self, mode: &str) -> &'static [(&'static str, &'static str)] {
        ocr_language_choices(self.ocr_provider_for(mode), mode)
    }
    pub fn ocr_language_for(&self, mode: &str) -> &str {
        ocr_language_value(self.ocr_provider_for(mode), mode, &self.ocr_language)
    }
    pub fn normalize_ocr_language(&mut self) {
        if !self.ocr_languages(&self.text_ocr_mode).is_empty() {
            self.ocr_language = self.ocr_language_for(&self.text_ocr_mode).into();
        }
    }
}
