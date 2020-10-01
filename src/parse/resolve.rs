//! Resolve strings and raw blocks.

use super::{is_newline, Scanner};
use crate::syntax::{Ident, NodeRaw};

/// Resolves all escape sequences in a string.
pub fn resolve_string(string: &str) -> String {
    let mut out = String::with_capacity(string.len());
    let mut s = Scanner::new(string);

    while let Some(c) = s.eat() {
        if c != '\\' {
            out.push(c);
            continue;
        }

        let start = s.prev_index();
        match s.eat() {
            Some('\\') => out.push('\\'),
            Some('"') => out.push('"'),

            Some('n') => out.push('\n'),
            Some('t') => out.push('\t'),
            Some('u') if s.eat_if('{') => {
                // TODO: Feedback if closing brace is missing.
                let sequence = s.eat_while(|c| c.is_ascii_hexdigit());
                let _terminated = s.eat_if('}');

                if let Some(c) = resolve_hex(sequence) {
                    out.push(c);
                } else {
                    // TODO: Feedback that escape sequence is wrong.
                    out += s.eaten_from(start);
                }
            }

            // TODO: Feedback about invalid escape sequence.
            _ => out += s.eaten_from(start),
        }
    }

    out
}

/// Resolve a hexademical escape sequence into a character
/// (only the inner hex letters without braces or `\u`).
pub fn resolve_hex(sequence: &str) -> Option<char> {
    u32::from_str_radix(sequence, 16).ok().and_then(std::char::from_u32)
}

/// Resolves the language tag and trims the raw text.
pub fn resolve_raw(raw: &str, backticks: usize) -> NodeRaw {
    if backticks > 1 {
        let (tag, inner) = split_at_lang_tag(raw);
        let (lines, had_newline) = trim_and_split_raw(inner);
        NodeRaw {
            lang: Ident::new(tag),
            lines,
            inline: !had_newline,
        }
    } else {
        NodeRaw {
            lang: None,
            lines: split_lines(raw),
            inline: true,
        }
    }
}

/// Parse the lang tag and return it alongside the remaining inner raw text.
fn split_at_lang_tag(raw: &str) -> (&str, &str) {
    let mut s = Scanner::new(raw);
    (
        s.eat_until(|c| c == '`' || c.is_whitespace() || is_newline(c)),
        s.rest(),
    )
}

/// Trims raw text and splits it into lines.
///
/// Returns whether at least one newline was contained in `raw`.
fn trim_and_split_raw(raw: &str) -> (Vec<String>, bool) {
    // Trims one whitespace at end and start.
    let raw = raw.strip_prefix(' ').unwrap_or(raw);
    let raw = raw.strip_suffix(' ').unwrap_or(raw);

    let mut lines = split_lines(raw);
    let had_newline = lines.len() > 1;
    let is_whitespace = |line: &String| line.chars().all(char::is_whitespace);

    // Trims a sequence of whitespace followed by a newline at the start.
    if lines.first().map(is_whitespace).unwrap_or(false) {
        lines.remove(0);
    }

    // Trims a newline followed by a sequence of whitespace at the end.
    if lines.last().map(is_whitespace).unwrap_or(false) {
        lines.pop();
    }

    (lines, had_newline)
}

/// Splits a string into a vector of lines
/// (respecting Unicode, Unix, Mac and Windows line breaks).
pub fn split_lines(text: &str) -> Vec<String> {
    let mut s = Scanner::new(text);
    let mut line = String::new();
    let mut lines = Vec::new();

    while let Some(c) = s.eat_merging_crlf() {
        if is_newline(c) {
            lines.push(std::mem::take(&mut line));
        } else {
            line.push(c);
        }
    }

    lines.push(line);
    lines
}

#[cfg(test)]
#[rustfmt::skip]
mod tests {
    use super::*;

    #[test]
    fn test_unescape_strings() {
        fn test(string: &str, expected: &str) {
            assert_eq!(resolve_string(string), expected.to_string());
        }

        test(r#"hello world"#,  "hello world");
        test(r#"hello\nworld"#, "hello\nworld");
        test(r#"a\"bc"#,        "a\"bc");
        test(r#"a\u{2603}bc"#,  "a☃bc");
        test(r#"a\u{26c3bg"#,   "a𦰻g");
        test(r#"av\u{6797"#,    "av林");
        test(r#"a\\"#,          "a\\");
        test(r#"a\\\nbc"#,      "a\\\nbc");
        test(r#"a\tbc"#,        "a\tbc");
        test(r"🌎",             "🌎");
        test(r"🌎\",            r"🌎\");
        test(r"\🌎",            r"\🌎");
    }

    #[test]
    fn test_split_at_lang_tag() {
        fn test(raw: &str, lang: &str, inner: &str) {
            assert_eq!(split_at_lang_tag(raw), (lang, inner));
        }

        test("typst it!",   "typst", " it!");
        test("typst\n it!", "typst", "\n it!");
        test("typst\n it!", "typst", "\n it!");
        test("abc`",        "abc",   "`");
        test(" hi",         "",      " hi");
        test("`",           "",      "`");
    }

    #[test]
    fn test_trim_raw() {
        fn test(raw: &str, expected: Vec<&str>) {
            assert_eq!(trim_and_split_raw(raw).0, expected);
        }

        test(" hi",          vec!["hi"]);
        test("  hi",         vec![" hi"]);
        test("\nhi",         vec!["hi"]);
        test("    \n hi",    vec![" hi"]);
        test("hi ",          vec!["hi"]);
        test("hi  ",         vec!["hi "]);
        test("hi\n",         vec!["hi"]);
        test("hi \n   ",     vec!["hi "]);
        test("  \n hi \n  ", vec![" hi "]);
    }

    #[test]
    fn test_split_lines() {
        fn test(raw: &str, expected: Vec<&str>) {
            assert_eq!(split_lines(raw), expected);
        }

        test("raw\ntext",  vec!["raw", "text"]);
        test("a\r\nb",     vec!["a", "b"]);
        test("a\n\nb",     vec!["a", "", "b"]);
        test("a\r\x0Bb",   vec!["a", "", "b"]);
        test("a\r\n\r\nb", vec!["a", "", "b"]);
    }
}
