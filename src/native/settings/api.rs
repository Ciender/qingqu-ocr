//! Provider-specific fields for the settings interface tabs.
use super::*;

impl SettingsEditor {
    fn cloud_fields(&mut self, h: &mut Host, provider: &str) {
        if matches!(provider, "阿里云 Qwen-OCR" | "阿里云 Qwen-VL") {
            self.qwen_fields(h, provider == "阿里云 Qwen-OCR");
        } else if provider.starts_with("腾讯") {
            self.add(h, "SecretId", "/tencent_secret_id", Input::Secret, true);
            self.add(h, "SecretKey", "/tencent_secret_key", Input::Secret, true);
            let path = if self.api == 4 {
                "/translation/tencent_region"
            } else {
                "/tencent_region"
            };
            self.add(h, "地域", path, Input::Text, false);
            if self.api == 4 {
                self.add(
                    h,
                    "ProjectId",
                    "/translation/tencent_project_id",
                    Input::Integer(0, i32::MAX as usize),
                    false,
                );
            }
        } else if provider.starts_with("阿里") {
            self.add(
                h,
                "AccessKey ID",
                "/aliyun_access_key_id",
                Input::Secret,
                true,
            );
            self.add(
                h,
                "AccessKey Secret",
                "/aliyun_access_key_secret",
                Input::Secret,
                true,
            );
            if self.api != 4 {
                self.add(
                    h,
                    "服务地域",
                    "/aliyun_region",
                    Input::Choice(vec![
                        ("cn-hangzhou", "中国杭州"),
                        ("ap-southeast-1", "新加坡"),
                    ]),
                    false,
                );
            }
        } else if provider == "Mathpix" {
            self.add(h, "服务地址", "/formula_url", Input::Text, false);
            self.add(h, "App ID", "/formula_appid", Input::Text, true);
            self.add(h, "App Key", "/formula_key", Input::Secret, true);
        } else {
            self.add(h, "API Key", "/api_key", Input::Secret, true);
            self.add(h, "Secret Key", "/secret_key", Input::Secret, true);
        }
    }
    fn qwen_fields(&mut self, h: &mut Host, ocr: bool) {
        let root = if ocr { "/qwen_ocr" } else { "/qwen_vl" };
        let path = |field: &str| format!("{root}/{field}");
        self.add(
            h,
            "百炼 API Key",
            if ocr { "/qwen_ocr_key" } else { "/qwen_vl_key" },
            Input::Secret,
            true,
        );
        self.add(
            h,
            "地域",
            &path("region"),
            Input::Choice(vec![
                ("cn-beijing", "北京"),
                ("ap-southeast-1", "新加坡"),
                ("us-east-1", "美国弗吉尼亚"),
            ]),
            false,
        );
        self.add(h, "工作空间 ID", &path("workspace"), Input::Text, false);
        self.add(
            h,
            "服务地址（可留空）",
            &path("base_url"),
            Input::Text,
            false,
        );
        if ocr {
            self.add(
                h,
                "调用协议",
                &path("protocol"),
                Input::Choice(vec![
                    ("dashscope", "DashScope 原生（内置任务）"),
                    ("chat", "OpenAI 兼容（提示词）"),
                ]),
                false,
            );
        }
        let models = if ocr {
            config::QWEN_OCR_MODELS
        } else {
            config::QWEN_VL_MODELS
        };
        let mut presets = vec![("", "自定义（手动填写）")];
        presets.extend(models.iter().map(|&s| (s, s)));
        self.add(h, "模型预设", &path("model"), Input::Preset(presets), false);
        self.add(h, "模型名称（可修改）", &path("model"), Input::Text, false);
        if !ocr || self.draft["qwen_ocr"]["protocol"].as_str() == Some("chat") {
            self.add(
                h,
                "补充识别提示词",
                &path("prompt"),
                Input::Multiline,
                false,
            );
        }
        self.add(
            h,
            "具体识别任务在“调用配置”中分别绑定。",
            "",
            Input::Heading,
            false,
        );
        self.add(
            h,
            "使用该地域百炼 Key；与传统 OCR AccessKey 不同。",
            "",
            Input::Heading,
            false,
        );
    }
    pub(super) fn binding_fields(&mut self, h: &mut Host) {
        let Ok(settings) = serde_json::from_value::<Settings>(self.draft.clone()) else {
            return;
        };
        let provider = settings.interface_provider(self.api).to_owned();
        let name = settings
            .interface_modes(self.api, &provider)
            .iter()
            .find(|v| v.0 == settings.interface_mode(self.api))
            .map_or("", |v| v.1);
        self.add(
            h,
            &format!(
                "{} → {} → {name}",
                config::TASK_NAMES[self.api],
                config::provider_label(&provider)
            ),
            "",
            Input::Heading,
            false,
        );
        if self.api == 5 && settings.picture_ocr_provider.is_empty() {
            self.add(
                h,
                "图片跟随文字的服务、具体接口和语言。可返回调用配置改为独立服务。",
                "",
                Input::Heading,
                false,
            );
            return;
        }
        if self.api == 0 || self.api == 5 {
            let picture = self.api == 5;
            let mode = settings.interface_mode(self.api);
            let languages = config::ocr_language_choices(&provider, mode);
            if !languages.is_empty() {
                let path = if picture {
                    "/picture_ocr_language"
                } else {
                    "/ocr_language"
                };
                let current = self
                    .draft
                    .pointer(path)
                    .and_then(Value::as_str)
                    .unwrap_or("auto_detect");
                let value = config::ocr_language_value(&provider, mode, current).to_owned();
                *self.draft.pointer_mut(path).unwrap() = json!(value);
                self.add(
                    h,
                    "识别语言",
                    path,
                    Input::Choice(languages.to_vec()),
                    false,
                );
            } else {
                self.add(
                    h,
                    "此接口没有可调语言参数，使用服务默认能力；识别日文需选择支持的接口。",
                    "",
                    Input::Heading,
                    false,
                );
            }
            if provider == "百度 OCR" && mode == "高精度" {
                self.add(
                    h,
                    "自动检测支持日语及中英等语言混排；已知语种时可手动指定。",
                    "",
                    Input::Heading,
                    false,
                );
            }
            if provider == "百度 OCR" {
                self.add(
                    h,
                    "自动校正方向",
                    "/correct_orientation",
                    Input::Check,
                    false,
                );
            }
        }
        if self.api == 4 {
            let mut source = vec![("auto", "自动检测")];
            source.extend_from_slice(config::TRANSLATION_LANGUAGES);
            self.add(
                h,
                "源语言",
                "/translation/source",
                Input::Choice(source),
                false,
            );
            self.add(
                h,
                "目标语言",
                "/translation/target",
                Input::Choice(config::TRANSLATION_TARGETS.to_vec()),
                false,
            );
            if matches!(provider.as_str(), "OpenAI 兼容" | "Index Translate") {
                self.add(h, "流式输出", "/translation/stream", Input::Check, false);
                self.add(
                    h,
                    "系统提示词",
                    "/translation/prompt",
                    Input::Multiline,
                    false,
                );
                self.add(
                    h,
                    "术语表",
                    "/translation/glossary",
                    Input::Multiline,
                    false,
                );
            }
        }
        self.add(
            h,
            "密钥、地域、模型在“API 服务”中管理；本页只设置当前功能参数。",
            "",
            Input::Heading,
            false,
        );
    }
    pub(super) fn service_fields(&mut self, h: &mut Host) {
        let provider = config::API_SERVICES[self.service];
        self.api = if config::TRANSLATION_PROVIDERS.contains(&provider) {
            4
        } else if provider == "Mathpix" {
            3
        } else {
            0
        };
        self.add(
            h,
            config::provider_label(provider),
            "",
            Input::Heading,
            false,
        );
        if self.api != 4 {
            self.cloud_fields(h, provider);
            self.add(
                h,
                "此处只配置服务连接；各功能使用哪个接口请到“调用配置”连接。",
                "",
                Input::Heading,
                false,
            );
            if matches!(provider, "阿里云 OCR" | "阿里云 OCR统一识别") {
                self.add(
                    h,
                    "两种 OCR 产品共用 AccessKey；具体接口及额度归属在调用配置的问号中查看。",
                    "",
                    Input::Heading,
                    false,
                );
            }
            return;
        }
        match provider {
            "谷歌翻译" => {
                self.add(h, "Cloud API Key", "/google_key", Input::Secret, true);
                self.add(
                    h,
                    "选择官方云 API 时需要密钥；网页方式受网络和限流影响。",
                    "",
                    Input::Heading,
                    false,
                );
            }
            "微软翻译" => {
                self.add(h, "订阅密钥", "/microsoft_key", Input::Secret, true);
                self.add(
                    h,
                    "资源地域",
                    "/translation/microsoft_region",
                    Input::Text,
                    false,
                );
                self.add(
                    h,
                    "选择官方云 API 时需要密钥；网页方式受网络和限流影响。",
                    "",
                    Input::Heading,
                    false,
                );
            }
            "百度翻译" => {
                self.add(h, "App ID", "/translate_appid", Input::Text, true);
                self.add(h, "密钥", "/translate_secret", Input::Secret, true);
                self.add(
                    h,
                    "百度翻译与百度 OCR 使用不同凭据。",
                    "",
                    Input::Heading,
                    false,
                );
            }
            "阿里云翻译" | "腾讯翻译" => self.cloud_fields(h, provider),
            _ => {
                self.add(
                    h,
                    "Base URL",
                    &format!("/translation/llm_profiles/{provider}/base_url"),
                    Input::Text,
                    false,
                );
                self.add(
                    h,
                    "模型名称",
                    &format!("/translation/llm_profiles/{provider}/model"),
                    Input::Text,
                    false,
                );
                self.add(
                    h,
                    "API Key（可选）",
                    &format!("/llm_profiles/{provider}"),
                    Input::Secret,
                    true,
                );
                self.add(
                    h,
                    "使用 Chat Completions 协议；官方 OpenAI 服务需填写 API Key。",
                    "",
                    Input::Heading,
                    false,
                );
            }
        }
    }
}
