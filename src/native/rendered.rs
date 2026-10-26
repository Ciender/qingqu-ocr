//! Optional CPU/native preview. The editable/copyable OCR source stays untouched.
use super::*;
use pulldown_cmark::{Event, Options, Parser, Tag, TagEnd};
use windows::{
    core::{IUnknown, Interface, BSTR},
    Win32::{
        System::Ole::{OleInitialize, OleUninitialize},
        UI::Controls::RichEdit::{ITextDocument2, EM_GETOLEINTERFACE},
    },
};

const MAX_PREVIEW: usize = 256 * 1024;
const MAX_SPANS: usize = 8192;
pub(super) const COPY_SOURCE_PROP: &str = "QingquCopyMathSource";
// Newer RichEdit supports this flag even though older tom.h enum tables omit it.
const TOM_CONVERT_TEX: i32 = 0x0020_0000;

#[derive(Clone, Copy, Default, Debug, PartialEq)]
struct Style {
    heading: u8,
    bold: bool,
    italic: bool,
    code: bool,
    strike: bool,
    display_math: bool,
}
#[derive(Debug)]
struct Span {
    start: i32,
    end: i32,
    math: bool,
    style: Style,
    text: String,
}
#[derive(Default)]
struct Plan {
    text: String,
    spans: Vec<Span>,
    end: i32,
}
fn delimiter_position(source: &str, delimiter: &str) -> Option<usize> {
    source.match_indices(delimiter).map(|(i, _)| i).find(|&i| {
        source.as_bytes()[..i]
            .iter()
            .rev()
            .take_while(|&&b| b == b'\\')
            .count()
            % 2
            == 0
    })
}

struct MarkdownMath {
    latex: String,
    display: bool,
}

/// Return the contents and end offset of a balanced TeX group.
///
/// OCR services commonly put whitespace between a command and its argument
/// (`\\frac { a } { b }`) and around superscript groups (`x ^ { 2 }`).
/// RichEdit accepts some of these spellings inconsistently across Windows
/// builds, so the preview path canonicalizes the syntax while keeping the
/// source editor and copy operation byte-for-byte unchanged.
fn balanced_group(source: &str, open: usize) -> Option<(&str, usize)> {
    if source.as_bytes().get(open) != Some(&b'{') {
        return None;
    }
    let mut depth = 0usize;
    let mut escaped = false;
    for (offset, ch) in source[open..].char_indices() {
        if escaped {
            escaped = false;
            continue;
        }
        if ch == '\\' {
            escaped = true;
            continue;
        }
        match ch {
            '{' => depth += 1,
            '}' => {
                depth = depth.checked_sub(1)?;
                if depth == 0 {
                    let begin = open + 1;
                    let end = open + offset;
                    return Some((&source[begin..end], end + 1));
                }
            }
            _ => {}
        }
    }
    None
}

/// Canonicalize only the syntax used by the renderer.
///
/// This is deliberately a small lexical pass rather than a TeX parser: OCR
/// output can contain macros that RichEdit does not implement and those must
/// still reach the native converter unchanged. We remove whitespace between a
/// control word and its braced argument, trim group edges, and normalize
/// whitespace immediately after `^`/`_`. Internal whitespace (including in
/// `\\text{...}`) is preserved.
fn normalize_latex(source: &str) -> String {
    normalize_latex_at_depth(source, 0)
}

fn normalize_latex_at_depth(source: &str, depth: usize) -> String {
    // A malformed OCR response must not exhaust the UI thread's stack.
    if depth >= 64 {
        return source.into();
    }
    let mut output = String::with_capacity(source.len());
    let mut cursor = 0usize;
    while cursor < source.len() {
        let (offset, ch) = source[cursor..].char_indices().next().unwrap();
        let at = cursor + offset;
        match ch {
            '^' | '_' => {
                while output.chars().last().is_some_and(char::is_whitespace) {
                    output.pop();
                }
                output.push(ch);
                cursor = at + ch.len_utf8();
                while let Some(next) = source[cursor..].chars().next() {
                    if !next.is_whitespace() {
                        break;
                    }
                    cursor += next.len_utf8();
                }
                if source.as_bytes().get(cursor) == Some(&b'{') {
                    if let Some((body, end)) = balanced_group(source, cursor) {
                        output.push('{');
                        output.push_str(normalize_latex_at_depth(body, depth + 1).trim());
                        output.push('}');
                        cursor = end;
                    }
                }
            }
            '\\' => {
                output.push('\\');
                cursor = at + 1;
                let command_start = cursor;
                while let Some(next) = source[cursor..].chars().next() {
                    if !next.is_ascii_alphabetic() {
                        break;
                    }
                    cursor += next.len_utf8();
                }
                if cursor == command_start {
                    // Escaped punctuation (`\\{`, `\\_`, `\\\\`) is copied as
                    // a unit so the following character is not treated as a
                    // group or a subscript marker.
                    if let Some(next) = source[cursor..].chars().next() {
                        output.push(next);
                        cursor += next.len_utf8();
                    }
                } else {
                    let command = &source[command_start..cursor];
                    output.push_str(command);
                    let mut lookahead = cursor;
                    while let Some(next) = source[lookahead..].chars().next() {
                        if !next.is_whitespace() {
                            break;
                        }
                        lookahead += next.len_utf8();
                    }
                    if source.as_bytes().get(lookahead) == Some(&b'{') {
                        if let Some((body, end)) = balanced_group(source, lookahead) {
                            output.push('{');
                            if matches!(
                                command,
                                "text"
                                    | "textrm"
                                    | "textit"
                                    | "textbf"
                                    | "textsf"
                                    | "texttt"
                                    | "textnormal"
                                    | "mbox"
                                    | "operatorname"
                            ) {
                                output.push_str(body);
                            } else {
                                output.push_str(normalize_latex_at_depth(body, depth + 1).trim());
                            }
                            output.push('}');
                            cursor = end;
                        }
                    }
                }
            }
            '{' => {
                if let Some((body, end)) = balanced_group(source, at) {
                    // A command's braced arguments are lexical groups, so
                    // whitespace before the opening brace is insignificant.
                    while output.chars().last().is_some_and(char::is_whitespace) {
                        output.pop();
                    }
                    output.push('{');
                    output.push_str(normalize_latex_at_depth(body, depth + 1).trim());
                    output.push('}');
                    cursor = end;
                } else {
                    output.push(ch);
                    cursor = at + ch.len_utf8();
                }
            }
            _ => {
                output.push(ch);
                cursor = at + ch.len_utf8();
            }
        }
    }
    output
}

/// Protect TeX before Markdown consumes \( / \[ as punctuation escapes or
/// interprets underscores/asterisks inside equations as Markdown formatting.
/// Opaque markers avoid converting through dollar delimiters (which can occur
/// inside TeX text). Code, existing dollar math, HTML and link targets stay raw.
fn prepare_markdown_math(source: &str, options: Options) -> (String, Vec<MarkdownMath>, String) {
    let mut protected: Vec<_> = Parser::new_ext(source, options)
        .into_offset_iter()
        .filter_map(|(event, range)| {
            matches!(
                event,
                Event::Code(_)
                    | Event::Html(_)
                    | Event::InlineHtml(_)
                    | Event::InlineMath(_)
                    | Event::DisplayMath(_)
                    | Event::Start(Tag::CodeBlock(_) | Tag::Link { .. } | Tag::Image { .. })
            )
            .then_some(range)
        })
        .collect();
    protected.sort_by_key(|range| range.start);
    let mut merged: Vec<std::ops::Range<usize>> = Vec::new();
    for range in protected {
        if let Some(last) = merged.last_mut().filter(|last| range.start <= last.end) {
            last.end = last.end.max(range.end);
        } else {
            merged.push(range);
        }
    }
    let mut marker = "\u{e000}qingqu-math-".to_owned();
    while source.contains(&marker) {
        marker.push('-');
    }
    let mut output = String::new();
    let mut math = Vec::new();
    let mut unmatched = [false; 2];
    let mut scan = 0;
    let mut copied = 0;
    while let Some((begin, close, display)) = [(r"\(", r"\)", false), (r"\[", r"\]", true)]
        .into_iter()
        .filter_map(|(open, close, display)| {
            if unmatched[display as usize] {
                return None;
            }
            delimiter_position(&source[scan..], open).map(|i| (scan + i, close, display))
        })
        .min_by_key(|(i, _, _)| *i)
    {
        let blocked = merged.partition_point(|range| range.end <= begin);
        if let Some(range) = merged.get(blocked).filter(|range| range.start <= begin) {
            scan = range.end;
            continue;
        }
        let body_start = begin + 2;
        let Some(end) = delimiter_position(&source[body_start..], close).map(|i| body_start + i)
        else {
            unmatched[display as usize] = true;
            scan = body_start;
            continue;
        };
        // Don't let an unmatched opener swallow a later code block, HTML or
        // another protected region while looking for its closing delimiter.
        if merged
            .get(blocked)
            .is_some_and(|range| range.start < end + 2)
        {
            scan = body_start;
            continue;
        }
        output.push_str(&source[copied..begin]);
        output.push_str(&format!("{marker}{}\u{e001}", math.len()));
        math.push(MarkdownMath {
            latex: source[body_start..end].to_owned(),
            display,
        });
        scan = end + 2;
        copied = scan;
    }
    output.push_str(&source[copied..]);
    (output, math, marker)
}
impl Plan {
    fn push(&mut self, text: &str, style: Style, math: bool) {
        let text = text.replace("\r\n", "\n").replace(['\n', '\r'], "\r");
        if text.is_empty() {
            return;
        }
        let start = self.end;
        self.end += text.encode_utf16().count() as i32;
        self.text.push_str(&text);
        self.spans.push(Span {
            start,
            end: self.end,
            math,
            style,
            text,
        });
    }
    fn newline(&mut self) {
        if !self.text.ends_with('\r') {
            self.push("\r", Style::default(), false);
        }
    }
    fn formula(source: &str) -> Self {
        let mut plan = Self::default();
        let mut rest = source;
        // Mathpix mixes prose with \(...\) / \[...\]. Also accept $/$$.
        while let Some((offset, open, close)) =
            [(r"\[", r"\]"), (r"\(", r"\)"), ("$$", "$$"), ("$", "$")]
                .into_iter()
                .filter_map(|(open, close)| {
                    delimiter_position(rest, open).map(|i| (i, open, close))
                })
                .min_by_key(|(i, _, _)| *i)
        {
            let Some(end) = delimiter_position(&rest[offset + open.len()..], close) else {
                break;
            };
            plan.push(&rest[..offset], Style::default(), false);
            let begin = offset + open.len();
            plan.push(
                &rest[begin..begin + end],
                Style {
                    display_math: matches!(open, r"\[" | "$$"),
                    ..Style::default()
                },
                true,
            );
            rest = &rest[begin + end + close.len()..];
        }
        if plan.spans.is_empty() {
            // Baidu and single-formula Mathpix return unwrapped LaTeX.
            for (index, line) in rest.split('\n').enumerate() {
                if index > 0 {
                    plan.push("\r", Style::default(), false);
                }
                let math = line.contains(['\\', '^', '_', '=']);
                plan.push(
                    line,
                    Style {
                        display_math: math,
                        ..Style::default()
                    },
                    math,
                );
            }
        } else {
            plan.push(rest, Style::default(), false);
        }
        plan
    }
    fn markdown(source: &str) -> Self {
        let mut plan = Self::default();
        let mut style = Style::default();
        let mut stack = Vec::new();
        let mut lists: Vec<Option<u64>> = Vec::new();
        let mut options =
            Options::ENABLE_MATH | Options::ENABLE_TABLES | Options::ENABLE_STRIKETHROUGH;
        options.insert(Options::ENABLE_TASKLISTS);
        let (prepared, math, marker) = prepare_markdown_math(source, options);
        for event in Parser::new_ext(&prepared, options) {
            match event {
                Event::Start(tag) => {
                    stack.push(style);
                    match tag {
                        Tag::Heading { level, .. } => {
                            plan.newline();
                            style.heading = level as u8;
                            style.bold = true;
                        }
                        Tag::Strong => style.bold = true,
                        Tag::Emphasis => style.italic = true,
                        Tag::Strikethrough => style.strike = true,
                        Tag::CodeBlock(_) => {
                            plan.newline();
                            style.code = true;
                        }
                        Tag::List(start) => {
                            plan.newline();
                            lists.push(start);
                        }
                        Tag::Item => {
                            plan.newline();
                            let prefix = if let Some(Some(n)) = lists.last_mut() {
                                let text = format!("{n}. ");
                                *n += 1;
                                text
                            } else {
                                "• ".into()
                            };
                            plan.push(&prefix, style, false);
                        }
                        Tag::BlockQuote(_) => {
                            plan.newline();
                            plan.push("│ ", style, false);
                        }
                        Tag::TableHead => style.bold = true,
                        _ => {}
                    }
                }
                Event::End(tag) => {
                    if matches!(
                        tag,
                        TagEnd::Paragraph
                            | TagEnd::Heading(_)
                            | TagEnd::CodeBlock
                            | TagEnd::Item
                            | TagEnd::TableHead
                            | TagEnd::TableRow
                            | TagEnd::BlockQuote(_)
                    ) {
                        plan.newline();
                    }
                    if tag == TagEnd::TableCell {
                        plan.push("\t", style, false);
                    }
                    if matches!(tag, TagEnd::List(_)) {
                        lists.pop();
                    }
                    style = stack.pop().unwrap_or_default();
                }
                Event::Text(text) => plan.markdown_text(&text, style, &math, &marker),
                Event::Code(text) => plan.push(
                    &text,
                    Style {
                        code: true,
                        ..style
                    },
                    false,
                ),
                Event::InlineMath(text) => plan.push(&text, style, true),
                Event::DisplayMath(text) => {
                    plan.newline();
                    plan.push(
                        &text,
                        Style {
                            display_math: true,
                            ..style
                        },
                        true,
                    );
                    plan.newline();
                }
                Event::SoftBreak => plan.push(" ", style, false),
                Event::HardBreak => plan.push("\r", style, false),
                Event::Rule => {
                    plan.newline();
                    plan.push("────────────\r", style, false);
                }
                Event::TaskListMarker(checked) => {
                    plan.push(if checked { "☑ " } else { "☐ " }, style, false)
                }
                // HTML stays visible literal text. No browser, scripts, URLs or images are loaded.
                Event::Html(text) | Event::InlineHtml(text) => plan.push(&text, style, false),
                _ => {}
            }
        }
        plan
    }

    fn markdown_text(&mut self, text: &str, style: Style, math: &[MarkdownMath], marker: &str) {
        let mut rest = text;
        while let Some(begin) = rest.find(marker) {
            self.push(&rest[..begin], style, false);
            rest = &rest[begin + marker.len()..];
            if let Some((index, tail)) = rest.split_once('\u{e001}') {
                if let Some(item) = index.parse::<usize>().ok().and_then(|i| math.get(i)) {
                    if item.display {
                        self.newline();
                    }
                    self.push(
                        &item.latex,
                        Style {
                            display_math: item.display,
                            ..style
                        },
                        true,
                    );
                    if item.display {
                        self.newline();
                    }
                    rest = tail;
                    continue;
                }
            }
            // Preserve literal content should a future parser split a marker.
            self.push(marker, style, false);
        }
        self.push(rest, style, false);
    }
}

struct Apartment;
impl Drop for Apartment {
    fn drop(&mut self) {
        unsafe {
            OleUninitialize();
        }
    }
}

/// Lazily allocated document preview used inside the main result pane.
/// The editable OCR source remains in the Scintilla plain-text editor.
pub struct DocumentView {
    pub editor: HWND,
    _apartment: Apartment,
}
impl DocumentView {
    pub fn create(h: &mut Host) -> Result<Self, String> {
        unsafe {
            OleInitialize(None).map_err(|_| "无法初始化系统公式组件。")?;
            let apartment = Apartment;
            let editor = h.rich(283, true);
            if editor.is_null() || SendMessageW(editor, SETTEXTMODE, 2 | 8, 0) != 0 {
                return Err("无法创建公式预览控件。".into());
            }
            SendMessageW(editor, SETUNDOLIMIT, 0, 0);
            SetPropW(
                editor,
                platform::wide(COPY_SOURCE_PROP).as_ptr(),
                1 as HANDLE,
            );
            ShowWindow(editor, SW_HIDE);
            Ok(Self {
                editor,
                _apartment: apartment,
            })
        }
    }
    pub fn rebuild(&self, h: &Host, source: &str, markdown: bool) -> Result<usize, String> {
        if source.len() > MAX_PREVIEW {
            return Err("内容超过 256 KiB，请选取较小片段预览。".into());
        }
        let plan = if markdown {
            Plan::markdown(source)
        } else {
            Plan::formula(source)
        };
        if plan.spans.len() > MAX_SPANS {
            return Err("文档格式片段过多，请选取较小片段预览。".into());
        }
        unsafe {
            SendMessageW(self.editor, WM_SETREDRAW, 0, 0);
            SendMessageW(self.editor, EM_SETREADONLY, 0, 0);
            edit_style(self.editor, h.theme, h.font_size, h.dpi);
            let points = h.font_size * h.dpi as f32 / GetDpiForWindow(self.editor).max(96) as f32;
            let result = render(self.editor, &plan, points);
            SendMessageW(self.editor, EM_SETREADONLY, 1, 0);
            SendMessageW(self.editor, WM_SETREDRAW, 1, 0);
            InvalidateRect(self.editor, null(), 0);
            result
        }
    }
    pub fn hide_and_clear(&self) {
        unsafe {
            ShowWindow(self.editor, SW_HIDE);
            set_text(self.editor, "");
        }
    }
}

/// Returns the number of math spans that the system couldn't convert.
unsafe fn render(editor: HWND, plan: &Plan, size: f32) -> Result<usize, String> {
    let mut raw: *mut std::ffi::c_void = null_mut();
    if SendMessageW(
        editor,
        EM_GETOLEINTERFACE,
        0,
        (&mut raw as *mut *mut std::ffi::c_void) as isize,
    ) == 0
        || raw.is_null()
    {
        return Err("系统 Rich Edit 没有提供公式接口；已保留源码。".into());
    }
    let ole = IUnknown::from_raw(raw);
    let doc: ITextDocument2 = ole
        .cast()
        .map_err(|_| "此 Windows 的 Rich Edit 不支持数学预览；已保留源码。")?;
    if plan.spans.iter().any(|span| span.math) {
        // A successful HRESULT alone is insufficient on older RichEdit builds.
        // Require a real structured radical before claiming TeX support.
        let probe = doc
            .Range2(0, i32::MAX)
            .map_err(|_| "无法检查系统公式能力。")?;
        let supported = probe
            .SetText2(TOM_CONVERT_TEX, &BSTR::from(r"\sqrt{2}"))
            .and_then(|_| probe.GetText2(0))
            .is_ok_and(|value| value.to_string().contains('\u{fdd0}'));
        if !supported {
            return Err("此 Windows 的 Rich Edit 不支持 LaTeX 排版，预览显示源码。".into());
        }
    }
    set_text(editor, &plan.text);
    let mut unsupported = 0;
    // Math insertion changes UTF-16 positions. Process from the end so earlier
    // ranges (including emoji/CJK) stay correct, and no selection is rewritten.
    for span in plan.spans.iter().rev() {
        let range = doc
            .Range2(span.start, span.end)
            .map_err(|_| "无法访问预览文本范围。")?;
        if span.math {
            let latex = if span.style.display_math {
                format!(r"\displaystyle {}", span.text)
            } else {
                span.text.clone()
            };
            let latex = normalize_latex(&latex);
            if range
                .SetText2(TOM_CONVERT_TEX, &BSTR::from(latex.as_str()))
                .is_err()
            {
                range
                    .SetText(&BSTR::from(span.text.as_str()))
                    .map_err(|_| "无法还原公式源码。")?;
                unsupported += 1;
            } else {
                // Old Windows builds may return success but leave TeX literal.
                // Keep it visible and flag the unsupported macro/conversion.
                let text = range
                    .GetText2(0)
                    .map_err(|_| "无法检查公式转换结果。")?
                    .to_string();
                if text.contains(['\\', '^'])
                    || (!span.text.trim().is_empty() && text.trim().is_empty())
                {
                    range
                        .SetText(&BSTR::from(span.text.as_str()))
                        .map_err(|_| "无法还原公式源码。")?;
                    unsupported += 1;
                }
            }
            range
                .GetFont()
                .and_then(|font| font.SetSize(size))
                .map_err(|_| "无法设置公式字号。")?;
        } else {
            let font = range.GetFont().map_err(|_| "无法设置预览字体。")?;
            font.SetBold(if span.style.bold { -1 } else { 0 })
                .map_err(|_| "无法设置预览字重。")?;
            font.SetItalic(if span.style.italic { -1 } else { 0 })
                .map_err(|_| "无法设置预览斜体。")?;
            font.SetStrikeThrough(if span.style.strike { -1 } else { 0 })
                .map_err(|_| "无法设置删除线。")?;
            font.SetName(&BSTR::from(if span.style.code {
                "Consolas"
            } else {
                "Microsoft YaHei UI"
            }))
            .map_err(|_| "无法设置预览字体。")?;
            font.SetSize(
                size * match span.style.heading {
                    1 => 1.7,
                    2 => 1.45,
                    3 => 1.25,
                    4..=6 => 1.1,
                    _ => 1.,
                },
            )
            .map_err(|_| "无法设置预览字号。")?;
        }
    }
    SendMessageW(editor, EM_SETSEL, 0, 0);
    SendMessageW(editor, WM_VSCROLL, SB_TOP as usize, 0);
    Ok(unsupported)
}
