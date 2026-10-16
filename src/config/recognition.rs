//! OCR task profiles, legacy mode migration and readable API routes.
use super::Settings;

/// Labels describe the actual provider; the legacy mode keys stay compatible.
pub fn text_ocr_modes(provider: &str) -> &'static [(&'static str, &'static str)] {
    match provider {
        "百度 OCR" => &[
            ("PP-OCRv6", "PaddleOCR"),
            ("高精度", "高精度 OCR"),
            ("通用多语言", "标准 OCR"),
        ],
        "阿里云 OCR" => &[("PP-OCRv6", "通用文字识别"), ("高精度", "全文识别高精版")],
        "阿里云 OCR统一识别" => &[
            ("通用文字", "统一识别·通用文字"),
            ("高精度", "统一识别·高精版"),
            ("手写体", "统一识别·手写体"),
            ("通用多语言", "统一识别·多语言"),
        ],
        "阿里云 Qwen-OCR" => &[
            ("PP-OCRv6", "文字提取"),
            ("高精度", "文字定位（含坐标）"),
            ("通用多语言", "多语言识别"),
        ],
        "阿里云 Qwen-VL" => &[("高精度", "视觉文字识别")],
        _ => &[
            ("PP-OCRv6", "通用文字识别"),
            ("高精度", "通用高精度识别"),
            ("通用多语言", "多语言文字识别"),
        ],
    }
}

impl Settings {
    pub fn text_modes(&self, provider: &str) -> &'static [(&'static str, &'static str)] {
        if provider == "阿里云 Qwen-OCR" {
            if self.qwen_ocr.protocol == "chat" {
                return &[("高精度", "视觉文字识别（提示词）")];
            }
            return &[
                ("PP-OCRv6", "文字提取"),
                ("高精度", "文字定位（含坐标）"),
                ("文档解析", "文档解析"),
                ("通用多语言", "多语言识别"),
            ];
        }
        text_ocr_modes(provider)
    }
    fn normalized_text_mode(&self, provider: &str, mode: &str) -> String {
        if self
            .text_modes(provider)
            .iter()
            .any(|&(key, _)| key == mode)
        {
            return mode.into();
        }
        // The old third dedicated-Aliyun item called RecognizeGeneral too.
        if provider == "阿里云 OCR" && mode == "通用多语言" {
            return "PP-OCRv6".into();
        }
        // Preserve the endpoint behind the legacy configurable unified item.
        if provider == "阿里云 OCR统一识别" && mode == "PP-OCRv6" {
            return match self.aliyun_unified_type.as_str() {
                "General" => "通用文字",
                "HandWriting" => "手写体",
                "MultiLang" => "通用多语言",
                _ => "高精度",
            }
            .into();
        }
        "高精度".into()
    }
    pub fn picture_provider(&self) -> &str {
        if self.picture_ocr_provider.is_empty() {
            &self.ocr_provider
        } else {
            &self.picture_ocr_provider
        }
    }
    /// Task snapshot: changing or running a picture never rewrites screenshot settings.
    pub fn picture_settings(&self) -> Self {
        let mut settings = self.clone();
        settings.ocr_provider = self.picture_provider().into();
        settings.ocr_mode = if self.picture_ocr_provider.is_empty() {
            self.text_ocr_mode.clone()
        } else {
            self.normalized_text_mode(self.picture_provider(), &self.picture_ocr_mode)
        };
        settings.text_ocr_mode = settings.ocr_mode.clone();
        settings.ocr_language = if self.picture_ocr_provider.is_empty() {
            self.ocr_language.clone()
        } else {
            self.picture_ocr_language.clone()
        };
        settings.normalize_ocr_language();
        settings
    }
    pub fn picture_menu_label(&self) -> String {
        let settings = self.picture_settings();
        let name = settings
            .text_modes(&settings.ocr_provider)
            .iter()
            .find(|&&(mode, _)| mode == settings.ocr_mode)
            .map_or("文字识别", |&(_, name)| name);
        format!(
            "识别图片 · {} / {name}",
            super::provider_label(&settings.ocr_provider)
        )
    }
    /// The same action is used by requests, status labels and service help.
    pub fn aliyun_action(&self, mode: &str) -> &'static str {
        match mode {
            "表格" if self.table_ocr_mode == "高精度" => "RecognizeAdvanced",
            "表格" => "RecognizeTableOcr",
            "公式" => "RecognizeEduFormula",
            "高精度" | "竖排" => "RecognizeAdvanced",
            _ => "RecognizeGeneral",
        }
    }

    pub fn aliyun_kind(&self, mode: &str) -> &str {
        match mode {
            "表格" => "Table",
            "高精度" | "竖排" => "Advanced",
            "通用多语言" => "MultiLang",
            "通用文字" => "General",
            "手写体" => "HandWriting",
            _ => &self.aliyun_unified_type,
        }
    }

    pub fn normalize_ocr_modes(&mut self) {
        self.migrate_bindings();
        // Empty means an older preferences file. Preserve a legacy text
        // selection, and recover the normal text default after a legacy
        // one-shot formula/table task was accidentally persisted as active.
        let mode = if self.text_ocr_mode.is_empty() {
            &self.ocr_mode
        } else {
            &self.text_ocr_mode
        };
        self.text_ocr_mode = self.normalized_text_mode(&self.ocr_provider, mode);
        self.ocr_mode = self.text_ocr_mode.clone();
        if !self.picture_ocr_provider.is_empty()
            && !super::OCR_PROVIDERS.contains(&self.picture_ocr_provider.as_str())
        {
            self.picture_ocr_provider.clear();
        }
        self.picture_ocr_mode =
            self.normalized_text_mode(self.picture_provider(), &self.picture_ocr_mode);
        self.normalize_task_bindings();
    }
    pub fn ocr_provider_for(&self, mode: &str) -> &str {
        match mode {
            "表格" => &self.table_provider,
            "竖排" => &self.vertical_provider,
            "公式" => &self.formula_provider,
            _ => &self.ocr_provider,
        }
    }
    /// Human-readable route shared by the mode selector and job status. A
    /// task selects its own interface profile; switching to formula therefore
    /// never silently uses the text endpoint from the text-interface tab.
    pub fn ocr_route(&self, mode: &str) -> String {
        let provider = self.ocr_provider_for(mode);
        let endpoint = match provider {
            "百度 OCR" => match mode {
                "公式" => match self.baidu_formula_endpoint.as_str() {
                    "doc_analysis" => "试卷分析 · doc_analysis",
                    _ => "公式识别 · formula",
                },
                "高精度" => "高精度文字 · accurate",
                "表格" => "表格识别 · table",
                "通用多语言" => "标准多语言文字 · general",
                "竖排" => "竖排文字 · general",
                _ => "PaddleOCR · PP-OCRv6 · pp_ocr",
            },
            "Mathpix" => "图文与公式 · text",
            "腾讯云 OCR" => match mode {
                "表格" => "RecognizeTableAccurateOCR",
                "高精度" | "竖排" => "GeneralAccurateOCR",
                _ => "GeneralBasicOCR",
            },
            "阿里云 OCR" => self.aliyun_action(mode),
            "阿里云 OCR统一识别" => {
                return format!("{provider} / RecognizeAllText · {}", self.aliyun_kind(mode));
            }
            "阿里云 Qwen-OCR" | "阿里云 Qwen-VL" => {
                let profile = if provider == "阿里云 Qwen-OCR" {
                    &self.qwen_ocr
                } else {
                    &self.qwen_vl
                };
                let model = profile.model.trim();
                let task = match profile.protocol.as_str() {
                    "dashscope" => self.qwen_task_for(mode),
                    // Compatible calls use a prompt and ignore profile.task,
                    // exactly as qwen::payload does for this protocol.
                    "chat" => "chat/completions（提示词）",
                    _ => "无效调用协议",
                };
                return format!("{provider} / {model} · {task}");
            }
            _ => mode,
        };
        format!("{} / {endpoint}", super::provider_label(provider))
    }
}
