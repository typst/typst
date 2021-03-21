use super::{is_newline, Scanner};
use crate::syntax::{Ident, RawNode, Span};

/// Resolve all escape sequences in a string.
pub fn resolve_string(string: &str) -> String {
    let mut out = String::with_capacity(string.len());
    let mut s = Scanner::new(string);

    while let Some(c) = s.eat() {
        if c != '\\' {
            out.push(c);
            continue;
        }

        let start = s.last_index();
        match s.eat() {
            Some('\\') => out.push('\\'),
            Some('"') => out.push('"'),
            Some('n') => out.push('\n'),
            Some('r') => out.push('\r'),
            Some('t') => out.push('\t'),
            Some('u') if s.eat_if('{') => {
                // TODO: Feedback if closing brace is missing.
                let sequence = s.eat_while(|c| c.is_ascii_hexdigit());
                let _terminated = s.eat_if('}');

                if let Some(c) = resolve_hex(sequence) {
                    out.push(c);
                } else {
                    // TODO: Feedback that unicode escape sequence is wrong.
                    out += s.eaten_from(start);
                }
            }

            // TODO: Feedback about invalid escape sequence.
            _ => out += s.eaten_from(start),
        }
    }

    out
}

/// Resolve a hexadecimal escape sequence into a character
/// (only the inner hex letters without braces or `\u`).
pub fn resolve_hex(sequence: &str) -> Option<char> {
    u32::from_str_radix(sequence, 16).ok().and_then(std::char::from_u32)
}

/// Resolve the language tag and trims the raw text.
pub fn resolve_raw(span: Span, text: &str, backticks: usize) -> RawNode {
    if backticks > 1 {
        let (tag, inner) = split_at_lang_tag(text);
        let (text, block) = trim_and_split_raw(inner);
        let lang = Ident::new(tag, span.start .. span.start + tag.len());
        RawNode { span, lang, text, block }
    } else {
        RawNode {
            span,
            lang: None,
            text: split_lines(text).join("\n"),
            block: false,
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

/// Trim raw text and splits it into lines.
///
/// Returns whether at least one newline was contained in `raw`.
fn trim_and_split_raw(mut raw: &str) -> (String, bool) {
    // Trims one space at the start.
    raw = raw.strip_prefix(' ').unwrap_or(raw);

    // Trim one space at the end if the last non-whitespace char is a backtick.
    if raw.trim_end().ends_with('`') {
        raw = raw.strip_suffix(' ').unwrap_or(raw);
    }

    let mut lines = split_lines(raw);
    let is_whitespace = |line: &String| line.chars().all(char::is_whitespace);
    let had_newline = lines.len() > 1;

    // Trims a sequence of whitespace followed by a newline at the start.
    if lines.first().map_or(false, is_whitespace) {
        lines.remove(0);
    }

    // Trims a newline followed by a sequence of whitespace at the end.
    if lines.last().map_or(false, is_whitespace) {
        lines.pop();
    }

    (lines.join("\n"), had_newline)
}

/// Split a string into a vector of lines
/// (respecting Unicode, Unix, Mac and Windows line breaks).
fn split_lines(text: &str) -> Vec<String> {
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
    use crate::syntax::Span;
    use super::*;

    #[test]
    fn test_resolve_strings() {
        #[track_caller]
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
        test(r#"a\t\r\nbc"#,    "a\t\r\nbc");
        test(r"🌎",             "🌎");
        test(r"🌎\",            r"🌎\");
        test(r"\🌎",            r"\🌎");
    }

    #[test]
    fn test_split_at_lang_tag() {
        #[track_caller]
        fn test(text: &str, lang: &str, inner: &str) {
            assert_eq!(split_at_lang_tag(text), (lang, inner));
        }

        test("typst it!",   "typst", " it!");
        test("typst\n it!", "typst", "\n it!");
        test("typst\n it!", "typst", "\n it!");
        test("abc`",        "abc",   "`");
        test(" hi",         "",      " hi");
        test("`",           "",      "`");
    }

    #[test]
    fn test_resolve_raw() {
        #[track_caller]
        fn test(
            raw: &str,
            backticks: usize,
            lang: Option<&str>,
            text: &str,
            block: bool,
        ) {
            Span::without_cmp(|| {
                assert_eq!(resolve_raw(Span::ZERO, raw, backticks), RawNode {
                    span: Span::ZERO,
                    lang: lang.and_then(|id| Ident::new(id, 0)),
                    text: text.into(),
                    block,
                });
            });
        }

        // Just one backtick.
        test("py",     1, None, "py",     false);
        test("1\n2",   1, None, "1\n2", false);
        test("1\r\n2", 1, None, "1\n2", false);

        // More than one backtick with lang tag.
        test("js alert()",     2, Some("js"), "alert()",        false);
        test("py quit(\n\n)",  3, Some("py"), "quit(\n\n)", true);
        test("♥",              2, None,       "",               false);

        // Trimming of whitespace (tested more thoroughly in separate test).
        test(" a",   2, None, "a",  false);
        test("  a",  2, None, " a", false);
        test(" \na", 2, None, "a",  true);
    }

    #[test]
    fn test_trim_raw() {
        #[track_caller]
        fn test(text: &str, expected: &str) {
            assert_eq!(trim_and_split_raw(text).0, expected);
        }

        test(" hi",          "hi");
        test("  hi",         " hi");
        test("\nhi",         "hi");
        test("    \n hi",    " hi");
        test("hi` ",         "hi`");
        test("hi`  ",        "hi` ");
        test("hi`   ",       "hi`  ");
        test("hi ",          "hi ");
        test("hi  ",         "hi  ");
        test("hi\n",         "hi");
        test("hi \n   ",     "hi ");
        test("  \n hi \n  ", " hi ");
    }
}
