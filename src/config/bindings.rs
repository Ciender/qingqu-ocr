//! One capability catalog for task bindings, labels and request dispatch.
use super::Settings;

pub const TASK_NAMES: &[&str] = &[
    "文字识别",
    "表格识别",
    "竖排识别",
    "公式识别",
    "翻译",
    "图片识别",
];
pub const API_SERVICES: &[&str] = &[
    "百度 OCR",
    "腾讯云 OCR",
    "阿里云 OCR",
    "阿里云 OCR统一识别",
    "阿里云 Qwen-OCR",
    "阿里云 Qwen-VL",
    "Mathpix",
    "谷歌翻译",
    "微软翻译",
    "Index Translate",
    "OpenAI 兼容",
    "百度翻译",
    "阿里云翻译",
    "腾讯翻译",
];

pub fn interface_mode_path(index: usize, provider: &str) -> Option<&'static str> {
    Some(match index {
        0 => "/text_ocr_mode",
        1 => "/table_ocr_mode",
        2 => "/vertical_ocr_mode",
        3 if provider == "百度 OCR" => "/baidu_formula_endpoint",
        3 => "/formula_ocr_mode",
        4 if provider == "谷歌翻译" => "/translation/google_mode",
        4 if provider == "微软翻译" => "/translation/microsoft_mode",
        4 => return None,
        5 => "/picture_ocr_mode",
        _ => return None,
    })
}

impl Settings {
    pub fn interface_provider(&self, index: usize) -> &str {
        match index {
            1 => &self.table_provider,
            2 => &self.vertical_provider,
            3 => &self.formula_provider,
            4 => &self.translation.provider,
            5 => self.picture_provider(),
            _ => &self.ocr_provider,
        }
    }
    pub fn interface_modes(
        &self,
        index: usize,
        provider: &str,
    ) -> &'static [(&'static str, &'static str)] {
        let native_qwen = provider == "阿里云 Qwen-OCR" && self.qwen_ocr.protocol == "dashscope";
        match index {
            0 | 5 => self.text_modes(provider),
            1 => match provider {
                "百度 OCR" => &[("表格", "表格识别")],
                "腾讯云 OCR" => &[("表格", "表格识别 V3")],
                "阿里云 OCR" => &[
                    ("表格", "专用表格识别"),
                    ("高精度", "全文高精版（输出表格）"),
                ],
                "阿里云 OCR统一识别" => &[("表格", "统一识别·表格")],
                _ if native_qwen => &[("table_parsing", "表格解析（内置任务）")],
                "阿里云 Qwen-OCR" | "阿里云 Qwen-VL" => &[("chat", "视觉表格识别（提示词）")],
                _ => &[],
            },
            2 => match provider {
                "百度 OCR" => &[("竖排", "标准 OCR（竖排）")],
                "腾讯云 OCR" => &[("竖排", "高精度 OCR（竖排重排）")],
                "阿里云 OCR" | "阿里云 OCR统一识别" => &[("竖排", "高精版（竖排重排）")],
                _ if native_qwen => &[("advanced_recognition", "文字定位（竖排重排）")],
                "阿里云 Qwen-OCR" | "阿里云 Qwen-VL" => &[("chat", "视觉竖排识别（提示词）")],
                _ => &[],
            },
            3 => match provider {
                "百度 OCR" => &[
                    ("formula", "公式识别（LaTeX）"),
                    ("doc_analysis", "试卷分析（公式字段）"),
                ],
                "阿里云 OCR" => &[("公式", "公式识别（LaTeX）")],
                "Mathpix" => &[("公式", "图文与公式识别")],
                _ if native_qwen => &[("formula_recognition", "公式识别（内置任务）")],
                "阿里云 Qwen-OCR" | "阿里云 Qwen-VL" => &[("chat", "视觉公式识别（提示词）")],
                _ => &[],
            },
            4 => match provider {
                "谷歌翻译" | "微软翻译" => {
                    &[("web", "网页翻译（免密钥）"), ("cloud", "官方云翻译 API")]
                }
                "百度翻译" => &[("translate", "通用翻译 API")],
                "阿里云翻译" => &[("TranslateGeneral", "通用版机器翻译")],
                "腾讯翻译" => &[("TextTranslate", "文本翻译")],
                "Index Translate" | "OpenAI 兼容" => &[("chat", "Chat Completions")],
                _ => &[],
            },
            _ => &[],
        }
    }
    pub fn interface_mode(&self, index: usize) -> &str {
        match index {
            1 => &self.table_ocr_mode,
            2 => &self.vertical_ocr_mode,
            3 if self.formula_provider == "百度 OCR" => &self.baidu_formula_endpoint,
            3 => &self.formula_ocr_mode,
            4 if self.translation.provider == "谷歌翻译" => &self.translation.google_mode,
            4 if self.translation.provider == "微软翻译" => &self.translation.microsoft_mode,
            4 => self
                .interface_modes(4, &self.translation.provider)
                .first()
                .map_or("", |v| v.0),
            5 if self.picture_ocr_provider.is_empty() => &self.text_ocr_mode,
            5 => &self.picture_ocr_mode,
            _ => &self.text_ocr_mode,
        }
    }
    pub fn migrate_bindings(&mut self) {
        if self.recognition_bindings_version >= 1 {
            return;
        }
        // A global native task used to override every task button. Recover its
        // text choice once; specialized buttons now always bind their own task.
        if self.qwen_ocr.protocol == "dashscope" && self.qwen_ocr.task != "auto" {
            let legacy = match self.qwen_ocr.task.as_str() {
                "text_recognition" => "PP-OCRv6",
                "multi_lan" => "通用多语言",
                "document_parsing" => "文档解析",
                _ => "高精度",
            };
            if self.ocr_provider == "阿里云 Qwen-OCR" {
                self.text_ocr_mode = legacy.into();
            }
            if self.picture_provider() == "阿里云 Qwen-OCR" {
                self.picture_ocr_mode = legacy.into();
            }
        }
        // Empty provider now follows the complete text profile. Retain an old
        // picture's explicitly different endpoint/language as an independent row.
        let text_mode = if self.text_ocr_mode.is_empty() {
            &self.ocr_mode
        } else {
            &self.text_ocr_mode
        };
        if self.picture_ocr_provider.is_empty()
            && (self.picture_ocr_mode != *text_mode
                || self.picture_ocr_language != self.ocr_language)
        {
            self.picture_ocr_provider = self.ocr_provider.clone();
        }
        self.recognition_bindings_version = 1;
    }
    pub fn normalize_task_bindings(&mut self) {
        for index in 1..=3 {
            let provider = self.interface_provider(index);
            let choices = self.interface_modes(index, provider);
            let mode = self.interface_mode(index);
            if !choices.iter().any(|v| v.0 == mode) {
                let value = choices.first().map_or("", |v| v.0).to_owned();
                match index {
                    1 => self.table_ocr_mode = value,
                    2 => self.vertical_ocr_mode = value,
                    3 if self.formula_provider == "百度 OCR" => {
                        self.baidu_formula_endpoint = value
                    }
                    _ => self.formula_ocr_mode = value,
                }
            }
        }
    }
    pub fn qwen_task_for(&self, mode: &str) -> &'static str {
        // The operation is determined by the bound task, never a shared global override.
        match mode {
            "表格" => "table_parsing",
            "公式" => "formula_recognition",
            "竖排" | "高精度" => "advanced_recognition",
            "文档解析" => "document_parsing",
            "通用多语言" => "multi_lan",
            _ => "text_recognition",
        }
    }
}
