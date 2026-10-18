use crate::{
    cloud,
    config::{Credentials, Settings},
    ocr::{Cell, OcrResult, Table},
    text::{
        paragraph_groups_from_texts, recover_layout, recover_layout_with_hints, vertical_text,
        PositionedLine,
    },
};
use base64::{engine::general_purpose::STANDARD, Engine};
use image::RgbaImage;
use serde_json::{json, Value};
use std::time::Instant;
mod table;

pub fn validate(settings: &Settings, keys: &Credentials) -> Result<(), String> {
    let ready = match settings.ocr_provider_for(&settings.ocr_mode) {
        "百度 OCR" => keys.ready(),
        "腾讯云 OCR" => {
            !keys.tencent_secret_id.trim().is_empty() && !keys.tencent_secret_key.trim().is_empty()
        }
        "阿里云 OCR" => {
            !keys.aliyun_access_key_id.trim().is_empty()
                && !keys.aliyun_access_key_secret.trim().is_empty()
        }
        "阿里云 OCR统一识别" => {
            !keys.aliyun_access_key_id.trim().is_empty()
                && !keys.aliyun_access_key_secret.trim().is_empty()
        }
        "阿里云 Qwen-OCR" => !keys.qwen_ocr_key.trim().is_empty(),
        "阿里云 Qwen-VL" => !keys.qwen_vl_key.trim().is_empty(),
        "Mathpix" => !keys.formula_appid.trim().is_empty() && !keys.formula_key.trim().is_empty(),
        _ => return Err("请选择已支持的 OCR 提供商。".into()),
    };
    if ready {
        Ok(())
    } else {
        Err(format!(
            "请配置 {} 的凭据。",
            settings.ocr_provider_for(&settings.ocr_mode)
        ))
    }
}
pub fn recognize(
    image: RgbaImage,
    settings: &Settings,
    keys: &Credentials,
) -> Result<OcrResult, String> {
    recognize_recorded(image, settings, keys, None)
}

/// Explicit diagnostics can retain the service response for offline replay.
pub fn recognize_recorded(
    image: RgbaImage,
    settings: &Settings,
    keys: &Credentials,
    response_path: Option<&std::path::Path>,
) -> Result<OcrResult, String> {
    validate(settings, keys)?;
    let start = Instant::now();
    let provider = settings.ocr_provider_for(&settings.ocr_mode);
    let line_less = settings.ocr_mode == "表格" && !has_vertical_rules(&image);
    let mut policy = crate::ocr::input::Policy::standard();
    if provider == "腾讯云 OCR" {
        policy.min_edge = 20;
    }
    if provider == "阿里云 OCR" && settings.ocr_mode == "公式" {
        policy.max_edge = 1023;
    }
    let bytes = crate::ocr::input::png(image, policy)?;
    let (text, layout, tables) = match provider {
        "腾讯云 OCR" => {
            if bytes.len() > 7_500_000 {
                return Err("图片过大，请缩小选区后使用腾讯云 OCR。".into());
            }
            let action = match settings.ocr_mode.as_str() {
                "表格" => "RecognizeTableAccurateOCR",
                "公式" => return Err("腾讯旧公式接口已发布下线提示，请选择百度或阿里云。".into()),
                "高精度" | "竖排" => "GeneralAccurateOCR",
                _ => "GeneralBasicOCR",
            };
            let response = cloud::tencent(
                "ocr",
                action,
                &settings.tencent_region,
                &tencent_payload(
                    STANDARD.encode(bytes),
                    action,
                    settings.ocr_language_for(&settings.ocr_mode),
                ),
                keys,
            )?;
            record_response(&response, response_path)?;
            parse_tencent(
                &response,
                settings.ocr_mode == "表格",
                settings.ocr_mode == "竖排",
            )?
        }
        "阿里云 OCR" | "阿里云 OCR统一识别" => {
            if bytes.len() > 10 * 1024 * 1024 {
                return Err("阿里云图片超过 10 MB，请缩小选区。".into());
            }
            if provider == "阿里云 OCR统一识别" {
                let kind = settings.aliyun_kind(&settings.ocr_mode);
                if !matches!(
                    kind,
                    "General" | "Advanced" | "HandWriting" | "MultiLang" | "Table"
                ) {
                    return Err("请选择支持的统一识别类型。".into());
                }
                let mut parameters = vec![("Type", kind)];
                if kind != "General" {
                    parameters.push(("OutputCoordinate", "points"));
                }
                let body = cloud::aliyun_query(
                    "RecognizeAllText",
                    &settings.aliyun_region,
                    bytes,
                    false,
                    keys,
                    &parameters,
                )?;
                record_response(&body, response_path)?;
                let parsed = parse_aliyun_unified(
                    &body,
                    settings.ocr_mode == "表格",
                    settings.ocr_mode == "竖排",
                )?;
                return Ok(OcrResult {
                    lines: parsed.0.lines().count(),
                    text: parsed.0,
                    layout: parsed.1,
                    tables: parsed.2,
                    elapsed: start.elapsed(),
                });
            }
            let action = settings.aliyun_action(&settings.ocr_mode);
            let response = if settings.ocr_mode == "表格" && action == "RecognizeAdvanced" {
                cloud::aliyun_query(
                    action,
                    &settings.aliyun_region,
                    bytes,
                    false,
                    keys,
                    &[("OutputTable", "true")],
                )?
            } else if settings.ocr_mode == "表格" && action == "RecognizeTableOcr" {
                cloud::aliyun_query(
                    action,
                    &settings.aliyun_region,
                    bytes,
                    false,
                    keys,
                    &[("LineLess", if line_less { "true" } else { "false" })],
                )?
            } else {
                cloud::aliyun(action, &settings.aliyun_region, bytes, false, keys)?
            };
            record_response(&response, response_path)?;
            if settings.ocr_mode == "公式" {
                parse_aliyun_formula(&response)?
            } else {
                parse_aliyun(
                    &response,
                    settings.ocr_mode == "表格",
                    settings.ocr_mode == "竖排",
                )?
            }
        }
        _ => return Err("OCR 提供商路由错误。".into()),
    };
    Ok(OcrResult {
        lines: text.lines().count(),
        text,
        layout,
        tables,
        elapsed: start.elapsed(),
    })
}

fn record_response(body: &Value, path: Option<&std::path::Path>) -> Result<(), String> {
    if let Some(path) = path {
        let bytes = serde_json::to_vec_pretty(body).map_err(|_| "无法编码接口响应。")?;
        std::fs::write(path, bytes).map_err(|_| "无法保存接口响应。")?;
    }
    Ok(())
}

/// Long dark vertical rules distinguish a ruled scan from striped UI tables.
/// Text strokes alone must not force the ruled-table service mode.
pub(crate) fn has_vertical_rules(image: &RgbaImage) -> bool {
    let (width, height) = image.dimensions();
    let required = (height / 3).max(60);
    let mut groups = 0;
    let mut previous = false;
    for x in 0..width {
        let mut run = 0;
        let mut longest = 0;
        for y in 0..height {
            let p = image.get_pixel(x, y).0;
            if p[3] > 128 && p[..3].iter().all(|v| *v < 160) {
                run += 1;
                longest = longest.max(run);
            } else {
                run = 0;
            }
        }
        let rule = longest >= required;
        if rule && !previous {
            groups += 1;
        }
        if groups >= 2 {
            return true;
        }
        previous = rule;
    }
    false
}
type Parsed = (String, Option<String>, Vec<Table>);
fn tencent_payload(image: String, action: &str, language: &str) -> Value {
    let mut payload = json!({"ImageBase64":image});
    if action == "GeneralBasicOCR" {
        payload["LanguageType"] = json!(match language {
            "JAP" => "jap",
            "KOR" => "kor",
            "FRE" => "fre",
            "GER" => "ger",
            "RUS" => "rus",
            "SPA" => "spa",
            "POR" => "por",
            "ITA" => "ita",
            "MIX" => "mix",
            "CHN_ENG" | "ENG" => "zh",
            _ => "auto",
        });
    } else if action == "GeneralAccurateOCR" && language == "MUL" {
        payload["ConfigID"] = json!("MulOCR");
    }
    payload
}
fn number(value: &Value, name: &str) -> Result<usize, String> {
    value
        .get(name)
        .and_then(Value::as_u64)
        .and_then(|n| usize::try_from(n).ok())
        .ok_or_else(|| format!("表格缺少有效的 {name}。"))
}
fn add_cell(
    table: &mut Table,
    row: usize,
    col: usize,
    row_end: usize,
    col_end: usize,
    text: &str,
) -> Result<(), String> {
    if row_end < row || col_end < col || row_end > 5000 || col_end > 500 {
        return Err("表格范围过大或无效。".into());
    }
    let rowspan = (row_end - row).max(1);
    let colspan = (col_end - col).max(1);
    table.rows = table.rows.max(row + rowspan);
    table.cols = table.cols.max(col + colspan);
    if table.rows.saturating_mul(table.cols) > 100_000 {
        return Err("表格超过 10 万格，请缩小选区。".into());
    }
    table.cells.push(Cell {
        row,
        col,
        rowspan,
        colspan,
        text: text.into(),
    });
    Ok(())
}
fn table_output(tables: Vec<Table>) -> Parsed {
    (
        tables
            .iter()
            .map(Table::document_text)
            .collect::<Vec<_>>()
            .join("\n\n"),
        None,
        tables,
    )
}
fn text_output(lines: Vec<PositionedLine>, vertical: bool) -> Parsed {
    let has_positions = !lines.is_empty()
        && lines.iter().all(|line| {
            line.x.is_finite()
                && line.y.is_finite()
                && line.width.is_finite()
                && line.height.is_finite()
                && line.width > 0.
                && line.height > 0.
        });
    if vertical && has_positions {
        return (vertical_text(&lines), None, Vec::new());
    }
    let text = lines
        .iter()
        .map(|l| l.text.as_str())
        .collect::<Vec<_>>()
        .join("\n");
    let layout = (has_positions && !vertical).then(|| recover_layout(&lines));
    (text, layout, Vec::new())
}
fn box_from_points(value: &Value) -> Option<(f64, f64, f64, f64)> {
    let points = value.as_array()?;
    let get = |v: &Value, lower: &str, upper: &str| {
        v.get(lower)
            .or_else(|| v.get(upper))
            .and_then(Value::as_f64)
    };
    let xs = points
        .iter()
        .map(|p| get(p, "x", "X"))
        .collect::<Option<Vec<_>>>()?;
    let ys = points
        .iter()
        .map(|p| get(p, "y", "Y"))
        .collect::<Option<Vec<_>>>()?;
    let x = xs.iter().copied().reduce(f64::min)?;
    let y = ys.iter().copied().reduce(f64::min)?;
    Some((
        x,
        y,
        xs.iter().copied().reduce(f64::max)? - x,
        ys.iter().copied().reduce(f64::max)? - y,
    ))
}
pub(crate) fn parse_tencent(body: &Value, table: bool, vertical: bool) -> Result<Parsed, String> {
    if table {
        let entries = body
            .get("TableDetections")
            .and_then(Value::as_array)
            .ok_or("腾讯云未返回 TableDetections。")?;
        let mut tables = Vec::new();
        for entry in entries {
            let mut table = Table::default();
            for cell in entry
                .get("Cells")
                .and_then(Value::as_array)
                .ok_or("腾讯云表格缺少 Cells。")?
            {
                add_cell(
                    &mut table,
                    number(cell, "RowTl")?,
                    number(cell, "ColTl")?,
                    number(cell, "RowBr")?,
                    number(cell, "ColBr")?,
                    cell.get("Text").and_then(Value::as_str).unwrap_or(""),
                )?;
            }
            tables.push(table);
        }
        return Ok(table_output(tables));
    }
    let detections = body
        .get("TextDetections")
        .and_then(Value::as_array)
        .ok_or("腾讯云未返回 TextDetections。")?;
    let mut positioned = Vec::new();
    for (i, item) in detections.iter().enumerate() {
        let text = item
            .get("DetectedText")
            .and_then(Value::as_str)
            .ok_or("腾讯云文字行格式错误。")?;
        let (x, y, width, height) =
            item.get("Polygon")
                .and_then(box_from_points)
                .unwrap_or((0., i as f64 * 24., 0., 20.));
        positioned.push(PositionedLine {
            text: text.into(),
            x,
            y,
            width,
            height,
        });
    }
    Ok(text_output(positioned, vertical))
}
fn aliyun_data(body: &Value) -> Result<Value, String> {
    let raw = body.get("Data").ok_or("阿里云响应缺少 Data。")?;
    Ok(if let Some(s) = raw.as_str() {
        serde_json::from_str(s).map_err(|_| "阿里云 Data JSON 无法解析。")?
    } else {
        raw.clone()
    })
}
fn parse_aliyun_formula(body: &Value) -> Result<Parsed, String> {
    let data = aliyun_data(body)?;
    if let Some(latex) = data
        .get("content")
        .and_then(Value::as_str)
        .filter(|s| !s.trim().is_empty())
    {
        return Ok((latex.into(), None, Vec::new()));
    }
    parse_aliyun(body, false, false)
}
pub(crate) fn parse_aliyun(body: &Value, table: bool, vertical: bool) -> Result<Parsed, String> {
    let data = aliyun_data(body)?;
    if table {
        return Ok(table_output(self::table::parse(&data)?));
    }
    if let Some(items) = data.get("prism_wordsInfo").and_then(Value::as_array) {
        let mut lines = Vec::new();
        for (i, item) in items.iter().enumerate() {
            let text = item
                .get("word")
                .and_then(Value::as_str)
                .ok_or("阿里云文字行格式错误。")?;
            let (x, y, width, height) =
                item.get("pos")
                    .and_then(box_from_points)
                    .unwrap_or((0., i as f64 * 24., 0., 20.));
            lines.push(PositionedLine {
                text: text.into(),
                x,
                y,
                width,
                height,
            });
        }
        return Ok(text_output(lines, vertical));
    }
    let text = data
        .get("content")
        .and_then(Value::as_str)
        .ok_or("阿里云响应缺少识别正文。")?
        .to_owned();
    Ok((text, None, Vec::new()))
}
pub(crate) fn parse_aliyun_unified(
    body: &Value,
    table: bool,
    vertical: bool,
) -> Result<Parsed, String> {
    let raw = body.get("Data").ok_or("统一识别响应缺少 Data。")?;
    let data = if let Some(text) = raw.as_str() {
        serde_json::from_str::<Value>(text).map_err(|_| "统一识别 Data 无法解析。")?
    } else {
        raw.clone()
    };
    let images = data
        .get("SubImages")
        .and_then(Value::as_array)
        .ok_or("统一识别缺少 SubImages。")?;
    if table {
        let mut tables = Vec::new();
        for image in images {
            if let Some(entries) = image
                .pointer("/TableInfo/TableDetails")
                .and_then(Value::as_array)
            {
                for entry in entries {
                    let notes = |key: &str| {
                        entry
                            .get(key)
                            .and_then(|v| v.get("Contents"))
                            .and_then(Value::as_array)
                            .into_iter()
                            .flatten()
                            .filter_map(Value::as_str)
                            .collect::<Vec<_>>()
                            .join("\n")
                    };
                    let mut table = Table {
                        header: notes("Header"),
                        footer: notes("Footer"),
                        ..Table::default()
                    };
                    for cell in entry
                        .get("CellDetails")
                        .and_then(Value::as_array)
                        .ok_or("统一表格缺少 CellDetails。")?
                    {
                        add_cell(
                            &mut table,
                            number(cell, "RowStart")?,
                            number(cell, "ColumnStart")?,
                            number(cell, "RowEnd")?
                                .checked_add(1)
                                .ok_or("表格行溢出。")?,
                            number(cell, "ColumnEnd")?
                                .checked_add(1)
                                .ok_or("表格列溢出。")?,
                            cell.get("CellContent")
                                .and_then(Value::as_str)
                                .unwrap_or(""),
                        )?;
                    }
                    for (key, dimension) in [
                        ("RowCount", &mut table.rows),
                        ("ColumnCount", &mut table.cols),
                    ] {
                        if entry.get(key).is_some() {
                            let declared = number(entry, key)?;
                            if declared < *dimension {
                                return Err("统一接口声明的行列数小于单元格范围。".into());
                            }
                            *dimension = declared;
                        }
                    }
                    table.grid()?;
                    tables.push(table);
                }
            }
        }
        if tables.is_empty() {
            return Err("统一识别没有返回表格，请确认使用表格模式。".into());
        }
        return Ok(table_output(tables));
    }
    let mut blocks = Vec::new();
    let mut paragraphs = Vec::new();
    let mut has_positions = true;
    for image in images {
        if let Some(items) = image
            .pointer("/ParagraphInfo/ParagraphDetails")
            .and_then(Value::as_array)
        {
            paragraphs.extend(
                items
                    .iter()
                    .filter_map(|v| v.get("ParagraphContent").and_then(Value::as_str))
                    .map(String::from),
            );
        }
        if let Some(items) = image
            .pointer("/BlockInfo/BlockDetails")
            .and_then(Value::as_array)
        {
            for item in items {
                if let Some(text) = item.get("BlockContent").and_then(Value::as_str) {
                    let coords = item.get("BlockPoints").and_then(box_from_points);
                    has_positions &= coords.is_some();
                    let (x, y, width, height) = coords.unwrap_or((0., 0., 0., 0.));
                    blocks.push(PositionedLine {
                        text: text.into(),
                        x,
                        y,
                        width,
                        height,
                    });
                }
            }
        }
    }
    has_positions &= !blocks.is_empty()
        && blocks
            .iter()
            .all(|line| line.width > 0. && line.height > 0.);
    let groups = paragraph_groups_from_texts(&blocks, &paragraphs);
    let layout = (has_positions && !blocks.is_empty() && !vertical)
        .then(|| recover_layout_with_hints(&blocks, &groups));
    let mut output = text_output(blocks, vertical);
    output.1 = layout;
    if !has_positions {
        output.1 = None;
    }
    if let Some(content) = data.get("Content").and_then(Value::as_str) {
        if !vertical || output.0.is_empty() {
            // Disagreeing text channels must not let layout restore a stale or
            // incomplete version of the API's full recognized content.
            if !same_content(&output.0, content) {
                output.1 = None;
            }
            output.0 = content.into();
        }
    }
    if !has_positions
        && !vertical
        && !paragraphs.is_empty()
        && same_content(&output.0, &paragraphs.join(""))
    {
        // Some API variants return paragraphs without any coordinates. These
        // are already logical paragraphs; preserve them as the fallback.
        output.1 = Some(paragraphs.join("\n\n"));
    }
    Ok(output)
}

fn same_content(a: &str, b: &str) -> bool {
    a.chars()
        .filter(|c| !c.is_whitespace())
        .eq(b.chars().filter(|c| !c.is_whitespace()))
}
