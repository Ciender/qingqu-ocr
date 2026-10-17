//! Dated public service information, never an account balance or a live quote.
use crate::config::Settings;

pub const VERIFIED: &str = "2026-11-01";
pub struct Source {
    pub label: &'static str,
    pub url: &'static str,
}
pub struct ServiceInfo {
    pub title: String,
    pub body: String,
    pub sources: Vec<Source>,
}
const ALI_FREE: &str = "https://help.aliyun.com/zh/ocr/product-overview/free-quota";
const ALI_PAY: &str = "https://help.aliyun.com/zh/ocr/product-overview/pay-as-you-go";
const QWEN_PAY: &str = "https://help.aliyun.com/zh/model-studio/model-pricing";
const QWEN_FREE: &str = "https://help.aliyun.com/zh/model-studio/new-free-quota";
const BAIDU_FREE: &str = "https://ai.baidu.com/ai-doc/OCR/fk3h7xu7h";
const BAIDU_EDU: &str = "https://ai.baidu.com/ai-doc/OCR/Ek3h7xvq7";
const BAIDU_PP: &str = "https://ai.baidu.com/ai-doc/OCR/Imtti0n62";
const BAIDU_RETIRE: &str = "https://ai.baidu.com/support/news?action=detail&id=3264";
const BAIDU_PRICE: &str = "https://ai.baidu.com/ai-doc/OCR/9k3h7xuv6";

fn source(label: &'static str, url: &'static str) -> Source {
    Source { label, url }
}
impl ServiceInfo {
    pub fn for_interface(settings: &Settings, index: usize) -> Self {
        if index == 5 {
            return Self::for_interface(&settings.picture_settings(), 0);
        }
        let mode = match index {
            1 => "表格",
            2 => "竖排",
            3 => "公式",
            4 => return Self::translation(settings),
            _ => &settings.text_ocr_mode,
        };
        let provider = settings.ocr_provider_for(mode);
        let mut info = Self {
            title: format!(
                "{} · 免费额度与计费",
                crate::config::provider_label(provider)
            ),
            body: format!("当前接口：{}\r\n\r\n", settings.ocr_route(mode)),
            sources: Vec::new(),
        };
        match provider {
            "阿里云 OCR" | "阿里云 OCR统一识别" => {
                info.body.push_str(if provider == "阿里云 OCR统一识别" {
                    "每月免费：200 次/月。RecognizeAllText 各识别类型共享额度，不是每种类型各 200 次；与独立通用文字、高精版 API 的额度分开。"
                } else {
                    "每月免费：当前 API 200 次/月；通用文字、高精版、表格、公式等独立 API 分别统计，且与 OCR 统一识别的额度分开。"
                });
                info.body.push_str("\r\n有效期：当月有效，未用完不结转；成功调用才消耗免费额度。\r\n收费：按调用次数计费，超额后抵扣资源包或按量后付费；开通后默认可产生后付费。阶梯单价请查看对应 API 的官方计费表。\r\n凭据：阿里云 AccessKey ID / Secret；需开通相应产品权限。");
                if settings.aliyun_region == "cn-hangzhou" {
                    let advanced = (mode == "表格"
                        && provider == "阿里云 OCR"
                        && settings.aliyun_action(mode) == "RecognizeAdvanced")
                        || mode == "高精度"
                        || mode == "竖排"
                        || (provider == "阿里云 OCR统一识别"
                            && matches!(
                                settings.aliyun_kind(mode),
                                "Advanced" | "HandWriting" | "MultiLang"
                            ));
                    if advanced {
                        info.body.push_str("\r\n杭州首档价格（月调用量不超过 1 万次）：高精版 0.225 元/次；后续按阶梯计费。");
                    } else {
                        info.body.push_str("\r\n杭州首档价格（月调用量不超过 1 万次）：0.0825 元/次；后续按阶梯计费。");
                    }
                }
                if mode == "竖排" {
                    info.body.push_str("\r\n竖排：使用高精版坐标恢复从右至左列序；复杂竖排效果需实测，与该高精版 API 共用额度。");
                }
                if mode == "公式" {
                    info.body.push_str("\r\n公式：专用于印刷体数学公式，返回 LaTeX；图片长宽均须小于 1024 像素。手写及整页混排可另选 Qwen 测试。");
                }
                info.sources = vec![
                    source("官方免费额度", ALI_FREE),
                    source("官方按量价格", ALI_PAY),
                ];
                if mode == "公式" {
                    info.sources.push(source("公式能力与图片限制", "https://help.aliyun.com/zh/ocr/developer-reference/api-ocr-api-2021-07-07-recognizeeduformula"));
                } else if mode == "高精度" || mode == "竖排" {
                    info.sources.push(source("高精版接口文档", "https://help.aliyun.com/zh/ocr/developer-reference/api-ocr-api-2021-07-07-recognizeadvanced"));
                }
            }
            "阿里云 Qwen-OCR" | "阿里云 Qwen-VL" => {
                let profile = if provider == "阿里云 Qwen-OCR" {
                    &settings.qwen_ocr
                } else {
                    &settings.qwen_vl
                };
                let model = profile.model.trim();
                if !profile.base_url.trim().is_empty() {
                    info.body.push_str("免费与收费：当前配置了自定义服务地址，不能套用阿里云官方价格或额度；请核对该服务商的计费规则。\r\n以下链接仅供对照阿里云官方服务。");
                } else if profile.region != "cn-beijing" {
                    info.body.push_str("每月免费：没有固定月赠。当前地域不属于北京新人免费额度适用范围，按该地域及模型价格计费。");
                } else {
                    info.body.push_str(match model {
                        "qwen3.5-ocr" | "qwen-vl-ocr" | "qwen3-vl-plus" => "每月免费：没有固定月赠。符合条件的北京地域账号可有 100 万 Token 新人试用额度，90 天有效，不按月重置；是否已领取及剩余量以控制台为准。",
                        "qwen-vl-ocr-2025-11-20" => "每月免费：没有固定月赠。官方当前价格表中此固定快照无免费额度，请勿沿用 qwen-vl-ocr 别名的试用额度。",
                        _ => "每月免费：没有固定月赠。部分北京地域模型有 90 天新人试用；当前模型的赠送数量与资格请查价格表及控制台，不套用其他模型额度。",
                    });
                    info.body.push_str(match model {
                        "qwen3.5-ocr" => "\r\n北京价格：每百万 Token 输入 0.5 元、输出 2 元。",
                        "qwen-vl-ocr" => "\r\n北京价格：每百万 Token 输入 0.3 元、输出 0.5 元。",
                        "qwen3-vl-plus" => "\r\n北京首档价格（输入不超过 32K Token）：每百万 Token 输入 1 元、输出 10 元；更长输入按官方阶梯价格。",
                        _ => "",
                    });
                }
                info.body.push_str("\r\n收费：按输入与输出 Token 计费，图片也折算为 Token；超出或过期后按模型、地域计费，不是按图片固定收费。\r\n凭据：当前地域的百炼 API Key，与传统 OCR 的 AccessKey 不同。");
                if provider == "阿里云 Qwen-OCR" {
                    info.body.push_str("\r\n能力：公式任务返回 LaTeX；整页文字与公式混排可选文档解析，表格使用表格解析。文字定位任务用于返回坐标，不代表更换了更高精度模型。");
                } else {
                    info.body.push_str("\r\n能力：使用视觉模型及任务提示词识别文字、公式、表格和竖排；效果随模型变化。");
                }
                info.sources = vec![
                    source("官方模型价格（含额度）", QWEN_PAY),
                    source("新人额度与 90 天有效期", QWEN_FREE),
                    if provider == "阿里云 Qwen-OCR" {
                        source(
                            "Qwen-OCR 任务文档",
                            "https://help.aliyun.com/zh/model-studio/qwen-vl-ocr",
                        )
                    } else {
                        source(
                            "Qwen-VL 视觉理解文档",
                            "https://help.aliyun.com/zh/model-studio/vision",
                        )
                    },
                ];
            }
            "百度 OCR" => {
                if mode == "公式" && settings.baidu_formula_endpoint == "formula" {
                    info.body.push_str("接口状态：旧公式识别已列入官方停止升级及下线公告，公告计划停服日期为 2026-04-30。个别账号目前返回成功也不代表长期可用。\r\n免费额度：不将旧接口标为有长期固定月赠；可用性、额度及收费须查账号控制台。可在公式设置中选试卷分析，或改用其他厂商测试。");
                    info.sources.push(source("官方下线公告", BAIDU_RETIRE));
                } else if mode == "公式" {
                    info.body.push_str("免费测试：个人 500 次、企业 1000 次；官方价格页未标为每月重置，不作为长期月赠。\r\n收费：按量后付费 0.16 元/次；资源包另计。");
                    info.sources.push(source("试卷分析额度与价格", BAIDU_EDU));
                } else if mode == "高精度" {
                    info.body.push_str("每月免费：本程序使用高精度含位置版 accurate；个人 500 次/月，企业 1000 次/月。不要套用不含位置版 accurate_basic 的额度。\r\n收费：免费测试资源耗尽后按已开通的计费方式收费，资源包及阶梯单价见官方表。");
                    info.body.push_str("\r\n首档价格（月调用量不超过 5 万次）：0.04 元/次；高精度不含位置版的 0.03 元/次不适用于此接口。");
                } else if mode == "表格" {
                    info.body.push_str("每月免费：表格识别 V2，个人 500 次/月，企业 1000 次/月。\r\n收费：免费测试资源耗尽后按已开通的计费方式收费，单价及资源包见官方表。");
                    info.body.push_str("\r\n按量价格：0.12 元/次，资源包另计。");
                } else if mode == "PP-OCRv6" {
                    info.body.push_str("免费测试：个人 1000 次、企业 2000 次；当前官方页未标为每月发放，不承诺按月重置。\r\n收费：后付费 0.025 元/次；资源包另计。");
                    info.sources.push(source("PP-OCRv6 额度与价格", BAIDU_PP));
                } else {
                    info.body.push_str("每月免费：本程序使用标准含位置版 general；个人 1000 次/月、企业 2000 次/月。竖排与该接口共用额度。\r\n收费：免费测试资源耗尽后按已开通的计费方式收费，价格见官方计费入口。");
                }
                info.body.push_str("\r\n注意：需实名及领取相应测试资源；成功和失败调用均可能消耗免费测试资源。月额度当月有效，不结转。");
                info.sources.push(source("官方免费测试资源", BAIDU_FREE));
                if info.sources.len() < 3 {
                    info.sources
                        .push(source("百度 OCR 官方计费表", BAIDU_PRICE));
                }
            }
            "腾讯云 OCR" => {
                info.body.push_str("免费与收费：按具体 API、资源包与账号资格分别计算，本摘要未收录已核实的固定月赠数字。超额可能按量计费，请在调用前查看官方说明及控制台。\r\n同一厂商的文字、表格接口不应视为共用同一份免费额度。");
                info.sources.push(source(
                    "官方免费资源包说明",
                    "https://cloud.tencent.com/document/product/866/35945",
                ));
            }
            "Mathpix" => {
                info.body.push_str("免费与收费：属于独立的 Mathpix API 服务，按其账号方案计费；不套用阿里云或百度的月额度。具体免费资格、单价和余额请查看 Mathpix 控制台。");
                info.sources.push(source(
                    "Mathpix API 官方价格",
                    "https://mathpix.com/ocr/pricing",
                ));
            }
            _ => info
                .body
                .push_str("当前服务未收录已核实的额度。请查看服务商的账号控制台与计费说明。"),
        }
        let mut setup = Self::setup_sources(settings, index);
        setup.append(&mut info.sources);
        info.sources = setup;
        info
    }
    pub fn setup_sources(settings: &Settings, index: usize) -> Vec<Source> {
        if index == 5 {
            return Self::setup_sources(&settings.picture_settings(), 0);
        }
        let provider = match index {
            1 => &settings.table_provider,
            2 => &settings.vertical_provider,
            3 => &settings.formula_provider,
            4 => &settings.translation.provider,
            _ => &settings.ocr_provider,
        };
        match provider.as_str() {
            "阿里云 Qwen-OCR" | "阿里云 Qwen-VL" => {
                let profile = if provider == "阿里云 Qwen-OCR" {
                    &settings.qwen_ocr
                } else {
                    &settings.qwen_vl
                };
                let key = match profile.region.as_str() {
                    "cn-beijing" => {
                        "https://bailian.console.aliyun.com/cn-beijing/model/settings/api-key"
                    }
                    "ap-southeast-1" => {
                        "https://bailian.console.aliyun.com/ap-southeast-1/model/settings/api-key"
                    }
                    _ => "https://help.aliyun.com/zh/model-studio/get-api-key",
                };
                vec![source("获取百炼 API Key（对应所选地域）", key)]
            }
            "阿里云 OCR" | "阿里云 OCR统一识别" | "阿里云翻译" => vec![
                source(
                    "获取 AccessKey ID / Secret",
                    "https://ram.console.aliyun.com/profile/access-keys",
                ),
                if index == 4 {
                    source("开通机器翻译", "https://mt.console.aliyun.com/")
                } else {
                    source("开通 OCR / 查看用量", "https://ocr.console.aliyun.com/")
                },
            ],
            "百度 OCR" => vec![source(
                "创建 OCR 应用 / 获取 API Key 与 Secret Key",
                "https://console.bce.baidu.com/ai-engine/ocr/overview/index",
            )],
            "百度翻译" => vec![source(
                "获取百度翻译 App ID / 密钥",
                "https://fanyi-api.baidu.com/",
            )],
            "腾讯云 OCR" | "腾讯翻译" => vec![source(
                "获取 SecretId / SecretKey",
                "https://console.cloud.tencent.com/cam/capi",
            )],
            "Mathpix" => vec![source(
                "获取 Mathpix App ID / App Key",
                "https://accounts.mathpix.com/",
            )],
            "谷歌翻译" => vec![source(
                "Google Cloud API 凭据",
                "https://console.cloud.google.com/apis/credentials",
            )],
            "微软翻译" => vec![source(
                "Azure Translator 资源与密钥",
                "https://portal.azure.com/",
            )],
            _ => Vec::new(),
        }
    }

    /// Compact rules appear first so the key links remain easy to reach.
    pub fn reference_text(&self) -> String {
        let mut lines = vec![self.title.clone()];
        for prefixes in [
            &["每月免费：", "免费测试：", "免费额度：", "免费与收费："][..],
            &[
                "北京价格：",
                "北京首档价格",
                "杭州首档价格",
                "首档价格",
                "按量价格：",
            ][..],
            &["收费：", "接口状态："][..],
        ] {
            for line in self.body.lines() {
                if prefixes.iter().any(|prefix| line.starts_with(prefix)) {
                    lines.push(line.to_owned());
                }
            }
        }
        lines.push(format!(
            "资料核对：{VERIFIED}；公开规则摘要，非账号实时余额。"
        ));
        lines.push(String::new());
        lines.push("打开下方官方入口，复制对应密钥填入上方输入框，再点“保存并应用”。".into());
        lines.push("传统阿里云 OCR 填 AccessKey ID / Secret；Qwen 填百炼 API Key；百度 OCR 填应用 API Key / Secret Key。".into());
        for link in &self.sources {
            lines.push(format!("{}\r\n{}", link.label, link.url));
        }
        lines.push(format!(
            "\r\n资料核对：{VERIFIED}；公开规则摘要，非账号实时余额。以官方当前规则和控制台为准。"
        ));
        lines.join("\r\n")
    }
    fn translation(settings: &Settings) -> Self {
        let mut sources = Self::setup_sources(settings, 4);
        sources.extend(match settings.translation.provider.as_str() {
            "阿里云翻译" => vec![source(
                "阿里云机器翻译计费",
                "https://help.aliyun.com/zh/machine-translation/product-overview/billing-overview",
            )],
            "百度翻译" => vec![source("百度翻译开放平台", "https://fanyi-api.baidu.com/")],
            "谷歌翻译" => vec![source(
                "Google Cloud Translation 价格",
                "https://cloud.google.com/translate/pricing",
            )],
            "微软翻译" => vec![source(
                "Microsoft Translator 价格",
                "https://azure.microsoft.com/pricing/details/cognitive-services/translator/",
            )],
            "腾讯翻译" => vec![source(
                "腾讯机器翻译官方文档",
                "https://cloud.tencent.com/document/product/551",
            )],
            _ => Vec::new(),
        });
        Self {
            title: format!("{} · 免费额度与计费", settings.translation.provider),
            body: "翻译与 OCR 使用不同服务及计费规则，OCR 的每月免费次数不适用于翻译。\r\n\r\n网页翻译入口受服务条款、限流和网络影响，不作为稳定的官方免费 API。\r\n官方云翻译通常按字符量或所选套餐计费；自定义 LLM 按配置服务商与模型计费，免费额度和余额须在该服务商控制台确认。".into(),
            sources,
        }
    }
}
