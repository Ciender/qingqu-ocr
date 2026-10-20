//! Shared, model-free OCR layout reconstruction.
//!
//! Combines Umi-OCR's gap-tree/paragraph-alignment approach with local leading,
//! ragged paragraph ends, protected structures, and optional provider hints.
//! Provider groups are evidence, never the authority over geometric structure.
//! Adaptation notices: vendor/text-layout/SOURCE.md.
#[path = "layout/gap_tree.rs"]
mod gap_tree;

use super::{block_marker, join_line, structured, FenceState, PositionedLine};

#[derive(Clone, Copy, Debug)]
struct Rect {
    left: f64,
    top: f64,
    right: f64,
    bottom: f64,
}
impl Rect {
    fn from_line(line: &PositionedLine) -> Self {
        Self {
            left: line.x,
            top: line.y,
            right: line.x + line.width,
            bottom: line.y + line.height,
        }
    }
    fn height(self) -> f64 {
        self.bottom - self.top
    }
    fn width(self) -> f64 {
        self.right - self.left
    }
    fn union(self, other: Self) -> Self {
        Self {
            left: self.left.min(other.left),
            top: self.top.min(other.top),
            right: self.right.max(other.right),
            bottom: self.bottom.max(other.bottom),
        }
    }
}

#[derive(Clone)]
struct Row {
    text: String,
    rect: Rect,
    group: Option<usize>,
}

pub(super) fn valid_box(line: &PositionedLine) -> bool {
    [
        line.x,
        line.y,
        line.width,
        line.height,
        line.x + line.width,
        line.y + line.height,
    ]
    .iter()
    .all(|value| value.is_finite())
        && line.width > 0.
        && line.height > 0.
}

/// Internal layout representation: physical wraps use LF and paragraph
/// boundaries use a blank row. Display spacing is applied separately.
pub(super) fn recover(lines: &[PositionedLine], groups: &[Vec<usize>]) -> String {
    if lines.is_empty() {
        return String::new();
    }
    if !lines.iter().all(valid_box) {
        // Incomplete geometry is not permission to invent a full-width page.
        // Preserve all text and let the plain-text formatter handle it.
        let fenced = lines.iter().any(|line| block_marker(&line.text));
        return lines
            .iter()
            .map(|line| line.text.as_str())
            .collect::<Vec<_>>()
            .join(if fenced { "\n" } else { "\n\n" });
    }
    let hints = group_map(lines.len(), groups);
    let boxes: Vec<_> = lines.iter().map(Rect::from_line).collect();
    let literal = lines
        .iter()
        .any(|line| block_marker(&line.text) || table_row(&line.text));
    let regions = if literal {
        vec![(0..lines.len()).collect()]
    } else {
        gap_tree::regions(&boxes)
    };
    let mut output = String::new();
    for region in regions {
        let mut rows: Vec<_> = region
            .into_iter()
            .map(|i| Row {
                text: lines[i].text.clone(),
                rect: boxes[i],
                group: hints[i],
            })
            .collect();
        if !literal {
            rows = coalesce(rows);
        }
        if !output.is_empty() {
            output.push_str("\n\n");
        }
        render_region(&rows, &mut output);
    }
    output
}

fn group_map(count: usize, groups: &[Vec<usize>]) -> Vec<Option<usize>> {
    let mut map = vec![None; count];
    for (group, indices) in groups.iter().enumerate() {
        if indices.is_empty() || indices.iter().any(|&i| i >= count || map[i].is_some()) {
            continue;
        }
        let mut unique = indices.clone();
        unique.sort_unstable();
        unique.dedup();
        if unique.len() != indices.len() {
            continue;
        }
        for &i in indices {
            map[i] = Some(group);
        }
    }
    map
}

fn table_row(text: &str) -> bool {
    text.contains('\t') || text.matches('|').count() > 1
}

fn list_start(text: &str) -> bool {
    let text = text.trim_start();
    text.starts_with(['-', '*', '•', '·', '●', '○', '◦', '▪', '‣'])
        || (structured(text)
            && !text.starts_with(['#', '第'])
            && !table_row(text)
            && !block_marker(text))
}

fn coalesce(rows: Vec<Row>) -> Vec<Row> {
    let mut output = Vec::with_capacity(rows.len());
    let mut fragments: Vec<Row> = Vec::new();
    let flush = |fragments: &mut Vec<Row>, output: &mut Vec<Row>| {
        fragments.sort_by(|a, b| a.rect.left.total_cmp(&b.rect.left));
        let mut current: Option<Row> = None;
        for fragment in fragments.drain(..) {
            if let Some(row) = current.as_mut() {
                let height = row.rect.height().min(fragment.rect.height());
                let gap = fragment.rect.left - row.rect.right;
                if (-height * 0.2..=height * 1.25).contains(&gap) {
                    join_line(&mut row.text, fragment.text.trim());
                    row.rect = row.rect.union(fragment.rect);
                    if row.group != fragment.group {
                        row.group = None;
                    }
                    continue;
                }
                output.push(current.take().unwrap());
            }
            current = Some(fragment);
        }
        if let Some(row) = current {
            output.push(row);
        }
    };
    for row in rows {
        let same_row = fragments.first().is_none_or(|first| {
            let height = first.rect.height().min(row.rect.height());
            (first.rect.bottom - row.rect.bottom).abs() <= height * 0.35
                && first.rect.height().max(row.rect.height()) <= height * 1.5
        });
        if !same_row {
            flush(&mut fragments, &mut output);
        }
        fragments.push(row);
    }
    flush(&mut fragments, &mut output);
    output
}

fn sentence_end(text: &str) -> bool {
    text.trim_end()
        .trim_end_matches(['”', '’', '"', '\'', ')', '）', ']', '】'])
        .ends_with(['。', '！', '？', '!', '?', '.', '…'])
}

/// Estimate leading from nearby rows of similar size. Small captions cannot
/// redefine normal leading for large body text elsewhere on the same page.
fn leading(rows: &[Row], at: usize, height: f64) -> f64 {
    let start = at.saturating_sub(6);
    let end = (at + 7).min(rows.len());
    let mut gaps: Vec<_> = rows[start..end]
        .windows(2)
        .filter_map(|pair| {
            let a = pair[0].rect;
            let b = pair[1].rect;
            let h = a.height().min(b.height());
            let gap = b.top - a.bottom;
            (h >= height * 0.7 && h <= height * 1.4 && gap >= 0. && gap < height * 2.)
                .then_some(gap)
        })
        .collect();
    gaps.sort_by(f64::total_cmp);
    gaps.get(gaps.len().saturating_sub(1) / 2)
        .copied()
        .unwrap_or(height * 0.35)
}

fn render_region(rows: &[Row], output: &mut String) {
    if rows.is_empty() {
        return;
    }
    let left = rows
        .iter()
        .map(|row| row.rect.left)
        .fold(f64::INFINITY, f64::min);
    let right = rows
        .iter()
        .map(|row| row.rect.right)
        .fold(f64::NEG_INFINITY, f64::max);
    let mut heights: Vec<_> = rows.iter().map(|row| row.rect.height()).collect();
    heights.sort_by(f64::total_cmp);
    let typical = heights[heights.len() / 2];
    let mut fence = FenceState::default();
    let mut list_indent: Option<f64> = None;
    // Track paragraph alignment, rather than comparing every row to a single
    // page-wide font/width threshold (Umi ParagraphParse's central idea).
    let mut paragraph_left = rows[0].rect.left;
    let mut paragraph_right = rows[0].rect.right;
    for (i, row) in rows.iter().enumerate() {
        let text = row.text.trim();
        let protected = fence.protect(text) || table_row(text);
        // A small hollow bullet is sometimes recognized as CJK full stop.
        // Require list context/repetition before treating it as a marker.
        let list = list_start(text)
            || (text.starts_with('。')
                && (list_indent.is_some()
                    || rows
                        .get(i + 1)
                        .is_some_and(|next| next.text.trim_start().starts_with('。'))));
        let mut boundary = false;
        let mut list_continuation = false;
        if i > 0 {
            let previous = &rows[i - 1];
            let a = previous.rect;
            let b = row.rect;
            let h = a.height().min(b.height()).max(1.);
            let gap = b.top - a.bottom;
            let normal_gap = leading(rows, i, h);
            let new_column = b.top < a.top - h
                || ((b.left - a.left).abs() > a.width().max(b.width()) * 0.65
                    && (b.top - a.top).abs() < h * 2.);
            let heading =
                a.height().max(b.height()) > h * 1.5 && a.height().max(b.height()) > typical * 1.25;
            let large_gap = gap > (normal_gap + h * 0.55).max(h * 0.85);
            let indent = b.left - left > h * 1.3 && b.left - a.left > h;
            let short_end = right - a.right > (h * 2.).max((right - left) * 0.18)
                && sentence_end(&previous.text);
            let same_group = previous.group.is_some() && previous.group == row.group;
            let different_group = previous.group.zip(row.group).is_some_and(|(a, b)| a != b);
            let aligned = (paragraph_left - b.left).abs() <= h * 1.2
                && (paragraph_right - b.right).abs() <= h * 1.2;
            let mut score = 0.;
            if large_gap {
                score += 2.5;
            }
            if indent {
                score += 2.;
            }
            if short_end {
                score += 2.;
            }
            // A hint can settle an ambiguous extra gap after a finished line,
            // but cannot break unfinished names/clauses on its own.
            if different_group {
                score += 0.65;
                if sentence_end(&previous.text) && gap >= h * 0.55 {
                    score += 0.9;
                }
            } else if same_group && aligned {
                score -= 0.35;
            }
            list_continuation = !list
                && !protected
                && !heading
                && !large_gap
                && !new_column
                && list_indent.is_some_and(|x| b.left >= x + h * 0.5)
                && !structured(text);
            if protected || block_marker(&previous.text) || table_row(&previous.text) {
                boundary = false;
            } else {
                boundary = list
                    || (list_indent.is_some() && !list_continuation)
                    || (structured(text) && !list_continuation)
                    || (structured(&previous.text) && !list_start(&previous.text))
                    || heading
                    || new_column
                    || score >= 1.5;
            }
            if list_continuation {
                join_line(output, text);
            } else {
                output.push_str(if boundary { "\n\n" } else { "\n" });
            }
        }
        if !list_continuation {
            output.push_str(if protected { &row.text } else { text });
        }
        if list {
            list_indent = Some(row.rect.left);
        } else if !list_continuation {
            list_indent = None;
        }
        if boundary {
            paragraph_left = row.rect.left;
            paragraph_right = row.rect.right;
        } else {
            paragraph_left = (paragraph_left + row.rect.left) * 0.5;
            paragraph_right = (paragraph_right + row.rect.right) * 0.5;
        }
    }
}
