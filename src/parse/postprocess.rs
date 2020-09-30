//! Post-processing of strings and raw blocks.

use super::is_newline_char;
use crate::syntax::{Ident, Raw};

/// Resolves all escape sequences in a string.
pub fn unescape_string(string: &str) -> String {
    let mut iter = string.chars().peekable();
    let mut out = String::with_capacity(string.len());

    while let Some(c) = iter.next() {
        if c != '\\' {
            out.push(c);
            continue;
        }

        match iter.next() {
            Some('\\') => out.push('\\'),
            Some('"') => out.push('"'),

            Some('n') => out.push('\n'),
            Some('t') => out.push('\t'),
            Some('u') if iter.peek() == Some(&'{') => {
                iter.next();

                // TODO: Feedback if closing brace is missing.
                let mut sequence = String::new();
                let terminated = loop {
                    match iter.peek() {
                        Some('}') => {
                            iter.next();
                            break true;
                        }
                        Some(&c) if c.is_ascii_hexdigit() => {
                            iter.next();
                            sequence.push(c);
                        }
                        _ => break false,
                    }
                };

                if let Some(c) = hex_to_char(&sequence) {
                    out.push(c);
                } else {
                    // TODO: Feedback that escape sequence is wrong.
                    out.push_str("\\u{");
                    out.push_str(&sequence);
                    if terminated {
                        out.push('}');
                    }
                }
            }

            other => {
                out.push('\\');
                out.extend(other);
            }
        }
    }

    out
}

/// Resolves the language tag and trims the raw text.
///
/// Returns:
/// - The language tag
/// - The raw lines
/// - Whether at least one newline was present in the untrimmed text.
pub fn process_raw(raw: &str) -> Raw {
    let (lang, inner) = split_after_lang_tag(raw);
    let (lines, had_newline) = trim_and_split_raw(inner);
    Raw { lang, lines, inline: !had_newline }
}

/// Parse the lang tag and return it alongside the remaining inner raw text.
fn split_after_lang_tag(raw: &str) -> (Option<Ident>, &str) {
    let mut lang = String::new();

    let mut inner = raw;
    let mut iter = raw.chars();

    while let Some(c) = iter.next() {
        if c == '`' || c.is_whitespace() || is_newline_char(c) {
            break;
        }

        inner = iter.as_str();
        lang.push(c);
    }

    (Ident::new(lang), inner)
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

/// Splits a string into a vector of lines (respecting Unicode & Windows line breaks).
pub fn split_lines(text: &str) -> Vec<String> {
    let mut iter = text.chars().peekable();
    let mut line = String::new();
    let mut lines = Vec::new();

    while let Some(c) = iter.next() {
        if is_newline_char(c) {
            if c == '\r' && iter.peek() == Some(&'\n') {
                iter.next();
            }

            lines.push(std::mem::take(&mut line));
        } else {
            line.push(c);
        }
    }

    lines.push(line);
    lines
}

/// Converts a hexademical sequence (without braces or "\u") into a character.
pub fn hex_to_char(sequence: &str) -> Option<char> {
    u32::from_str_radix(sequence, 16).ok().and_then(std::char::from_u32)
}

#[cfg(test)]
#[rustfmt::skip]
mod tests {
    use super::*;

    #[test]
    fn test_unescape_strings() {
        fn test(string: &str, expected: &str) {
            assert_eq!(unescape_string(string), expected.to_string());
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
    fn test_split_after_lang_tag() {
        fn test(raw: &str, lang: Option<&str>, inner: &str) {
            let (found_lang, found_inner) = split_after_lang_tag(raw);
            assert_eq!(found_lang.as_ref().map(|id| id.as_str()), lang);
            assert_eq!(found_inner, inner);
        }

        test("typst it!",   Some("typst"), " it!");
        test("typst\n it!", Some("typst"), "\n it!");
        test("typst\n it!", Some("typst"), "\n it!");
        test("abc`",        Some("abc"),   "`");
        test(" hi",         None,          " hi");
        test("`",           None,          "`");
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
