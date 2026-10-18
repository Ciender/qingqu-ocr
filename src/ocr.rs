use crate::config::{Credentials, Settings};
mod formula;
mod spreadsheet;
use base64::{engine::general_purpose::STANDARD, Engine};
pub(crate) use formula::{parse_baidu_formula, parse_mathpix};
use image::RgbaImage;
pub(crate) mod input;
use reqwest::blocking::Client;
use serde_json::Value;
use std::time::{Duration, Instant};

const TOKEN_URL: &str = "https://aip.baidubce.com/oauth/2.0/token";
const OCR_URL: &str = "https://aip.baidubce.com/rest/2.0/ocr/v1/pp_ocr";

/// Shared routing for the desktop worker and explicit real-image diagnostics.
pub fn recognize(
    client: &mut BaiduClient,
    image: RgbaImage,
    settings: &Settings,
    keys: &Credentials,
) -> Result<OcrResult, String> {
    match settings.ocr_provider_for(&settings.ocr_mode) {
        "阿里云 Qwen-OCR" | "阿里云 Qwen-VL" => crate::qwen::recognize(image, settings, keys),
        "腾讯云 OCR" | "阿里云 OCR" | "阿里云 OCR统一识别" => {
            crate::ocr_cloud::recognize(image, settings, keys)
        }
        "Mathpix" if settings.ocr_mode == "公式" => formula(image, keys, &settings.formula_url),
        "百度 OCR" => client.recognize(
            image,
            keys,
            settings.correct_orientation,
            if settings.ocr_mode == "公式" {
                &settings.baidu_formula_endpoint
            } else {
                &settings.ocr_mode
            },
            settings.ocr_language_for(&settings.ocr_mode),
        ),
        _ => Err("请选择已支持的 OCR 提供商。".into()),
    }
}

#[derive(Clone)]
pub struct OcrResult {
    pub text: String,
    pub lines: usize,
    pub elapsed: Duration,
    pub tables: Vec<Table>,
    pub layout: Option<String>,
}

#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct Table {
    pub cells: Vec<Cell>,
    pub rows: usize,
    pub cols: usize,
    #[serde(default)]
    pub header: String,
    #[serde(default)]
    pub footer: String,
}
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct Cell {
    pub row: usize,
    pub col: usize,
    pub rowspan: usize,
    pub colspan: usize,
    pub text: String,
}
impl Table {
    /// Full OCR text retains notes outside the grid. Office copies use XML/RTF/TSV.
    pub fn document_text(&self) -> String {
        [&self.header, &self.tsv(), &self.footer]
            .into_iter()
            .filter(|text| !text.is_empty())
            .map(String::as_str)
            .collect::<Vec<_>>()
            .join("\n")
    }
    pub fn at(&self, row: usize, col: usize) -> Option<&Cell> {
        self.cells.iter().find(|c| c.row == row && c.col == col)
    }
    /// Validate the API's logical grid without inferring or removing any rows.
    pub fn grid(&self) -> Result<Vec<Option<usize>>, String> {
        if self.rows == 0
            || self.cols == 0
            || self.rows > 5000
            || self.cols > 500
            || self.rows.saturating_mul(self.cols) > 100_000
        {
            return Err("表格范围无效或超过 10 万格。".into());
        }
        let mut owners = vec![None; self.rows * self.cols];
        for (i, c) in self.cells.iter().enumerate() {
            let bottom = c.row.checked_add(c.rowspan).ok_or("表格行溢出。")?;
            let right = c.col.checked_add(c.colspan).ok_or("表格列溢出。")?;
            if c.rowspan == 0 || c.colspan == 0 || bottom > self.rows || right > self.cols {
                return Err("表格单元格超出声明的行列范围。".into());
            }
            for row in c.row..bottom {
                for col in c.col..right {
                    let owner = &mut owners[row * self.cols + col];
                    if owner.is_some() {
                        return Err(
                            "接口返回了重叠单元格，无法忠实显示；请更换接口或缩小截图区域。".into(),
                        );
                    }
                    *owner = Some(i);
                }
            }
        }
        Ok(owners)
    }
    /// Expand a cell selection to include every intersecting merge in full.
    pub fn selection(&self, a: (usize, usize), b: (usize, usize)) -> Result<Self, String> {
        self.grid()?;
        let (mut top, mut bottom, mut left, mut right) =
            (a.0.min(b.0), a.0.max(b.0), a.1.min(b.1), a.1.max(b.1));
        bottom = bottom.min(self.rows - 1);
        right = right.min(self.cols - 1);
        top = top.min(bottom);
        left = left.min(right);
        loop {
            let before = (top, bottom, left, right);
            for c in &self.cells {
                if c.row <= bottom
                    && c.row + c.rowspan > top
                    && c.col <= right
                    && c.col + c.colspan > left
                {
                    top = top.min(c.row);
                    bottom = bottom.max(c.row + c.rowspan - 1);
                    left = left.min(c.col);
                    right = right.max(c.col + c.colspan - 1);
                }
            }
            if before == (top, bottom, left, right) {
                break;
            }
        }
        Ok(Self {
            rows: bottom - top + 1,
            cols: right - left + 1,
            header: String::new(),
            footer: String::new(),
            cells: self
                .cells
                .iter()
                .filter(|c| c.row >= top && c.row <= bottom && c.col >= left && c.col <= right)
                .map(|c| Cell {
                    row: c.row - top,
                    col: c.col - left,
                    ..c.clone()
                })
                .collect(),
        })
    }
    pub fn tsv(&self) -> String {
        (0..self.rows)
            .map(|r| {
                (0..self.cols)
                    .map(|c| {
                        self.at(r, c)
                            .map(|v| v.text.replace(['\t', '\r', '\n'], " "))
                            .unwrap_or_default()
                    })
                    .collect::<Vec<_>>()
                    .join("\t")
            })
            .collect::<Vec<_>>()
            .join("\r\n")
    }
}
fn needs_excel_text(text: &str) -> bool {
    let text = text.trim();
    let numeric = text.replace([',', '，'], "");
    let number = numeric.strip_suffix('%').unwrap_or(&numeric);
    // Preserve dates, identifiers, formulas and precision-sensitive digit strings
    // as source text while ordinary quantities stay usable as Excel numbers.
    !number.parse::<f64>().is_ok_and(f64::is_finite)
        || (number.len() > 1 && number.starts_with('0') && !number.starts_with("0."))
        || number.chars().filter(|c| c.is_ascii_digit()).count() > 15
}

struct Token {
    value: String,
    api_key: String,
    secret_key: String,
    expires: Instant,
}

#[derive(Default)]
pub struct BaiduClient {
    client: Option<Client>,
    token: Option<Token>,
    pub timings: Option<RequestTimings>,
}

/// Local diagnostics only: durations and byte counts, never URLs or credentials.
#[derive(Default, serde::Serialize)]
pub struct RequestTimings {
    prepare_ms: f64,
    base64_ms: f64,
    auth_ms: f64,
    request_build_ms: f64,
    http_ms: f64,
    parse_ms: f64,
    token_requests: usize,
    png_bytes: usize,
    request_bytes: usize,
}

impl BaiduClient {
    /// Release idle sockets/runtime while retaining the token until its expiry.
    pub fn release_idle_http(&mut self) {
        self.client = None;
        self.timings = None;
    }

    fn cached_token(&self, keys: &Credentials) -> Option<&str> {
        self.token.as_ref().and_then(|token| {
            (token.api_key == keys.api_key
                && token.secret_key == keys.secret_key
                && Instant::now() < token.expires)
                .then_some(token.value.as_str())
        })
    }

    fn http(&mut self) -> Result<Client, String> {
        if self.client.is_none() {
            self.client = Some(
                Client::builder()
                    .connect_timeout(Duration::from_secs(12))
                    .timeout(Duration::from_secs(45))
                    .user_agent("QingquOCR/0.1")
                    .build()
                    .map_err(|_| "无法初始化安全网络连接。")?,
            );
        }
        Ok(self.client.as_ref().unwrap().clone())
    }

    fn access_token(&mut self, keys: &Credentials) -> Result<String, String> {
        if let Some(token) = self.cached_token(keys) {
            return Ok(token.to_owned());
        }
        let response = self
            .http()?
            .post(TOKEN_URL)
            .form(&[
                ("grant_type", "client_credentials"),
                ("client_id", keys.api_key.trim()),
                ("client_secret", keys.secret_key.trim()),
            ])
            .send()
            .map_err(network_error)?;
        let status = response.status();
        let body: Value = response
            .json()
            .map_err(|_| "百度鉴权返回了无法解析的响应。")?;
        let value = body
            .get("access_token")
            .and_then(Value::as_str)
            .ok_or_else(|| {
                if status.is_server_error() {
                    "百度鉴权服务暂不可用，请稍后重试。".to_string()
                } else {
                    "百度鉴权失败，请检查 API Key 和 Secret Key。".to_string()
                }
            })?;
        let life = body
            .get("expires_in")
            .and_then(Value::as_u64)
            .unwrap_or(3600);
        self.token = Some(Token {
            value: value.to_owned(),
            api_key: keys.api_key.clone(),
            secret_key: keys.secret_key.clone(),
            expires: Instant::now() + Duration::from_secs(life.saturating_sub(120).min(2_592_000)),
        });
        Ok(value.to_owned())
    }

    pub fn recognize(
        &mut self,
        image: RgbaImage,
        keys: &Credentials,
        orientation: bool,
        mode: &str,
        language: &str,
    ) -> Result<OcrResult, String> {
        if !keys.ready() {
            return Err("请先在设置中填写百度 API Key 和 Secret Key。".into());
        }
        self.timings = None;
        let start = Instant::now();
        let bytes = input::png(image, input::Policy::baidu())?;
        let mut timings = RequestTimings {
            prepare_ms: start.elapsed().as_secs_f64() * 1000.,
            png_bytes: bytes.len(),
            ..RequestTimings::default()
        };
        let phase = Instant::now();
        let encoded = STANDARD.encode(bytes);
        timings.base64_ms = phase.elapsed().as_secs_f64() * 1000.;
        let endpoint = match mode {
            "高精度" => "accurate",
            "通用多语言" | "竖排" => "general",
            "表格" => "table",
            "formula" => "formula",
            "doc_analysis" => "doc_analysis",
            _ => "pp_ocr",
        };
        for attempt in 0..2 {
            let phase = Instant::now();
            let cached = self.cached_token(keys).is_some();
            let token = self.access_token(keys)?;
            timings.token_requests += usize::from(!cached);
            timings.auth_ms += phase.elapsed().as_secs_f64() * 1000.;
            let phase = Instant::now();
            let mut fields = vec![("image", encoded.as_str())];
            let orient = if orientation { "true" } else { "false" };
            if endpoint == "pp_ocr" {
                fields.extend([
                    ("useDocOrientationClassify", orient),
                    ("useTextlineOrientation", orient),
                    ("useDocUnwarping", "false"),
                ]);
            } else if endpoint == "table" {
                fields.push(("return_excel", "false"));
            } else if endpoint == "formula" {
                fields.push(("disp_formula", "true"));
            } else if endpoint == "doc_analysis" {
                fields.extend([("recg_formula", "true"), ("layout_analysis", "true")]);
            } else {
                fields.extend([
                    ("detect_direction", orient),
                    ("paragraph", "true"),
                    ("vertexes_location", "true"),
                ]);
                if mode == "通用多语言" || mode == "高精度" {
                    fields.push(("language_type", language));
                }
                if mode == "竖排" {
                    fields.push(("language_type", "CHN_ENG"));
                }
            }
            let url = if endpoint == "pp_ocr" {
                OCR_URL.to_string()
            } else {
                format!("https://aip.baidubce.com/rest/2.0/ocr/v1/{endpoint}")
            };
            let request = self
                .http()?
                .post(url)
                .query(&[("access_token", token)])
                .form(&fields)
                .build()
                .map_err(|_| "无法创建识别请求。")?;
            timings.request_bytes = request
                .body()
                .and_then(|b| b.as_bytes())
                .map_or(0, |b| b.len());
            timings.request_build_ms += phase.elapsed().as_secs_f64() * 1000.;
            if request
                .body()
                .and_then(|b| b.as_bytes())
                .is_some_and(|b| b.len() > 10 * 1024 * 1024)
            {
                return Err("截图编码后超过百度接口的 10 MB 限制，请缩小选区。".into());
            }
            let phase = Instant::now();
            let response = self.http()?.execute(request).map_err(network_error)?;
            let status = response.status();
            let body: Value = response.json().map_err(|_| "百度返回了无法解析的响应。")?;
            timings.http_ms += phase.elapsed().as_secs_f64() * 1000.;
            if matches!(
                body.get("error_code").and_then(Value::as_i64),
                Some(110 | 111)
            ) && attempt == 0
            {
                self.token = None;
                continue;
            }
            if !status.is_success() && body.get("error_code").is_none() {
                return Err(format!(
                    "百度服务返回 HTTP {}，请稍后重试。",
                    status.as_u16()
                ));
            }
            let phase = Instant::now();
            let tables = if endpoint == "table" {
                parse_tables(&body)?
            } else {
                Vec::new()
            };
            let (text, lines) = if endpoint == "table" {
                let text = tables
                    .iter()
                    .map(Table::document_text)
                    .collect::<Vec<_>>()
                    .join("\n\n");
                let lines = text.lines().count();
                (text, lines)
            } else if endpoint == "pp_ocr" {
                parse_response(&body)?
            } else if matches!(endpoint, "formula" | "doc_analysis") {
                parse_baidu_formula(&body)?
            } else {
                parse_words(&body, mode == "竖排")?
            };
            let layout = if mode == "竖排" {
                None
            } else {
                layout_from_response(&body)
            };
            timings.parse_ms = phase.elapsed().as_secs_f64() * 1000.;
            self.timings = Some(timings);
            return Ok(OcrResult {
                text,
                lines,
                elapsed: start.elapsed(),
                tables,
                layout,
            });
        }
        Err("百度鉴权已失效，请重新检查密钥。".into())
    }
}

fn layout_from_response(body: &Value) -> Option<String> {
    use crate::text::{recover_layout, recover_layout_with_hints, PositionedLine};
    if let Some(pages) = body.get("page_result").and_then(Value::as_array) {
        let mut output = Vec::new();
        for page in pages {
            let lines = page.get("lines")?.as_array()?;
            let boxes = page.get("rec_boxes")?.as_array()?;
            if lines.len() != boxes.len() {
                return None;
            }
            let positioned = lines
                .iter()
                .zip(boxes)
                .map(|(s, b)| {
                    let b = b.as_array()?;
                    let x = b.first()?.as_f64()?;
                    let y = b.get(1)?.as_f64()?;
                    Some(PositionedLine {
                        text: s.as_str()?.into(),
                        x,
                        y,
                        width: b.get(2)?.as_f64()? - x,
                        height: b.get(3)?.as_f64()? - y,
                    })
                })
                .collect::<Option<Vec<_>>>()?;
            output.push(recover_layout(&positioned));
        }
        return Some(output.join("\n\n"));
    }
    let lines = body.get("words_result")?.as_array()?;
    let lines = lines
        .iter()
        .map(|v| {
            let location = v.get("location")?;
            Some(PositionedLine {
                text: v.get("words")?.as_str()?.into(),
                x: location.get("left")?.as_f64()?,
                y: location.get("top")?.as_f64()?,
                width: location.get("width")?.as_f64()?,
                height: location.get("height")?.as_f64()?,
            })
        })
        .collect::<Option<Vec<_>>>()?;
    let groups: Vec<Vec<usize>> = body
        .get("paragraphs_result")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(|paragraph| {
            paragraph
                .get("words_result_idx")?
                .as_array()?
                .iter()
                .map(|value| usize::try_from(value.as_u64()?).ok())
                .collect::<Option<Vec<_>>>()
        })
        .collect();
    Some(recover_layout_with_hints(&lines, &groups))
}
fn parse_words(body: &Value, vertical: bool) -> Result<(String, usize), String> {
    check_error(body)?;
    let words = body
        .get("words_result")
        .and_then(Value::as_array)
        .ok_or("响应缺少文字结果。")?;
    let mut rows = words
        .iter()
        .map(|v| {
            let t = v
                .get("words")
                .and_then(Value::as_str)
                .ok_or("文字行格式错误。")?;
            let x = v
                .pointer("/location/left")
                .and_then(Value::as_i64)
                .unwrap_or(0);
            let y = v
                .pointer("/location/top")
                .and_then(Value::as_i64)
                .unwrap_or(0);
            let width = v
                .pointer("/location/width")
                .and_then(Value::as_i64)
                .unwrap_or(1)
                .max(1);
            Ok((x, y, width, t))
        })
        .collect::<Result<Vec<_>, String>>()?;
    if vertical {
        // Individual glyph boxes jitter horizontally. Sorting every glyph by
        // x first scrambles characters within the same vertical column.
        rows.sort_by_key(|(x, _, width, _)| -(2 * x + width));
        type Column<'a> = (i64, i64, Vec<(i64, &'a str)>);
        let mut columns: Vec<Column<'_>> = Vec::new();
        for (x, y, width, t) in rows {
            let right = x + width;
            if let Some(column) = columns.iter_mut().find(|(left, end, _)| {
                let overlap = right.min(*end) - x.max(*left);
                overlap * 2 >= width.min(*end - *left)
            }) {
                column.0 = column.0.min(x);
                column.1 = column.1.max(right);
                column.2.push((y, t));
            } else {
                columns.push((x, right, vec![(y, t)]));
            }
        }
        columns.sort_by_key(|(left, right, _)| -(left + right));
        let count = columns.len();
        return Ok((
            columns
                .into_iter()
                .map(|(_, _, mut glyphs)| {
                    glyphs.sort_by_key(|(y, _)| *y);
                    glyphs.into_iter().map(|(_, text)| text).collect::<String>()
                })
                .collect::<Vec<_>>()
                .join("\n"),
            count,
        ));
    }
    Ok((
        rows.iter()
            .map(|(_, _, _, s)| *s)
            .collect::<Vec<_>>()
            .join("\n"),
        rows.len(),
    ))
}
fn check_error(body: &Value) -> Result<(), String> {
    if body.get("error_code").is_some() {
        parse_response(body).map(|_| ())
    } else {
        Ok(())
    }
}
pub(crate) fn parse_tables(body: &Value) -> Result<Vec<Table>, String> {
    check_error(body)?;
    let array = body
        .get("tables_result")
        .and_then(Value::as_array)
        .ok_or("响应缺少 tables_result。")?;
    let mut tables = Vec::new();
    for t in array {
        let notes = |section: &str| {
            t.get(section)
                .and_then(Value::as_array)
                .into_iter()
                .flatten()
                .filter_map(|item| item.get("words").and_then(Value::as_str))
                .collect::<Vec<_>>()
                .join("\n")
        };
        let mut table = Table {
            header: notes("header"),
            footer: notes("footer"),
            ..Table::default()
        };
        // V2 header/footer entries are free text outside the grid and have
        // locations, but no row_start/col_start. Only body contains cells.
        for section in ["body"] {
            if let Some(cells) = t.get(section).and_then(Value::as_array) {
                for v in cells {
                    let get = |k: &str| {
                        v.get(k)
                            .and_then(Value::as_u64)
                            .map(|n| n as usize)
                            .ok_or_else(|| format!("表格单元格缺少 {k}。"))
                    };
                    let row = get("row_start")?;
                    let col = get("col_start")?;
                    let row_end = get("row_end")?;
                    let col_end = get("col_end")?;
                    if row_end < row || col_end < col || row_end > 5000 || col_end > 500 {
                        return Err("表格范围无效或过大。".into());
                    }
                    let cell = Cell {
                        row,
                        col,
                        rowspan: (row_end - row).max(1),
                        colspan: (col_end - col).max(1),
                        text: v.get("words").and_then(Value::as_str).unwrap_or("").into(),
                    };
                    table.rows = table.rows.max(row + cell.rowspan);
                    table.cols = table.cols.max(col + cell.colspan);
                    table.cells.push(cell);
                }
            }
        }
        if table.rows.saturating_mul(table.cols) > 100_000 {
            return Err("表格超过 10 万格，无法预览。".into());
        }
        if table.rows > 0 && table.cols > 0 {
            tables.push(table);
        }
    }
    if tables.is_empty() {
        return Err("百度未检测到表格，请截取包含完整行列的表格区域。".into());
    }
    Ok(tables)
}

pub fn formula(image: RgbaImage, keys: &Credentials, url: &str) -> Result<OcrResult, String> {
    if keys.formula_appid.trim().is_empty() || keys.formula_key.trim().is_empty() {
        return Err("请填写 Mathpix App ID / App Key，或在公式接口设置中选择百度 OCR。".into());
    }
    let start = Instant::now();
    let png = input::png(image, input::Policy::standard())?;
    let client = Client::builder()
        .timeout(Duration::from_secs(60))
        .build()
        .map_err(|_| "无法初始化公式请求。")?;
    let response=client.post(url).header("app_id",keys.formula_appid.trim()).header("app_key",keys.formula_key.trim()).json(&serde_json::json!({"src":format!("data:image/png;base64,{}",STANDARD.encode(png)),"formats":["text","latex_styled"]})).send().map_err(network_error)?;
    let status = response.status();
    let body: Value = response.json().map_err(|_| "公式服务响应格式错误。")?;
    if !status.is_success() || body.get("error").is_some() {
        return Err(format!(
            "公式识别失败（HTTP {}），请检查凭据、额度和图片。",
            status.as_u16()
        ));
    }
    let text = parse_mathpix(&body)?;
    Ok(OcrResult {
        lines: text.lines().count(),
        text,
        tables: Vec::new(),
        layout: None,
        elapsed: start.elapsed(),
    })
}

fn network_error(error: reqwest::Error) -> String {
    // Never display the request URL: its query includes the access token.
    if error.is_timeout() {
        "请求超时，请检查网络后重试。".into()
    } else if error.is_connect() {
        "无法连接百度，请检查网络或代理。".into()
    } else {
        "网络请求失败，请检查网络后重试。".into()
    }
}

fn parse_response(body: &Value) -> Result<(String, usize), String> {
    if let Some(code) = body.get("error_code").and_then(Value::as_i64) {
        let reason = match code {
            17..=19 => "配额或调用频率超限，请检查百度控制台或稍后重试",
            6 | 100 | 110 | 111 => "鉴权失败或当前接口未开通，请核对该接口的权限",
            216201..=216203 => "图片格式、大小或内容不符合接口要求",
            282000 | 282003 => "识别服务暂时不可用，请稍后重试",
            _ => "识别失败，请在百度控制台核对该错误码",
        };
        // Do not echo arbitrary upstream text: it can contain access tokens.
        let detail = match body.get("error_msg").and_then(Value::as_str) {
            Some("No permission to access data") => {
                " No permission to access data（当前应用缺少本接口权限）"
            }
            _ => "",
        };
        return Err(format!("百度错误 {code}：{reason}。{detail}"));
    }
    let pages = body
        .get("page_result")
        .and_then(Value::as_array)
        .ok_or("百度响应缺少 page_result，接口格式可能已变化。")?;
    let mut count = 0;
    let mut text_pages = Vec::new();
    for page in pages {
        let lines = page
            .get("lines")
            .and_then(Value::as_array)
            .ok_or("百度响应缺少 lines。")?;
        let mut text = Vec::new();
        for line in lines {
            let line = line.as_str().ok_or("百度返回了无法识别的文本行格式。")?;
            if !line.is_empty() {
                text.push(line);
                count += 1;
            }
        }
        if !text.is_empty() {
            text_pages.push(text.join("\n"));
        }
    }
    Ok((text_pages.join("\n\n"), count))
}
