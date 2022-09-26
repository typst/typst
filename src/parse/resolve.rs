use unscanny::Scanner;

use super::{is_ident, is_newline};
use crate::syntax::ast::RawNode;
use crate::util::EcoString;

/// Resolve all escape sequences in a string.
pub fn resolve_string(string: &str) -> EcoString {
    let mut out = EcoString::with_capacity(string.len());
    let mut s = Scanner::new(string);

    while let Some(c) = s.eat() {
        if c != '\\' {
            out.push(c);
            continue;
        }

        let start = s.locate(-1);
        match s.eat() {
            Some('\\') => out.push('\\'),
            Some('"') => out.push('"'),
            Some('n') => out.push('\n'),
            Some('r') => out.push('\r'),
            Some('t') => out.push('\t'),
            Some('u') if s.eat_if('{') => {
                // TODO: Error if closing brace is missing.
                let sequence = s.eat_while(char::is_ascii_hexdigit);
                let _terminated = s.eat_if('}');
                match resolve_hex(sequence) {
                    Some(c) => out.push(c),
                    None => out.push_str(s.from(start)),
                }
            }

            _ => out.push_str(s.from(start)),
        }
    }

    out
}

/// Resolve a hexadecimal escape sequence into a character
/// (only the inner hex letters without braces or `\u`).
pub fn resolve_hex(sequence: &str) -> Option<char> {
    u32::from_str_radix(sequence, 16).ok().and_then(std::char::from_u32)
}

/// Resolve the language tag and trim the raw text.
pub fn resolve_raw(column: usize, backticks: usize, text: &str) -> RawNode {
    if backticks > 1 {
        let (tag, inner) = split_at_lang_tag(text);
        let (text, block) = trim_and_split_raw(column, inner);
        RawNode {
            lang: is_ident(tag).then(|| tag.into()),
            text: text.into(),
            block,
        }
    } else {
        RawNode {
            lang: None,
            text: split_lines(text).join("\n").into(),
            block: false,
        }
    }
}

/// Parse the lang tag and return it alongside the remaining inner raw text.
fn split_at_lang_tag(raw: &str) -> (&str, &str) {
    let mut s = Scanner::new(raw);
    (
        s.eat_until(|c: char| c == '`' || c.is_whitespace() || is_newline(c)),
        s.after(),
    )
}

/// Trim raw text and splits it into lines.
///
/// Also returns whether at least one newline was contained in `raw`.
fn trim_and_split_raw(column: usize, mut raw: &str) -> (String, bool) {
    // Trims one space at the start.
    raw = raw.strip_prefix(' ').unwrap_or(raw);

    // Trim one space at the end if the last non-whitespace char is a backtick.
    if raw.trim_end().ends_with('`') {
        raw = raw.strip_suffix(' ').unwrap_or(raw);
    }

    let mut lines = split_lines(raw);

    // Dedent based on column, but not for the first line.
    for line in lines.iter_mut().skip(1) {
        let offset = line
            .chars()
            .take(column)
            .take_while(|c| c.is_whitespace())
            .map(char::len_utf8)
            .sum();
        *line = &line[offset ..];
    }

    let had_newline = lines.len() > 1;
    let is_whitespace = |line: &&str| line.chars().all(char::is_whitespace);

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
fn split_lines(text: &str) -> Vec<&str> {
    let mut s = Scanner::new(text);
    let mut lines = Vec::new();
    let mut start = 0;
    let mut end = 0;

    while let Some(c) = s.eat() {
        if is_newline(c) {
            if c == '\r' {
                s.eat_if('\n');
            }

            lines.push(&text[start .. end]);
            start = s.cursor();
        }
        end = s.cursor();
    }

    lines.push(&text[start ..]);
    lines
}

#[cfg(test)]
#[rustfmt::skip]
mod tests {
    use super::*;

    #[test]
    fn test_resolve_strings() {
        #[track_caller]
        fn test(string: &str, expected: &str) {
            assert_eq!(resolve_string(string), expected);
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
            column: usize,
            backticks: usize,
            raw: &str,
            lang: Option<&str>,
            text: &str,
            block: bool,
        ) {
            let node = resolve_raw(column, backticks, raw);
            assert_eq!(node.lang.as_deref(), lang);
            assert_eq!(node.text, text);
            assert_eq!(node.block, block);
        }

        // Just one backtick.
        test(0, 1, "py",     None, "py",   false);
        test(0, 1, "1\n2",   None, "1\n2", false);
        test(0, 1, "1\r\n2", None, "1\n2", false);

        // More than one backtick with lang tag.
        test(0, 2, "js alert()",    Some("js"), "alert()",    false);
        test(0, 3, "py quit(\n\n)", Some("py"), "quit(\n\n)", true);
        test(0, 2, "♥",             None,       "",           false);

        // Trimming of whitespace (tested more thoroughly in separate test).
        test(0, 2, " a",   None, "a",  false);
        test(0, 2, "  a",  None, " a", false);
        test(0, 2, " \na", None, "a",  true);

        // Dedenting
        test(2, 3, " def foo():\n    bar()", None, "def foo():\n  bar()", true);
    }

    #[test]
    fn test_trim_raw() {
        #[track_caller]
        fn test(text: &str, expected: &str) {
            assert_eq!(trim_and_split_raw(0, text).0, expected);
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
