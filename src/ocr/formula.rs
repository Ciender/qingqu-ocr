//! Formula response parsing, including document text and formula reconciliation.
use serde_json::Value;

#[derive(Clone, Copy, Debug)]
struct Bounds {
    left: f64,
    top: f64,
    width: f64,
    height: f64,
}
impl Bounds {
    fn from_item(item: &Value) -> Option<Self> {
        let value = item.get("form_location").or_else(|| item.get("location"))?;
        let number = |key| value.get(key).and_then(Value::as_f64);
        let bounds = Self {
            left: number("left")?,
            top: number("top")?,
            width: number("width")?,
            height: number("height")?,
        };
        (bounds.width > 0.
            && bounds.height > 0.
            && [bounds.left, bounds.top, bounds.width, bounds.height]
                .into_iter()
                .all(f64::is_finite))
        .then_some(bounds)
    }
    fn right(self) -> f64 {
        self.left + self.width
    }
    fn bottom(self) -> f64 {
        self.top + self.height
    }
    fn overlaps(self, other: Self) -> bool {
        // Formula boxes may describe only the numerator's baseline. A small
        // vertical tolerance also catches flattened denominator fragments.
        let margin = (self.height * 0.2).min(other.height * 0.6);
        self.left < other.right()
            && other.left < self.right()
            && self.top - margin < other.bottom()
            && other.top < self.bottom() + margin
    }
    fn same_line(self, other: Self) -> bool {
        other.left > self.left
            && (self.bottom().min(other.bottom()) - self.top.max(other.top))
                >= self.height.min(other.height) * 0.4
    }
    fn slice(self, start: f64, end: f64) -> Self {
        Self {
            left: self.left + self.width * start,
            width: self.width * (end - start),
            ..self
        }
    }
}

fn content(item: &Value) -> Option<&str> {
    ["form_words", "latex", "formula", "words"]
        .iter()
        .filter_map(|key| item.get(key).and_then(Value::as_str))
        .find(|value| !value.trim().is_empty())
        .or_else(|| item.as_str().filter(|s| !s.trim().is_empty()))
}
fn is_tex(text: &str) -> bool {
    text.contains(['\\', '^', '_'])
}
fn wrapped(text: &str) -> String {
    if [r"\(", r"\[", "$"].iter().any(|mark| text.contains(mark)) {
        text.into()
    } else {
        format!(r"\({}\)", text.trim())
    }
}

struct Word<'a> {
    text: &'a str,
    bounds: Option<Bounds>,
    removed: Vec<std::ops::Range<usize>>,
}
struct Piece {
    text: String,
    bounds: Option<Bounds>,
    order: f64,
}

fn join(pieces: impl IntoIterator<Item = Piece>) -> (String, usize) {
    let mut output = String::new();
    let mut previous: Option<Bounds> = None;
    for piece in pieces {
        if piece.text.trim().is_empty() {
            continue;
        }
        if !output.is_empty() {
            output.push(
                if previous
                    .zip(piece.bounds)
                    .is_some_and(|(a, b)| a.same_line(b))
                {
                    ' '
                } else {
                    '\n'
                },
            );
        }
        output.push_str(piece.text.trim());
        previous = piece.bounds;
    }
    let count = output.lines().count();
    (output, count)
}

/// Identify flat mathematical tokens without swallowing neighboring prose.
/// Exact character boxes are not supplied by doc_analysis, so only these runs
/// are eligible for coordinate-based replacement inside a mixed OCR line.
fn math_runs(text: &str) -> Vec<std::ops::Range<usize>> {
    fn math_token(token: &str) -> bool {
        token.chars().any(|c| {
            c.is_ascii_digit()
                || matches!(
                    c,
                    '=' | '+'
                        | '-'
                        | '−'
                        | '*'
                        | '/'
                        | '^'
                        | '_'
                        | '<'
                        | '>'
                        | '±'
                        | '√'
                        | '×'
                        | '÷'
                        | '∑'
                        | '∫'
                        | '∞'
                        | '∂'
                )
                || ('\u{0370}'..='\u{03ff}').contains(&c)
        }) || (token.chars().filter(|c| c.is_alphabetic()).count() == 1)
            || matches!(token, "sin" | "cos" | "tan" | "log" | "ln" | "lim" | "sqrt")
    }
    fn boundary(c: char) -> bool {
        ('\u{4e00}'..='\u{9fff}').contains(&c)
            || matches!(
                c,
                '：' | ':' | '，' | ',' | '。' | ';' | '；' | '！' | '!' | '？' | '?'
            )
    }
    let mut tokens = Vec::new();
    let mut begin = None;
    for (index, ch) in text
        .char_indices()
        .chain(std::iter::once((text.len(), '\n')))
    {
        let sentence_period = ch == '.'
            && !(text[..index]
                .chars()
                .next_back()
                .is_some_and(|c| c.is_ascii_digit())
                && text[index + 1..]
                    .chars()
                    .next()
                    .is_some_and(|c| c.is_ascii_digit()));
        if ch.is_whitespace() || boundary(ch) || sentence_period {
            if let Some(start) = begin.take() {
                if math_token(&text[start..index]) {
                    tokens.push(start..index);
                }
            }
        } else {
            begin.get_or_insert(index);
        }
    }
    let mut runs: Vec<std::ops::Range<usize>> = Vec::new();
    for token in tokens {
        if let Some(last) = runs.last_mut() {
            if text[last.end..token.start].chars().all(char::is_whitespace) {
                last.end = token.end;
                continue;
            }
        }
        runs.push(token);
    }
    runs
}

fn advance(ch: char) -> f64 {
    if ch.is_ascii() {
        0.55
    } else {
        1.
    }
}
fn fraction(text: &str, offset: usize) -> f64 {
    let total = text.chars().map(advance).sum::<f64>();
    if total == 0. {
        0.
    } else {
        text[..offset].chars().map(advance).sum::<f64>() / total
    }
}

/// Parse the fused formula endpoint or reconcile doc_analysis's two result
/// arrays. Fused words_result is authoritative: adding formula_result again
/// would duplicate every equation. For a document response, only overlapping
/// flat math fragments are replaced; surrounding paragraphs remain present.
pub(crate) fn parse_baidu_formula(body: &Value) -> Result<(String, usize), String> {
    super::check_error(body)?;
    let word_items = body.get("words_result").and_then(Value::as_array);
    let formula_items = body.get("formula_result").and_then(Value::as_array);
    if word_items.is_none() && formula_items.is_none() {
        return Err("百度公式响应缺少 words_result／formula_result。".into());
    }
    let mut words = word_items
        .into_iter()
        .flatten()
        .map(|item| {
            Ok(Word {
                text: content(item).ok_or("百度公式结果格式无法识别。")?,
                bounds: Bounds::from_item(item),
                removed: Vec::new(),
            })
        })
        .collect::<Result<Vec<_>, String>>()?;
    // doc_analysis identifies its independent equations by form_words. An
    // underscore in neighboring source code is not evidence that its ordinary
    // words_result has already fused in those equations.
    let separate_document_formulas = formula_items
        .is_some_and(|items| items.iter().any(|item| item.get("form_words").is_some()));
    let fused = !separate_document_formulas && words.iter().any(|word| is_tex(word.text));
    if fused || formula_items.is_none_or(|items| items.is_empty()) {
        return Ok(join(words.into_iter().map(|word| {
            Piece {
                text: if fused
                    && body.get("formula_result").is_some()
                    && is_tex(word.text)
                    && !word
                        .text
                        .chars()
                        .any(|c| ('\u{4e00}'..='\u{9fff}').contains(&c))
                {
                    wrapped(word.text)
                } else {
                    word.text.into()
                },
                bounds: word.bounds,
                order: 0.,
            }
        })));
    }
    let formulas = formula_items
        .unwrap()
        .iter()
        .map(|item| {
            Ok((
                content(item).ok_or("百度公式结构格式无法识别。")?,
                Bounds::from_item(item),
            ))
        })
        .collect::<Result<Vec<_>, String>>()?;
    if words.is_empty() {
        return Ok((
            formulas
                .iter()
                .map(|(text, _)| *text)
                .collect::<Vec<_>>()
                .join("\n"),
            formulas.len(),
        ));
    }
    let mut pieces = Vec::new();
    for (index, (latex, bounds)) in formulas.iter().enumerate() {
        let mut anchor: Option<f64> = None;
        for (word_index, word) in words.iter_mut().enumerate() {
            let matching_bounds = bounds.zip(word.bounds).filter(|(a, b)| a.overlaps(*b));
            if let Some((formula_box, word_box)) = matching_bounds {
                for range in math_runs(word.text) {
                    let start = fraction(word.text, range.start);
                    let end = fraction(word.text, range.end);
                    if formula_box.overlaps(word_box.slice(start, end)) {
                        anchor = Some(
                            anchor
                                .unwrap_or(f64::MAX)
                                .min(word_index as f64 + start * 0.8),
                        );
                        word.removed.push(range);
                    }
                }
            } else if bounds.is_none() && word.text.trim() == latex.trim() {
                anchor = Some(word_index as f64);
                word.removed.push(0..word.text.len());
            }
        }
        let order = anchor.unwrap_or_else(|| {
            // Standalone formula blocks need an insertion point even when no
            // ordinary OCR fragment overlaps them. Keep source order for
            // responses without coordinates.
            bounds
                .and_then(|formula_box| {
                    words.iter().enumerate().find_map(|(i, word)| {
                        let word_box = word.bounds?;
                        (word_box.top > formula_box.top + formula_box.height * 0.5
                            || (formula_box.same_line(word_box)
                                && word_box.left > formula_box.left))
                            .then_some(i as f64 - 0.1)
                    })
                })
                .unwrap_or(words.len() as f64 + index as f64)
        });
        pieces.push(Piece {
            text: wrapped(latex),
            bounds: *bounds,
            order,
        });
    }
    for (index, mut word) in words.into_iter().enumerate() {
        word.removed.sort_by_key(|range| range.start);
        let mut cursor = 0;
        for range in word
            .removed
            .into_iter()
            .chain(std::iter::once(word.text.len()..word.text.len()))
        {
            if range.start > cursor {
                let start = fraction(word.text, cursor);
                let end = fraction(word.text, range.start);
                pieces.push(Piece {
                    text: word.text[cursor..range.start].into(),
                    bounds: word.bounds.map(|bounds| bounds.slice(start, end)),
                    order: index as f64 + start * 0.8,
                });
            }
            cursor = cursor.max(range.end);
        }
    }
    pieces.sort_by(|a, b| a.order.total_cmp(&b.order));
    Ok(join(pieces))
}

pub(crate) fn parse_mathpix(body: &Value) -> Result<String, String> {
    let nonempty = |key| {
        body.get(key)
            .and_then(Value::as_str)
            .filter(|text| !text.trim().is_empty())
    };
    let text = nonempty("text");
    let latex = nonempty("latex_styled");
    // Mathpix's text preserves prose and math delimiters; latex_styled may
    // contain only the extracted equation, so it must not hide that prose.
    let mixed = text.is_some_and(|text| {
        let mut rest = text;
        let mut prose = String::new();
        let mut found_math = false;
        loop {
            let next = [(r"\(", r"\)"), (r"\[", r"\]"), ("$$", "$$"), ("$", "$")]
                .into_iter()
                .filter_map(|(open, close)| rest.find(open).map(|at| (at, open, close)))
                .min_by_key(|(at, _, _)| *at);
            let Some((at, open, close)) = next else {
                prose.push_str(rest);
                break;
            };
            let after = &rest[at + open.len()..];
            let Some(end) = after.find(close) else {
                prose.push_str(rest);
                break;
            };
            prose.push_str(&rest[..at]);
            found_math = true;
            rest = &after[end + close.len()..];
        }
        found_math && prose.chars().any(|c| c.is_alphanumeric())
    });
    (if mixed {
        text.or(latex)
    } else {
        latex.or(text)
    })
    .map(str::to_owned)
    .ok_or_else(|| "公式服务未返回非空 LaTeX 或公式文本。".into())
}
