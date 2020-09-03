use crate::syntax::tokens::is_newline_char;

/// Resolves all escape sequences in a string.
pub fn unescape_string(string: &str) -> String {
    let mut iter = string.chars().peekable();
    let mut out = String::with_capacity(string.len());

    while let Some(c) = iter.next() {
        if c == '\\' {
            match iter.next() {
                Some('\\') => out.push('\\'),
                Some('"') => out.push('"'),
                Some('u') if iter.peek() == Some(&'{') => {
                    iter.next();

                    let mut sequence = String::new();
                    let terminated = loop {
                        match iter.peek() {
                            // TODO: Feedback that closing brace is missing.
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

                    // TODO: Feedback that escape sequence is wrong.
                    if let Some(c) = hex_to_char(&sequence) {
                        out.push(c);
                    } else {
                        out.push_str("\\u{");
                        out.push_str(&sequence);
                        if terminated {
                            out.push('}');
                        }
                    }
                }
                Some('n') => out.push('\n'),
                Some('t') => out.push('\t'),
                Some(c) => {
                    out.push('\\');
                    out.push(c);
                }
                None => out.push('\\'),
            }
        } else {
            out.push(c);
        }
    }

    out
}

/// Resolves all escape sequences in raw markup (between backticks) and splits it into
/// into lines.
pub fn unescape_raw(raw: &str) -> Vec<String> {
    let mut iter = raw.chars();
    let mut text = String::new();

    while let Some(c) = iter.next() {
        if c == '\\' {
            if let Some(c) = iter.next() {
                if c != '\\' && c != '`' {
                    text.push('\\');
                }

                text.push(c);
            } else {
                text.push('\\');
            }
        } else {
            text.push(c);
        }
    }

    split_lines(&text)
}

/// Resolves all escape sequences in code markup (between triple backticks) and splits it
/// into into lines.
pub fn unescape_code(raw: &str) -> Vec<String> {
    let mut iter = raw.chars().peekable();
    let mut text = String::new();
    let mut backticks = 0u32;
    let mut update_backtick_count;

    while let Some(c) = iter.next() {
        update_backtick_count = true;

        if c == '\\' && backticks > 0 {
            let mut tail = String::new();
            let mut escape_success = false;
            let mut backticks_after_slash = 0u32;

            while let Some(&s) = iter.peek() {
                match s {
                    '\\' => {
                        if backticks_after_slash == 0 {
                            tail.push('\\');
                        } else {
                            // Pattern like `\`\` should fail
                            // escape and just be printed verbantim.
                            break;
                        }
                    }
                    '`' => {
                        tail.push(s);
                        backticks_after_slash += 1;
                        if backticks_after_slash == 2 {
                            escape_success = true;
                            iter.next();
                            break;
                        }
                    }
                    _ => break,
                }

                iter.next();
            }

            if !escape_success {
                text.push(c);
                backticks = backticks_after_slash;
                update_backtick_count = false;
            } else {
                backticks = 0;
            }

            text.push_str(&tail);
        } else {
            text.push(c);
        }

        if update_backtick_count {
            if c == '`' {
                backticks += 1;
            } else {
                backticks = 0;
            }
        }
    }

    split_lines(&text)
}

/// Converts a hexademical sequence (without braces or "\u") into a character.
pub fn hex_to_char(sequence: &str) -> Option<char> {
    u32::from_str_radix(sequence, 16).ok().and_then(std::char::from_u32)
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[rustfmt::skip]
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
    #[rustfmt::skip]
    fn test_unescape_raws() {
        fn test(raw: &str, expected: Vec<&str>) {
            assert_eq!(unescape_raw(raw), expected);
        }

        test("raw\\`",     vec!["raw`"]);
        test("raw\\\\`",   vec!["raw\\`"]);
        test("raw\ntext",  vec!["raw", "text"]);
        test("a\r\nb",     vec!["a", "b"]);
        test("a\n\nb",     vec!["a", "", "b"]);
        test("a\r\x0Bb",   vec!["a", "", "b"]);
        test("a\r\n\r\nb", vec!["a", "", "b"]);
        test("raw\\a",     vec!["raw\\a"]);
        test("raw\\",      vec!["raw\\"]);
    }

    #[test]
    #[rustfmt::skip]
    fn test_unescape_code() {
        fn test(raw: &str, expected: Vec<&str>) {
            assert_eq!(unescape_code(raw), expected);
        }

        test("code\\`",       vec!["code\\`"]);
        test("code`\\``",     vec!["code```"]);
        test("code`\\`a",     vec!["code`\\`a"]);
        test("code``hi`\\``", vec!["code``hi```"]);
        test("code`\\\\``",   vec!["code`\\``"]);
        test("code`\\`\\`go", vec!["code`\\`\\`go"]);
        test("code`\\`\\``",  vec!["code`\\```"]);
        test("code\ntext",    vec!["code", "text"]);
        test("a\r\nb",        vec!["a", "b"]);
        test("a\n\nb",        vec!["a", "", "b"]);
        test("a\r\x0Bb",      vec!["a", "", "b"]);
        test("a\r\n\r\nb",    vec!["a", "", "b"]);
        test("code\\a",       vec!["code\\a"]);
        test("code\\",        vec!["code\\"]);
    }
}
