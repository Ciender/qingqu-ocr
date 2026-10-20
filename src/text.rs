//! Lightweight local text processing. Explicit paragraphs and structured blocks win over heuristics.
#[path = "text/layout.rs"]
mod layout;
#[path = "text/prose.rs"]
mod prose;
use ferrous_opencc::{config::BuiltinConfig, OpenCC};
use regex::Regex;
use std::sync::LazyLock;

#[derive(Clone, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct FormatOptions {
    pub repair_wrap: bool,
    pub paragraphs: bool,
    pub trim: bool,
    pub indent: bool,
    pub blank_lines: bool,
}
impl Default for FormatOptions {
    fn default() -> Self {
        Self {
            repair_wrap: true,
            paragraphs: true,
            trim: true,
            indent: false,
            blank_lines: false,
        }
    }
}

/// Apply the explicit 智能排版 command once. These options belong to this
/// command alone; selecting an OCR paragraph mode must not enable indentation,
/// cleanup or other formatting as a side effect.
pub fn intelligent_format(
    input: &str,
    positioned: Option<&str>,
    options: &FormatOptions,
    indent: usize,
) -> String {
    if !options.paragraphs
        && !options.repair_wrap
        && !options.trim
        && !options.indent
        && options.blank_lines
    {
        return input.to_owned();
    }
    // Use OCR boundaries only when the caller supplies a matching document.
    // Formatting edited text must never restore an older recognition result.
    let positioned = positioned.filter(|layout| {
        options.paragraphs
            && options.repair_wrap
            && input
                .chars()
                .filter(|c| !c.is_whitespace())
                .eq(layout.chars().filter(|c| !c.is_whitespace()))
    });
    let source = positioned.unwrap_or(input);
    let normalized = source.replace("\r\n", "\n").replace('\r', "\n");
    // A current editor row is a logical paragraph. Repair physical wraps only
    // inside explicit paragraph groups or a validated OCR layout. A period
    // alone is not evidence of a new paragraph.
    let join_rows =
        options.repair_wrap && (positioned.is_some() || has_prose_blank_line(&normalized));
    let blocks = format_current_rows(&normalized, join_rows, options.paragraphs);
    let mut output = String::new();
    for block in blocks {
        if !output.is_empty() {
            output.push_str(if options.blank_lines { "\n\n" } else { "\n" });
        }
        if block.protected {
            output.push_str(&block.text);
        } else {
            let cleaned;
            let text = if options.trim {
                cleaned = prose::clean(&block.text);
                cleaned.as_str()
            } else if join_rows {
                block.text.trim()
            } else {
                &block.text
            };
            if options.indent {
                output.push_str(&"　".repeat(indent.min(16)));
                output.push_str(text.trim_start());
            } else {
                output.push_str(text);
            }
        }
    }
    output
}

/// Format a caller-validated, unedited OCR snapshot. Reapply only the current
/// character-conversion modes to layout, so punctuation/traditional conversion
/// does not discard geometry or restore an older version of the text. Column
/// ordering may change, but every non-whitespace character must still match.
pub fn intelligent_format_ocr(
    input: &str,
    positioned: Option<&str>,
    modes: &ProcessingModes,
    options: &FormatOptions,
    indent: usize,
) -> String {
    if options.paragraphs && options.repair_wrap {
        if let Some(layout) = positioned {
            let mut conversions = modes.clone();
            conversions.layout = "split".into();
            if let Ok(adjusted) = conversions.apply(layout, None, options, indent) {
                let mut a: Vec<_> = input.chars().filter(|c| !c.is_whitespace()).collect();
                let mut b: Vec<_> = adjusted.chars().filter(|c| !c.is_whitespace()).collect();
                a.sort_unstable();
                b.sort_unstable();
                if a == b {
                    return intelligent_format(&adjusted, Some(&adjusted), options, indent);
                }
            }
            // The editor may still contain raw OCR before its conversion modes.
            if input
                .chars()
                .filter(|c| !c.is_whitespace())
                .eq(layout.chars().filter(|c| !c.is_whitespace()))
            {
                return intelligent_format(input, Some(layout), options, indent);
            }
        }
    }
    intelligent_format(input, None, options, indent)
}

/// The explicit format action on pasted text always produces one logical row.
/// Cleanup still applies; paragraph spacing and indentation never add padding.
pub fn format_pasted_text(input: &str, options: &FormatOptions) -> String {
    let normalized = pasted_line_breaks(input);
    let merged = merge(&normalized, true);
    if options.trim {
        prose::clean(&merged)
    } else {
        merged
    }
}

fn pasted_line_breaks(input: &str) -> String {
    input.replace("\r\n", "\n").replace(
        [
            '\r', '\u{0085}', '\u{2028}', '\u{2029}', '\u{000b}', '\u{000c}',
        ],
        "\n",
    )
}

struct FormatBlock {
    text: String,
    protected: bool,
}

fn has_prose_blank_line(input: &str) -> bool {
    let mut fence = FenceState::default();
    for line in input.trim().lines() {
        if !fence.protect(line) && line.trim().is_empty() {
            return true;
        }
    }
    false
}

/// Keep current logical paragraphs and structured blocks, or join rows inside
/// explicit paragraph groups. No sentence splitting or character-count guess.
fn format_current_rows(input: &str, join_rows: bool, recover_indents: bool) -> Vec<FormatBlock> {
    let mut output: Vec<FormatBlock> = Vec::new();
    let mut paragraph = String::new();
    let mut fence = FenceState::default();
    let flush = |paragraph: &mut String, output: &mut Vec<FormatBlock>| {
        if !paragraph.is_empty() {
            let text = if join_rows {
                prose::repair(&paragraph.lines().collect::<Vec<_>>())
            } else {
                std::mem::take(paragraph)
            };
            paragraph.clear();
            output.push(FormatBlock {
                text,
                protected: false,
            });
        }
    };
    for raw in input.lines() {
        let line = raw.trim();
        if fence.protect(line) || structured(line) {
            flush(&mut paragraph, &mut output);
            if let Some(previous) = output.last_mut().filter(|b| b.protected) {
                previous.text.push('\n');
                previous.text.push_str(raw);
            } else {
                output.push(FormatBlock {
                    text: raw.to_owned(),
                    protected: true,
                });
            }
            continue;
        }
        if line.is_empty() {
            flush(&mut paragraph, &mut output);
            continue;
        }
        if !join_rows || (recover_indents && raw.starts_with([' ', '\u{3000}'])) {
            flush(&mut paragraph, &mut output);
        }
        if join_rows {
            if !paragraph.is_empty() {
                paragraph.push('\n');
            }
            paragraph.push_str(line);
        } else {
            paragraph.push_str(raw);
        }
    }
    flush(&mut paragraph, &mut output);
    output
}

/// Persistent output modes, shared by toolbar actions and subsequent OCR jobs.
#[derive(Clone, Default, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct ProcessingModes {
    pub layout: String,
    pub punctuation: String,
    pub chinese: String,
}
impl ProcessingModes {
    /// Pasted text has real line/paragraph separators but no OCR geometry.
    /// Indented paragraph starts provide additional evidence for rejoining
    /// wrapped rows; otherwise the shared conservative fallback keeps rows.
    pub fn apply_pasted(
        &self,
        input: &str,
        options: &FormatOptions,
        indent: usize,
    ) -> Result<String, String> {
        let source = pasted_line_breaks(input);
        let mut fence = FenceState::default();
        let indented = source.lines().any(|line| {
            !fence.protect(line)
                && !line.trim().is_empty()
                && !structured(line)
                && line.starts_with([' ', '\u{3000}'])
        });
        let hints = (self.layout == "smart" && indented).then_some(source.as_str());
        self.apply(&source, hints, options, indent)
    }

    pub fn layout_text(
        &self,
        input: &str,
        positioned: Option<&str>,
        _options: &FormatOptions,
        _indent: usize,
    ) -> String {
        let output = match self.layout.as_str() {
            // "smart" is the persisted compatibility value for 自动分段.
            // Paragraph boundaries are single logical line breaks; blank
            // paragraph spacing belongs to the explicit format command.
            "smart" => automatic_paragraphs(input, positioned),
            // Explicit merge always produces one logical line, including lists
            // and source blocks, with word spacing preserved at joined rows.
            "merge" => merge(positioned.unwrap_or(input), true),
            // 段落拆分 preserves OCR line boundaries. Sentence splitting is
            // a separate explicit tool and must not run as an OCR mode.
            "split" => input.replace("\r\n", "\n").replace('\r', "\n"),
            _ => input.to_owned(),
        };
        match self.punctuation.as_str() {
            "chinese" => punctuation(&output, true),
            "english" => punctuation(&output, false),
            _ => output,
        }
    }
    pub fn apply(
        &self,
        input: &str,
        positioned: Option<&str>,
        options: &FormatOptions,
        indent: usize,
    ) -> Result<String, String> {
        let output = self.layout_text(input, positioned, options, indent);
        match self.chinese.as_str() {
            "simplified" => convert_chinese(&output, false),
            "traditional" => convert_chinese(&output, true),
            _ => Ok(output),
        }
    }
}

fn automatic_paragraphs(input: &str, positioned: Option<&str>) -> String {
    if let Some(layout) = positioned {
        return compact_paragraph_spacing(&smart_paragraphs(layout));
    }
    // Without OCR geometry, current logical paragraphs are authoritative.
    // Explicit empty rows can still delimit groups containing physical wraps.
    let normalized = input.replace("\r\n", "\n").replace('\r', "\n");
    if has_prose_blank_line(&normalized) {
        compact_paragraph_spacing(&smart_paragraphs(&normalized))
    } else {
        compact_paragraph_spacing(&tidy(&normalized))
    }
}

/// Remove empty prose rows while retaining blank lines inside source blocks.
fn compact_paragraph_spacing(input: &str) -> String {
    let normalized = input.replace("\r\n", "\n").replace('\r', "\n");
    let mut rows = Vec::new();
    let mut fence = FenceState::default();
    for line in normalized.lines() {
        if fence.protect(line) || !line.trim().is_empty() {
            rows.push(line);
        }
    }
    rows.join("\n")
}

static HEADING: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"^(?:第[零一二三四五六七八九十百千\d]+[条章节款]|[一二三四五六七八九十]+[、．.]|[（(][一二三四五六七八九十\d]+[）)]|\d+(?:[、．]|[.)](?:\s|[^\d]|$))|[-+*•·●○◦▪‣]\s*|#{1,6}\s)").unwrap()
});
fn structured(s: &str) -> bool {
    HEADING.is_match(s.trim())
        || s.contains('\t')
        || s.matches('|').count() > 1
        || s.trim_start().starts_with("```")
        || s.trim_start().starts_with("~~~")
        || s.trim_start().starts_with("$$")
}
fn cjk(c: char) -> bool {
    matches!(c as u32, 0x2e80..=0x9fff | 0xac00..=0xd7af | 0xf900..=0xfaff | 0x20000..=0x323af)
}
fn join_line(out: &mut String, next: &str) {
    let soft_hyphen = out.ends_with('\u{00ad}');
    if soft_hyphen {
        out.pop();
    }
    if !soft_hyphen {
        if let (Some(a), Some(b)) = (out.chars().last(), next.chars().next()) {
            if b.is_alphanumeric()
                && !cjk(b)
                && !cjk(a)
                && (a.is_ascii() || a.is_alphanumeric() || matches!(a, '”' | '’'))
                && !matches!(a, '(' | '[' | '{' | '/' | '\\' | '-' | '_' | '@')
                && !a.is_whitespace()
            {
                out.push(' ');
            }
        }
    }
    out.push_str(next);
}

pub fn tidy(input: &str) -> String {
    let normalized = input.replace("\r\n", "\n").replace('\r', "\n");
    let mut out = Vec::new();
    let mut blank = false;
    let mut fence = FenceState::default();
    for raw in normalized.lines() {
        if fence.protect(raw) {
            out.push(raw.to_owned());
            blank = false;
            continue;
        }
        let line = raw.trim_end();
        if line.is_empty() {
            if !out.is_empty() && !blank {
                out.push(String::new());
            }
            blank = true;
        } else {
            out.push(line.to_owned());
            blank = false;
        }
    }
    if fence.close.is_none() {
        while out.last().is_some_and(String::is_empty) {
            out.pop();
        }
    }
    out.join("\n")
}

pub fn smart_paragraphs(input: &str) -> String {
    let clean = tidy(input);
    let mut result = String::new();
    let mut paragraph = String::new();
    let mut fence = FenceState::default();
    let flush = |p: &mut String, r: &mut String| {
        if !p.is_empty() {
            if !r.is_empty() && !r.ends_with('\n') {
                r.push('\n');
            }
            r.push_str(p);
            p.clear();
        }
    };
    let lines: Vec<_> = clean.lines().collect();
    for (i, raw) in lines.iter().enumerate() {
        let line = raw.trim();
        if fence.protect(raw) || structured(line) {
            flush(&mut paragraph, &mut result);
            if !result.is_empty() && !result.ends_with('\n') {
                result.push('\n');
            }
            result.push_str(raw);
            result.push('\n');
            continue;
        }
        if line.is_empty() {
            flush(&mut paragraph, &mut result);
            if !result.ends_with("\n\n") {
                result.push_str("\n\n");
            }
            continue;
        }
        if raw.starts_with([' ', '\u{3000}']) && !paragraph.is_empty() {
            flush(&mut paragraph, &mut result);
            result.push_str("\n\n");
        }
        join_line(&mut paragraph, line);
        let next_heading = lines.get(i + 1).is_some_and(|s| structured(s));
        // Text length is not visual line width, especially in mixed scripts.
        // Geometry is converted to explicit boundaries by recover_layout;
        // never split a paragraph merely because a sentence or short line ends.
        if next_heading {
            flush(&mut paragraph, &mut result);
            result.push_str("\n\n");
        }
    }
    flush(&mut paragraph, &mut result);
    tidy(&result)
}

#[derive(Clone, Debug)]
pub struct PositionedLine {
    pub text: String,
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
}
/// Read vertical columns right-to-left, and their glyphs top-to-bottom.
/// Horizontal box overlap keeps small glyph offsets from changing reading order.
pub fn vertical_text(lines: &[PositionedLine]) -> String {
    let mut rows: Vec<_> = lines.iter().collect();
    rows.sort_by(|a, b| (2. * b.x + b.width).total_cmp(&(2. * a.x + a.width)));
    let mut columns: Vec<(f64, f64, Vec<&PositionedLine>)> = Vec::new();
    for line in rows {
        let right = line.x + line.width;
        if let Some(column) = columns.iter_mut().find(|(left, end, _)| {
            let overlap = right.min(*end) - line.x.max(*left);
            overlap * 2. >= line.width.min(*end - *left)
        }) {
            column.0 = column.0.min(line.x);
            column.1 = column.1.max(right);
            column.2.push(line);
        } else {
            columns.push((line.x, right, vec![line]));
        }
    }
    columns.sort_by(|a, b| (b.0 + b.1).total_cmp(&(a.0 + a.1)));
    columns
        .into_iter()
        .map(|(_, _, mut glyphs)| {
            glyphs.sort_by(|a, b| a.y.total_cmp(&b.y));
            glyphs
                .into_iter()
                .map(|line| line.text.as_str())
                .collect::<String>()
        })
        .collect::<Vec<_>>()
        .join("\n")
}
fn block_marker(text: &str) -> bool {
    let text = text.trim();
    text.starts_with("```") || text.starts_with("~~~") || matches!(text, "$$" | "\\[" | "\\]")
}

#[derive(Default)]
struct FenceState {
    close: Option<(u8, usize)>,
}
impl FenceState {
    /// A closing delimiter is protected too. A different delimiter inside a
    /// fenced block is content, not a new boundary (e.g. $$ in Rust source).
    fn protect(&mut self, text: &str) -> bool {
        let text = text.trim();
        if let Some((marker, count)) = self.close {
            let closes = match marker {
                b'$' => text == "$$",
                b'[' => text == "\\]",
                _ => {
                    text.bytes().take_while(|&c| c == marker).count() >= count
                        && text.bytes().all(|c| c == marker)
                }
            };
            if closes {
                self.close = None;
            }
            return true;
        }
        if text == "$$" {
            self.close = Some((b'$', 2));
            return true;
        }
        if text == "\\[" {
            self.close = Some((b'[', 1));
            return true;
        }
        if text == "\\]" {
            return true;
        }
        if let Some(marker @ (b'`' | b'~')) = text.bytes().next() {
            let count = text.bytes().take_while(|&c| c == marker).count();
            if count >= 3 {
                self.close = Some((marker, count));
                return true;
            }
        }
        false
    }
}
/// Recover layout with the shared, provider-independent lightweight engine.
pub fn recover_layout(lines: &[PositionedLine]) -> String {
    layout::recover(lines, &[])
}

/// Provider paragraph groups are optional, validated auxiliary evidence.
pub fn recover_layout_with_hints(lines: &[PositionedLine], groups: &[Vec<usize>]) -> String {
    layout::recover(lines, groups)
}

/// Match provider paragraph text to complete OCR rows without trusting a
/// second, potentially incomplete text channel over the recognized content.
pub fn paragraph_groups_from_texts(
    lines: &[PositionedLine],
    paragraphs: &[String],
) -> Vec<Vec<usize>> {
    let compact = |text: &str| {
        text.chars()
            .filter(|c| !c.is_whitespace())
            .collect::<String>()
    };
    let contents: Vec<_> = lines.iter().map(|line| compact(&line.text)).collect();
    let paragraphs: Vec<_> = paragraphs
        .iter()
        .map(|p| compact(p))
        .filter(|p| !p.is_empty())
        .collect();
    if paragraphs.is_empty() || contents.concat() != paragraphs.concat() {
        return Vec::new();
    }
    let mut groups = vec![Vec::new(); paragraphs.len()];
    let mut group = 0;
    let mut paragraph_end = paragraphs[0].len();
    let mut offset = 0;
    for (index, content) in contents.iter().enumerate() {
        while group + 1 < paragraphs.len() && offset >= paragraph_end {
            group += 1;
            paragraph_end += paragraphs[group].len();
        }
        if !content.is_empty() && offset + content.len() <= paragraph_end {
            groups[group].push(index);
        }
        offset += content.len();
    }
    groups
}

pub fn merge(input: &str, all: bool) -> String {
    if all {
        let normalized = input.replace("\r\n", "\n").replace('\r', "\n");
        let mut output = String::new();
        for line in normalized.lines().map(str::trim).filter(|s| !s.is_empty()) {
            join_line(&mut output, line);
        }
        return output;
    }
    let clean = tidy(input);
    let mut out = String::new();
    let mut fence = FenceState::default();
    for raw in clean.lines() {
        let line = raw.trim();
        if fence.protect(raw) || structured(line) {
            if !out.is_empty() && !out.ends_with('\n') {
                out.push('\n');
            }
            out.push_str(raw);
            out.push('\n');
        } else if line.is_empty() {
            if !all && !out.ends_with("\n\n") {
                out.push_str("\n\n");
            }
        } else {
            join_line(&mut out, line);
        }
    }
    tidy(&out)
}

// Kept only as a regression reference. The production paragraph selector does
// not split sentences; 段落拆分 displays the original OCR rows.

pub fn indent_paragraphs(input: &str, count: usize) -> String {
    let prefix = "　".repeat(count.min(16));
    let normalized = input.replace("\r\n", "\n").replace('\r', "\n");
    let mut fence = FenceState::default();
    normalized
        .split('\n')
        .map(|line| {
            if fence.protect(line) || line.trim().is_empty() || structured(line) {
                line.to_owned()
            } else {
                format!("{prefix}{}", line.trim_start())
            }
        })
        .collect::<Vec<_>>()
        .join("\n")
}
pub fn wrap(input: &str, columns: usize) -> String {
    let input = merge(input, false);
    let mut fence = FenceState::default();
    input
        .split('\n')
        .map(|line| {
            if fence.protect(line) || structured(line) {
                line.to_string()
            } else {
                textwrap::fill(line, columns.clamp(10, 200))
            }
        })
        .collect::<Vec<_>>()
        .join("\n")
}

/// URLs, email, code, formula and decimal tokens are left untouched.
pub fn punctuation(input: &str, chinese: bool) -> String {
    static PROTECTED: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(
            r#"https?://[^\s\"'“”‘’]+|[^\s\"'“”‘’]+@[^\s\"'“”‘’]+|`[^`]*`|\$[^$]*\$|\d+[.,]\d+"#,
        )
        .unwrap()
    });
    let mut output = String::new();
    let mut fence = FenceState::default();
    let mut quotes = QuotePairs::default();
    for (line_i, line) in input.split('\n').enumerate() {
        if line_i > 0 {
            output.push('\n');
        }
        if fence.protect(line) || structured(line) {
            output.push_str(line);
            continue;
        }
        let protected = PROTECTED
            .find_iter(line)
            .map(|m| m.range())
            .collect::<Vec<_>>();
        convert_marks(line, &protected, chinese, &mut quotes, &mut output);
    }
    output
}
#[derive(Default)]
struct QuotePairs {
    double_open: bool,
    single_open: bool,
}
fn convert_marks(
    s: &str,
    protected: &[std::ops::Range<usize>],
    chinese: bool,
    quotes: &mut QuotePairs,
    out: &mut String,
) {
    let chars: Vec<_> = s.char_indices().collect();
    let mut next_cjk = vec![false; chars.len()];
    let mut next = None;
    for (i, (_, c)) in chars.iter().enumerate().rev() {
        next_cjk[i] = next.is_some_and(cjk);
        if !c.is_whitespace() {
            next = Some(*c);
        }
    }
    let mut prev = None;
    let mut protected_i = 0;
    for (i, (byte, c)) in chars.iter().copied().enumerate() {
        let adjacent_cjk = prev.is_some_and(cjk) || next_cjk[i];
        if !c.is_whitespace() {
            prev = Some(c);
        }
        while protected_i < protected.len() && byte >= protected[protected_i].end {
            protected_i += 1;
        }
        if protected
            .get(protected_i)
            .is_some_and(|range| range.contains(&byte))
        {
            out.push(c);
            continue;
        }
        // Quote state spans lines and protected URLs/code/formulas. Chinese
        // mode also converts an English quotation inside Chinese prose. Only
        // a literal apostrophe between ASCII word characters is excluded.
        let apostrophe = matches!(c, '\'' | '’')
            && i > 0
            && chars[i - 1].1.is_ascii_alphanumeric()
            && chars
                .get(i + 1)
                .is_some_and(|(_, n)| n.is_ascii_alphanumeric());
        let quote = if chinese && !apostrophe {
            match c {
                '"' => {
                    let mark = if quotes.double_open { '”' } else { '“' };
                    quotes.double_open = !quotes.double_open;
                    Some(mark)
                }
                '\'' => {
                    let mark = if quotes.single_open { '’' } else { '‘' };
                    quotes.single_open = !quotes.single_open;
                    Some(mark)
                }
                '“' => {
                    quotes.double_open = true;
                    Some(c)
                }
                '”' => {
                    quotes.double_open = false;
                    Some(c)
                }
                '‘' => {
                    quotes.single_open = true;
                    Some(c)
                }
                '’' => {
                    quotes.single_open = false;
                    Some(c)
                }
                _ => None,
            }
        } else {
            None
        };
        let mark = if let Some(quote) = quote {
            quote
        } else if chinese && adjacent_cjk {
            match c {
                ',' => '，',
                '.' => '。',
                ':' => '：',
                ';' => '；',
                '!' => '！',
                '?' => '？',
                '(' => '（',
                ')' => '）',
                _ => c,
            }
        } else if !chinese {
            match c {
                '，' => ',',
                '。' => '.',
                '：' => ':',
                '；' => ';',
                '！' => '!',
                '？' => '?',
                '（' => '(',
                '）' => ')',
                '“' | '”' => '"',
                '‘' | '’' => '\'',
                _ => c,
            }
        } else {
            c
        };
        out.push(mark);
    }
}
pub fn convert_chinese(input: &str, traditional: bool) -> Result<String, String> {
    // A user action owns the dictionary; it is released after conversion.
    OpenCC::from_config(if traditional {
        BuiltinConfig::S2t
    } else {
        BuiltinConfig::T2s
    })
    .map(|cc| cc.convert(input))
    .map_err(|_| "简繁词典加载失败。".into())
}
