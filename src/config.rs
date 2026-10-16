mod bindings;
mod languages;
mod recognition;
pub use bindings::{interface_mode_path, API_SERVICES, TASK_NAMES};
pub use languages::{
    ocr_language_choices, ocr_language_value, TRANSLATION_LANGUAGES, TRANSLATION_TARGETS,
};

use serde::{Deserialize, Serialize};
use std::{collections::BTreeMap, fs, path::PathBuf};
use windows_sys::Win32::{
    Foundation::LocalFree,
    Security::Cryptography::{
        CryptProtectData, CryptUnprotectData, CRYPTPROTECT_UI_FORBIDDEN, CRYPT_INTEGER_BLOB,
    },
};

pub const OCR_PROVIDERS: &[&str] = &[
    "百度 OCR",
    "腾讯云 OCR",
    "阿里云 OCR",
    "阿里云 Qwen-OCR",
    "阿里云 Qwen-VL",
    "阿里云 OCR统一识别",
];
pub const QWEN_OCR_MODELS: &[&str] = &["qwen3.5-ocr", "qwen-vl-ocr", "qwen-vl-ocr-2025-11-20"];
pub const QWEN_VL_MODELS: &[&str] = &[
    "qwen3-vl-plus",
    "qwen3-vl-flash",
    "qwen-vl-max",
    "qwen-vl-plus",
    "qwen3.8-max",
    "qwen3.8-flash",
    "qwen3.7-plus",
    "qwen3.7-flash",
    "qwen3.6-plus",
    "qwen3.6-flash",
    "qwen3.5-plus",
];
pub const TRANSLATION_PROVIDERS: &[&str] = &[
    "谷歌翻译",
    "微软翻译",
    "Index Translate",
    "OpenAI 兼容",
    "百度翻译",
    "阿里云翻译",
    "腾讯翻译",
];
pub const INTERFACE_PATHS: &[&str] = &[
    "/ocr_provider",
    "/table_provider",
    "/vertical_provider",
    "/formula_provider",
    "/translation/provider",
    "/picture_ocr_provider",
];
pub fn provider_label(provider: &str) -> &str {
    match provider {
        "阿里云 OCR" => "阿里云 OCR（独立接口）",
        _ => provider,
    }
}
pub fn interface_choices(index: usize) -> Vec<(&'static str, &'static str)> {
    match index {
        5 => std::iter::once(("", "跟随文字配置"))
            .chain(OCR_PROVIDERS.iter().map(|&s| (s, provider_label(s))))
            .collect(),
        3 => vec![
            ("百度 OCR", "百度 OCR"),
            ("阿里云 OCR", "阿里云 OCR（独立接口）"),
            ("Mathpix", "Mathpix"),
            ("阿里云 Qwen-OCR", "阿里云 Qwen-OCR"),
            ("阿里云 Qwen-VL", "阿里云 Qwen-VL"),
        ],
        4 => TRANSLATION_PROVIDERS.iter().map(|&s| (s, s)).collect(),
        _ => OCR_PROVIDERS
            .iter()
            .map(|&s| (s, provider_label(s)))
            .collect(),
    }
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
    pub auto_copy: bool,
    pub floating_button: bool,
    pub picture_info: bool,
    pub close_to_tray: bool,
    pub hotkey: String,
    pub correct_orientation: bool,
    pub dark: bool,
    pub palette: String,
    pub default_topmost: bool,
    pub auto_translate: bool,
    #[serde(default)]
    pub translation_ui_version: u8,
    pub show_translation: bool,
    pub startup: bool,
    pub hotkeys: Vec<Binding>,
    pub ocr_mode: String,
    /// Persisted text algorithm. Specialized toolbar actions take an isolated
    /// settings snapshot and never replace this selection.
    pub text_ocr_mode: String,
    pub ocr_language: String,
    /// Empty provider follows text; the picture's endpoint/language are independent.
    pub picture_ocr_provider: String,
    pub picture_ocr_mode: String,
    pub picture_ocr_language: String,
    /// Versioned migration from the old provider/mode controls.
    #[serde(default)]
    pub recognition_bindings_version: u8,
    pub table_ocr_mode: String,
    pub vertical_ocr_mode: String,
    pub formula_ocr_mode: String,
    pub translation: Translation,
    pub unlimited_url: String,
    pub unlimited_model: String,
    pub formula_url: String,
    pub format_options: crate::text::FormatOptions,
    pub editor_font_size: f32,
    pub preview_before_apply: bool,
    pub wrap_columns: usize,
    pub first_line_indent: usize,
    pub split_delimiter: String,
    pub processing: crate::text::ProcessingModes,
    pub formula_provider: String,
    pub baidu_formula_endpoint: String,
    pub ocr_provider: String,
    pub table_provider: String,
    pub vertical_provider: String,
    pub tencent_region: String,
    pub aliyun_region: String,
    pub aliyun_unified_type: String,
    pub qwen_ocr: QwenProfile,
    pub qwen_vl: QwenProfile,
}
impl Settings {
    /// Bring legacy “off” values into the toolbar's explicit default modes.
    pub fn normalize_processing_modes(&mut self) {
        if !matches!(self.processing.layout.as_str(), "smart" | "merge" | "split") {
            self.processing.layout = "smart".into();
        }
        if !matches!(self.processing.punctuation.as_str(), "chinese" | "english") {
            self.processing.punctuation = "chinese".into();
        }
    }
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct QwenProfile {
    pub region: String,
    pub workspace: String,
    pub base_url: String,
    pub protocol: String,
    pub model: String,
    pub task: String,
    pub prompt: String,
}
impl Default for QwenProfile {
    fn default() -> Self {
        Self {
            region: "cn-beijing".into(),
            workspace: String::new(),
            base_url: String::new(),
            protocol: "dashscope".into(),
            model: "qwen3.5-ocr".into(),
            task: "auto".into(),
            prompt: String::new(),
        }
    }
}
impl QwenProfile {
    pub fn vl() -> Self {
        Self {
            protocol: "chat".into(),
            model: "qwen3-vl-plus".into(),
            ..Self::default()
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct Binding {
    pub action: String,
    pub key: String,
}

pub const ACTIONS: &[(&str, &str)] = &[
    ("capture", "区域截图"),
    ("clipboard", "当前剪贴板"),
    ("show", "显示主窗口"),
    ("fullscreen", "全屏识别"),
    ("table", "表格截图"),
    ("formula", "公式截图"),
    ("translate", "翻译当前原文"),
    ("format", "智能排版"),
];

#[derive(Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Translation {
    pub provider: String,
    pub base_url: String,
    pub model: String,
    pub source: String,
    pub target: String,
    pub stream: bool,
    pub prompt: String,
    pub glossary: String,
    pub google_mode: String,
    pub microsoft_mode: String,
    pub microsoft_region: String,
    pub tencent_region: String,
    pub tencent_project_id: usize,
    pub llm_profiles: BTreeMap<String, LlmProfile>,
}
#[derive(Clone, Serialize, Deserialize)]
pub struct LlmProfile {
    pub base_url: String,
    pub model: String,
}
impl Translation {
    pub fn ensure_profiles(&mut self, keys: &mut Credentials) {
        if matches!(self.provider.as_str(), "Index Translate" | "OpenAI 兼容")
            && !self.llm_profiles.contains_key(&self.provider)
        {
            self.llm_profiles.insert(
                self.provider.clone(),
                LlmProfile {
                    base_url: self.base_url.clone(),
                    model: self.model.clone(),
                },
            );
            keys.llm_profiles
                .entry(self.provider.clone())
                .or_insert_with(|| keys.llm_key.clone());
        }
        self.llm_profiles
            .entry("Index Translate".into())
            .or_insert_with(|| LlmProfile {
                base_url: "https://index-translate.bilibili.com/v1".into(),
                model: "Index-Translate-35B-A3B".into(),
            });
        self.llm_profiles
            .entry("OpenAI 兼容".into())
            .or_insert_with(|| LlmProfile {
                base_url: "https://api.openai.com/v1".into(),
                model: String::new(),
            });
        for name in ["Index Translate", "OpenAI 兼容"] {
            keys.llm_profiles.entry(name.into()).or_default();
        }
    }
}
impl Default for Translation {
    fn default() -> Self {
        Self {
            provider: "Index Translate".into(),
            base_url: "https://index-translate.bilibili.com/v1".into(),
            model: "Index-Translate-35B-A3B".into(),
            source: "auto".into(),
            target: "英语".into(),
            stream: true,
            prompt: "只输出译文，保留原文的段落、列表和公式，不添加解释或思考内容。".into(),
            glossary: String::new(),
            google_mode: "web".into(),
            microsoft_mode: "web".into(),
            microsoft_region: "global".into(),
            tencent_region: "ap-beijing".into(),
            tencent_project_id: 0,
            llm_profiles: BTreeMap::new(),
        }
    }
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            auto_copy: true,
            floating_button: true,
            picture_info: false,
            close_to_tray: true,
            hotkey: "Ctrl + Shift + O".into(),
            correct_orientation: false,
            dark: false,
            palette: "vscode".into(),
            default_topmost: false,
            auto_translate: true,
            translation_ui_version: 1,
            show_translation: true,
            startup: false,
            hotkeys: ACTIONS
                .iter()
                .map(|(action, _)| Binding {
                    action: (*action).into(),
                    key: match *action {
                        "capture" => "Ctrl+Shift+O",
                        "clipboard" => "Alt+V",
                        "show" => "Ctrl+Shift+Q",
                        _ => "",
                    }
                    .into(),
                })
                .collect(),
            ocr_mode: "高精度".into(),
            // Empty is intentional for serde backward migration. Main/load
            // normalize it from the old ocr_mode field before use.
            text_ocr_mode: String::new(),
            ocr_language: "auto_detect".into(),
            picture_ocr_provider: String::new(),
            picture_ocr_mode: "高精度".into(),
            picture_ocr_language: "auto_detect".into(),
            recognition_bindings_version: 1,
            table_ocr_mode: String::new(),
            vertical_ocr_mode: String::new(),
            formula_ocr_mode: String::new(),
            translation: Translation::default(),
            unlimited_url: String::new(),
            unlimited_model: String::new(),
            formula_url: "https://api.mathpix.com/v3/text".into(),
            format_options: crate::text::FormatOptions::default(),
            editor_font_size: 14.,
            preview_before_apply: false,
            wrap_columns: 60,
            first_line_indent: 2,
            split_delimiter: ";".into(),
            processing: crate::text::ProcessingModes {
                layout: "smart".into(),
                punctuation: "chinese".into(),
                chinese: String::new(),
            },
            formula_provider: "百度 OCR".into(),
            baidu_formula_endpoint: "formula".into(),
            ocr_provider: "百度 OCR".into(),
            table_provider: "百度 OCR".into(),
            vertical_provider: "百度 OCR".into(),
            tencent_region: "ap-beijing".into(),
            aliyun_region: "cn-hangzhou".into(),
            aliyun_unified_type: "Advanced".into(),
            qwen_ocr: QwenProfile::default(),
            qwen_vl: QwenProfile::vl(),
        }
    }
}

#[derive(Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct Credentials {
    pub api_key: String,
    pub secret_key: String,
    pub llm_key: String,
    pub translate_appid: String,
    pub translate_secret: String,
    pub unlimited_key: String,
    pub formula_appid: String,
    pub formula_key: String,
    pub tencent_secret_id: String,
    pub tencent_secret_key: String,
    pub aliyun_access_key_id: String,
    pub aliyun_access_key_secret: String,
    pub google_key: String,
    pub microsoft_key: String,
    pub llm_profiles: BTreeMap<String, String>,
    pub qwen_ocr_key: String,
    pub qwen_vl_key: String,
}

impl Credentials {
    pub fn ready(&self) -> bool {
        !self.api_key.trim().is_empty() && !self.secret_key.trim().is_empty()
    }
}

pub fn directory() -> PathBuf {
    if let Some(path) = std::env::var_os("QINGQU_DATA_DIR") {
        return PathBuf::from(path);
    }
    PathBuf::from(std::env::var_os("LOCALAPPDATA").unwrap_or_else(|| ".".into())).join("QingquOCR")
}

pub fn load() -> (Settings, Credentials, Option<String>) {
    let dir = directory();
    let mut warning = None;
    let mut settings: Settings = match fs::read(dir.join("settings.json")) {
        Ok(bytes) => serde_json::from_slice(&bytes).unwrap_or_else(|_| {
            warning = Some("设置文件无法读取，已使用默认设置。".into());
            Settings::default()
        }),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Settings::default(),
        Err(_) => {
            warning = Some("无法读取本地设置。".into());
            Settings::default()
        }
    };
    settings.normalize_ocr_modes();
    settings.normalize_ocr_language();
    settings.normalize_processing_modes();
    let mut credentials: Credentials = match fs::read(dir.join("credentials.bin")) {
        Ok(bytes) => match unprotect(&bytes).and_then(|plain| {
            serde_json::from_slice(&plain).map_err(|_| "密钥数据格式错误。".into())
        }) {
            Ok(keys) => keys,
            Err(_) => {
                warning = Some("无法解密已保存的密钥，请在设置中重新填写。".into());
                Credentials::default()
            }
        },
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Credentials::default(),
        Err(_) => {
            warning = Some("无法读取已保存的密钥。".into());
            Credentials::default()
        }
    };
    settings.translation.ensure_profiles(&mut credentials);
    (settings, credentials, warning)
}

pub fn save(settings: &Settings, credentials: &Credentials) -> Result<(), String> {
    let dir = directory();
    fs::create_dir_all(&dir).map_err(|_| "无法创建设置目录。")?;
    let encoded = serde_json::to_vec(credentials).map_err(|_| "密钥序列化失败。")?;
    let protected = protect(&encoded)?;
    let options = serde_json::to_vec_pretty(settings).map_err(|_| "设置序列化失败。")?;
    atomic_write(&dir.join("credentials.bin"), &protected)?;
    atomic_write(&dir.join("settings.json"), &options)?;
    Ok(())
}

pub fn save_preferences(settings: &Settings) -> Result<(), String> {
    let dir = directory();
    fs::create_dir_all(&dir).map_err(|_| "无法创建设置目录。")?;
    let bytes = serde_json::to_vec_pretty(settings).map_err(|_| "设置序列化失败。")?;
    atomic_write(&dir.join("settings.json"), &bytes)
}

fn atomic_write(path: &std::path::Path, bytes: &[u8]) -> Result<(), String> {
    let temp = path.with_extension("tmp");
    fs::write(&temp, bytes).map_err(|_| "写入设置失败。")?;
    fs::rename(&temp, path).map_err(|_| "保存设置失败，请检查文件权限。".into())
}

fn crypt(bytes: &[u8], encrypt: bool) -> Result<Vec<u8>, String> {
    let input = CRYPT_INTEGER_BLOB {
        cbData: bytes.len().try_into().map_err(|_| "密钥数据过大。")?,
        pbData: bytes.as_ptr() as *mut u8,
    };
    let mut output: CRYPT_INTEGER_BLOB = unsafe { std::mem::zeroed() };
    let success = unsafe {
        if encrypt {
            CryptProtectData(
                &input,
                std::ptr::null(),
                std::ptr::null(),
                std::ptr::null(),
                std::ptr::null(),
                CRYPTPROTECT_UI_FORBIDDEN,
                &mut output,
            )
        } else {
            CryptUnprotectData(
                &input,
                std::ptr::null_mut(),
                std::ptr::null(),
                std::ptr::null(),
                std::ptr::null(),
                CRYPTPROTECT_UI_FORBIDDEN,
                &mut output,
            )
        }
    };
    if success == 0 {
        return Err("Windows 密钥加密或解密失败。".into());
    }
    let result =
        unsafe { std::slice::from_raw_parts(output.pbData, output.cbData as usize).to_vec() };
    unsafe { LocalFree(output.pbData.cast()) };
    Ok(result)
}

fn protect(bytes: &[u8]) -> Result<Vec<u8>, String> {
    crypt(bytes, true)
}

fn unprotect(bytes: &[u8]) -> Result<Vec<u8>, String> {
    crypt(bytes, false)
}
