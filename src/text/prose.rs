//! Small prose cleanup, separate from layout inference and spelling changes.
use regex::Regex;
use std::sync::LazyLock;

/// `textwrap::unfill` is useful after paragraph boundaries are known. Its
/// English word spacing must not be applied to CJK, URLs, or code tokens.
pub(super) fn repair(rows: &[&str]) -> String {
    if rows.len() > 1
        && rows.iter().all(|row| {
            row.is_ascii()
                && !row.trim_end().ends_with('-')
                && !row.contains(['`', '$', '\\', '@'])
                && !row.contains("://")
                && !row.trim_start().starts_with(['#', '>', '/', '*', '+', '-'])
        })
    {
        return textwrap::unfill(&rows.join("\n"))
            .0
            .trim_end_matches('\n')
            .to_owned();
    }
    let mut output = String::new();
    for row in rows {
        // An explicit soft hyphen is a typesetting wrap marker. A visible
        // hyphen may be meaningful (well-known), so it is retained.
        super::join_line(&mut output, row.trim());
    }
    output
}

/// Normalize horizontal prose spacing while retaining the user's mixed-script
/// spacing choices. Inline code, math, URLs, and emails are copied verbatim.
pub(super) fn clean(input: &str) -> String {
    static LITERAL: LazyLock<Regex> =
        LazyLock::new(|| Regex::new(r"https?://\S+|[^\s`$]+@[^\s`$]+|`[^`]*`|\$[^$]*\$").unwrap());
    let mut output = String::with_capacity(input.len());
    let mut start = 0;
    for literal in LITERAL.find_iter(input) {
        clean_span(&input[start..literal.start()], &mut output);
        output.push_str(literal.as_str());
        start = literal.end();
    }
    clean_span(&input[start..], &mut output);
    output.trim().to_owned()
}

fn clean_span(input: &str, output: &mut String) {
    for ch in input.chars() {
        match ch {
            '\u{200b}' | '\u{feff}' => {}
            c if c.is_whitespace() => {
                if !output.is_empty() && !output.ends_with(' ') {
                    output.push(' ');
                }
            }
            _ => output.push(ch),
        }
    }
}
