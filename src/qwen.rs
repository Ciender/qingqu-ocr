//! Alibaba Model Studio OCR and vision; separate from signed traditional OCR.
use crate::{
    config::{Credentials, QwenProfile, Settings},
    ocr::{Cell, OcrResult, Table},
    text::{recover_layout, vertical_text, PositionedLine},
};
use async_openai::{config::OpenAIConfig, Client};
use base64::{engine::general_purpose::STANDARD, Engine};
use image::RgbaImage;
use regex::Regex;
use serde_json::{json, Value};
use std::time::{Duration, Instant};

pub fn endpoint(profile: &QwenProfile) -> Result<String, String> {
    let root = if !profile.base_url.trim().is_empty() {
        crate::translate::base_url(&profile.base_url)?
    } else {
        match profile.region.as_str() {
            "cn-beijing" | "ap-southeast-1" | "us-east-1"
                if !profile.workspace.trim().is_empty() =>
            {
                let id = profile.workspace.trim();
                if !id.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'-') {
                    return Err("工作空间 ID 只能包含英文字母、数字和短横线。".into());
                }
                format!("https://{id}.{}.maas.aliyuncs.com", profile.region)
            }
            "cn-beijing" => "https://dashscope.aliyuncs.com".into(),
            "ap-southeast-1" => "https://dashscope-intl.aliyuncs.com".into(),
            "us-east-1" => "https://dashscope-us.aliyuncs.com".into(),
            _ => return Err("请选择百炼地域，或填写实际服务地址。".into()),
        }
    };
    let path = reqwest::Url::parse(&root)
        .map_err(|_| "百炼服务地址无效。")?
        .path()
        .trim_end_matches('/')
        .to_owned();
    if profile.protocol == "dashscope" {
        let suffix = "/api/v1/services/aigc/multimodal-generation/generation";
        if path.is_empty() {
            Ok(format!("{root}{suffix}"))
        } else if path == "/api/v1" {
            Ok(format!(
                "{root}/services/aigc/multimodal-generation/generation"
            ))
        } else if path.ends_with("/generation") {
            Ok(root)
        } else {
            Err("DashScope 地址应为服务根地址或完整 generation 地址。".into())
        }
    } else if profile.protocol == "chat" {
        if path.is_empty() {
            Ok(format!("{root}/compatible-mode/v1"))
        } else {
            Ok(root)
        }
    } else {
        Err("暂不支持这个百炼调用协议。".into())
    }
}
fn task<'a>(mode: &str, profile: &'a QwenProfile) -> Result<&'a str, String> {
    let name = if profile.task == "auto" {
        match mode {
            "表格" => "table_parsing",
            "公式" => "formula_recognition",
            "高精度" | "竖排" => "advanced_recognition",
            "通用多语言" => "multi_lan",
            _ => "text_recognition",
        }
    } else {
        profile.task.as_str()
    };
    if matches!(
        name,
        "text_recognition"
            | "advanced_recognition"
            | "document_parsing"
            | "table_parsing"
            | "formula_recognition"
            | "multi_lan"
    ) {
        Ok(name)
    } else {
        Err("请选择支持的 Qwen-OCR 任务。".into())
    }
}
fn prompt(mode: &str, extra: &str) -> String {
    let base = match mode {
        "表格" => "仅识别图片中的表格，严格输出 JSON，不要代码围栏或解释。结构为 {\"tables\":[{\"cells\":[{\"row\":0,\"col\":0,\"rowspan\":1,\"colspan\":1,\"text\":\"单元格原文\"}]}]}。行列从0开始，保留合并跨度、空单元格，不虚构内容。",
        "公式" => "识别图片里的数学公式，仅输出 LaTeX，不解释、不求解、不添加代码围栏。",
        "竖排" => "识别竖排文字，按从右至左的列序、每列从上至下输出，只返回原文，保留段落，不解释。",
        _ => "提取图片原文，保留语言、段落、标题及列表，合并段内视觉折行；不要翻译、解释或添加不存在的内容。",
    };
    if extra.trim().is_empty() {
        base.into()
    } else {
        format!("{base}\n{}", extra.trim())
    }
}
fn payload(mode: &str, profile: &QwenProfile, data_url: &str) -> Result<Value, String> {
    if profile.model.trim().is_empty() {
        return Err("请填写百炼模型名称。".into());
    }
    if profile.protocol == "dashscope" {
        Ok(
            json!({"model":profile.model.trim(),"input":{"messages":[{"role":"user","content":[{"image":data_url}]}]},"parameters":{"ocr_options":{"task":task(mode,profile)?}}}),
        )
    } else {
        Ok(
            json!({"model":profile.model.trim(),"messages":[{"role":"user","content":[{"type":"image_url","image_url":{"url":data_url}},{"type":"text","text":prompt(mode,&profile.prompt)}]}],"stream":false}),
        )
    }
}
pub fn recognize(
    image: RgbaImage,
    settings: &Settings,
    keys: &Credentials,
) -> Result<OcrResult, String> {
    let ocr = settings.ocr_provider_for(&settings.ocr_mode) == "阿里云 Qwen-OCR";
    let (profile, key) = if ocr {
        (&settings.qwen_ocr, keys.qwen_ocr_key.trim())
    } else {
        (&settings.qwen_vl, keys.qwen_vl_key.trim())
    };
    if key.is_empty() {
        return Err("请填写当前地域的百炼 API Key（不是传统 OCR 的 AccessKey）。".into());
    }
    let mut profile = profile.clone();
    profile.task = settings.qwen_task_for(&settings.ocr_mode).into();
    let start = Instant::now();
    let bytes = crate::ocr::input::png(image, crate::ocr::input::Policy::standard())?;
    let image = format!("data:image/png;base64,{}", STANDARD.encode(bytes));
    let response = send(
        &profile,
        key,
        payload(&settings.ocr_mode, &profile, &image)?,
    )?;
    let (mut text, tables) = parse(&response, settings.ocr_mode == "表格")?;
    let mut layout = None;
    if let Some(lines) = positioned(&response) {
        if settings.ocr_mode == "竖排" {
            text = vertical_text(&lines);
        } else {
            layout = Some(recover_layout(&lines));
            text = lines
                .iter()
                .map(|line| line.text.as_str())
                .collect::<Vec<_>>()
                .join("\n");
        }
    }
    Ok(OcrResult {
        lines: text.lines().count(),
        text,
        tables,
        layout,
        elapsed: start.elapsed(),
    })
}
fn send(profile: &QwenProfile, key: &str, body: Value) -> Result<Value, String> {
    let url = endpoint(profile)?;
    reqwest::header::HeaderValue::from_str(&format!("Bearer {key}"))
        .map_err(|_| "百炼 API Key 含无效字符。")?;
    if profile.protocol == "dashscope" {
        let client = reqwest::blocking::Client::builder()
            .connect_timeout(Duration::from_secs(12))
            .timeout(Duration::from_secs(120))
            .redirect(reqwest::redirect::Policy::none())
            .build()
            .map_err(|_| "无法初始化百炼连接。")?;
        let response = client
            .post(url)
            .bearer_auth(key)
            .json(&body)
            .send()
            .map_err(|_| "百炼识别连接失败或超时。")?;
        if !response.status().is_success() {
            return Err(format!(
                "百炼识别 HTTP {}，请核对地域、模型、API Key、权限和额度。",
                response.status().as_u16()
            ));
        }
        response
            .json()
            .map_err(|_| "百炼响应 JSON 无法解析。".into())
    } else {
        let config = OpenAIConfig::new()
            .with_api_base(url)
            .with_api_key(key)
            .with_org_id("")
            .with_project_id("");
        let client = reqwest::Client::builder()
            .connect_timeout(Duration::from_secs(12))
            .timeout(Duration::from_secs(120))
            .redirect(reqwest::redirect::Policy::none())
            .build()
            .map_err(|_| "无法初始化百炼视觉连接。")?;
        let client = Client::with_config(config).with_http_client(client);
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .map_err(|_| "无法启动百炼视觉任务。")?;
        runtime
            .block_on(async { client.chat().create_byot::<_, Value>(body).await })
            .map_err(|_| "百炼视觉调用失败，请核对地域、模型、API Key 和额度。".into())
    }
}
fn extract(value: &Value) -> Option<String> {
    if let Some(text) = value.as_str() {
        return Some(text.into());
    }
    if let Some(array) = value.as_array() {
        let parts = array.iter().filter_map(extract).collect::<Vec<_>>();
        return if parts.is_empty() {
            None
        } else {
            Some(parts.join("\n"))
        };
    }
    for name in ["text", "content", "ocr_result", "words_info"] {
        if let Some(text) = value.get(name).and_then(extract) {
            return Some(text);
        }
    }
    None
}
fn positioned(response: &Value) -> Option<Vec<PositionedLine>> {
    let content = response
        .pointer("/output/choices/0/message/content")?
        .as_array()?;
    let mut lines = Vec::new();
    for part in content {
        let Some(items) = part
            .pointer("/ocr_result/words_info")
            .and_then(Value::as_array)
        else {
            continue;
        };
        for item in items {
            let text = item.get("text")?.as_str()?;
            if let Some(rect) = item.get("rotate_rect").and_then(Value::as_array) {
                if rect.len() != 5 {
                    return None;
                }
                let values = rect.iter().map(Value::as_f64).collect::<Option<Vec<_>>>()?;
                if !values.iter().all(|v| v.is_finite()) || values[2] <= 0. || values[3] <= 0. {
                    return None;
                }
                let angle = values[4].to_radians();
                let width = values[2] * angle.cos().abs() + values[3] * angle.sin().abs();
                let height = values[2] * angle.sin().abs() + values[3] * angle.cos().abs();
                lines.push(PositionedLine {
                    text: text.into(),
                    x: values[0] - width / 2.,
                    y: values[1] - height / 2.,
                    width,
                    height,
                });
                continue;
            }
            let points = item.get("location")?.as_array()?;
            if points.len() != 8 {
                return None;
            }
            let coordinates = points
                .iter()
                .map(Value::as_f64)
                .collect::<Option<Vec<_>>>()?;
            if !coordinates.iter().all(|n| n.is_finite()) {
                return None;
            }
            let x = coordinates.iter().step_by(2).copied().reduce(f64::min)?;
            let y = coordinates
                .iter()
                .skip(1)
                .step_by(2)
                .copied()
                .reduce(f64::min)?;
            let right = coordinates.iter().step_by(2).copied().reduce(f64::max)?;
            let bottom = coordinates
                .iter()
                .skip(1)
                .step_by(2)
                .copied()
                .reduce(f64::max)?;
            lines.push(PositionedLine {
                text: text.into(),
                x,
                y,
                width: (right - x).max(1.),
                height: (bottom - y).max(1.),
            });
        }
    }
    (!lines.is_empty()).then_some(lines)
}
pub(crate) fn parse(response: &Value, table: bool) -> Result<(String, Vec<Table>), String> {
    if response.get("error").is_some() || response.get("code").is_some() {
        return Err("百炼返回识别错误，请检查模型、任务和权限。".into());
    }
    let content = response
        .pointer("/output/choices/0/message/content")
        .or_else(|| response.pointer("/choices/0/message/content"))
        .ok_or("百炼响应缺少识别内容。")?;
    let text = extract(content).ok_or("百炼未返回文字结果。")?;
    if text.len() > 4 * 1024 * 1024 {
        return Err("识别结果超过 4 MB。".into());
    }
    let tables = if table {
        parse_tables(&text)?
    } else {
        Vec::new()
    };
    if tables.is_empty() {
        Ok((text, tables))
    } else {
        Ok((
            tables
                .iter()
                .map(Table::document_text)
                .collect::<Vec<_>>()
                .join("\n\n"),
            tables,
        ))
    }
}
fn add(
    table: &mut Table,
    row: usize,
    col: usize,
    rowspan: usize,
    colspan: usize,
    text: String,
) -> Result<(), String> {
    let end_row = row.checked_add(rowspan).ok_or("表格行溢出。")?;
    let end_col = col.checked_add(colspan).ok_or("表格列溢出。")?;
    if rowspan == 0 || colspan == 0 || end_row > 5000 || end_col > 500 {
        return Err("模型表格跨度无效或过大。".into());
    }
    if table.cells.iter().any(|c| {
        row < c.row + c.rowspan && end_row > c.row && col < c.col + c.colspan && end_col > c.col
    }) {
        return Err("模型返回了重叠的单元格。".into());
    }
    table.rows = table.rows.max(end_row);
    table.cols = table.cols.max(end_col);
    if table.rows.saturating_mul(table.cols) > 100_000 {
        return Err("表格超过 10 万格。".into());
    }
    table.cells.push(Cell {
        row,
        col,
        rowspan,
        colspan,
        text,
    });
    Ok(())
}
fn parse_tables(text: &str) -> Result<Vec<Table>, String> {
    let clean = text
        .trim()
        .strip_prefix("```json")
        .or_else(|| text.trim().strip_prefix("```html"))
        .unwrap_or(text.trim())
        .trim_end_matches("```")
        .trim();
    if let Ok(value) = serde_json::from_str::<Value>(clean) {
        if let Some(entries) = value.get("tables").and_then(Value::as_array) {
            let mut tables = Vec::new();
            for entry in entries {
                let mut table = Table::default();
                for cell in entry
                    .get("cells")
                    .and_then(Value::as_array)
                    .ok_or("模型表格缺少 cells。")?
                {
                    let number = |key: &str, default: Option<usize>| {
                        cell.get(key)
                            .and_then(Value::as_u64)
                            .and_then(|n| usize::try_from(n).ok())
                            .or(default)
                            .ok_or_else(|| format!("单元格缺少有效的 {key}。"))
                    };
                    add(
                        &mut table,
                        number("row", None)?,
                        number("col", None)?,
                        number("rowspan", Some(1))?,
                        number("colspan", Some(1))?,
                        cell.get("text")
                            .and_then(Value::as_str)
                            .ok_or("单元格文字格式错误。")?
                            .into(),
                    )?;
                }
                tables.push(table);
            }
            return Ok(tables);
        }
    }
    let table_re = Regex::new(r"(?is)<table\b[^>]*>(.*?)</table>").unwrap();
    let row_re = Regex::new(r"(?is)<tr\b[^>]*>(.*?)</tr>").unwrap();
    let cell_re = Regex::new(r"(?is)<t[dh]\b([^>]*)>(.*?)</t[dh]>").unwrap();
    let attr_re = Regex::new(r#"(?i)(rowspan|colspan)\s*=\s*["']?(\d+)"#).unwrap();
    let tag_re = Regex::new(r"(?is)<[^>]+>").unwrap();
    let mut tables = Vec::new();
    for entry in table_re.captures_iter(clean) {
        let mut table = Table::default();
        for (row, line) in row_re.captures_iter(&entry[1]).enumerate() {
            let mut col = 0;
            for cell in cell_re.captures_iter(&line[1]) {
                while table.cells.iter().any(|c| {
                    row >= c.row
                        && row < c.row + c.rowspan
                        && col >= c.col
                        && col < c.col + c.colspan
                }) {
                    col += 1;
                }
                let mut rows = 1;
                let mut cols = 1;
                for attribute in attr_re.captures_iter(&cell[1]) {
                    let n = attribute[2]
                        .parse::<usize>()
                        .map_err(|_| "HTML 表格跨度无效。")?;
                    if attribute[1].eq_ignore_ascii_case("rowspan") {
                        rows = n;
                    } else {
                        cols = n;
                    }
                }
                let raw = tag_re.replace_all(&cell[2], "");
                let text = raw
                    .replace("&lt;", "<")
                    .replace("&gt;", ">")
                    .replace("&quot;", "\"")
                    .replace("&#39;", "'")
                    .replace("&amp;", "&");
                add(&mut table, row, col, rows, cols, text)?;
                col += cols;
            }
        }
        if !table.cells.is_empty() {
            tables.push(table);
        }
    }
    Ok(tables)
}
