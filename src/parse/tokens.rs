//! Tokenization.

use std::fmt::{self, Debug, Formatter};

use super::{is_newline, Scanner};
use crate::geom::Unit;
use crate::syntax::*;

use TokenMode::*;

/// An iterator over the tokens of a string of source code.
#[derive(Clone)]
pub struct Tokens<'s> {
    s: Scanner<'s>,
    mode: TokenMode,
}

/// Whether to tokenize in header mode which yields expression, comma and
/// similar tokens or in body mode which yields text and star, underscore,
/// backtick tokens.
#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum TokenMode {
    Header,
    Body,
}

impl<'s> Tokens<'s> {
    /// Create a new token iterator with the given mode.
    pub fn new(src: &'s str, mode: TokenMode) -> Self {
        Self { s: Scanner::new(src), mode }
    }

    /// Get the current token mode.
    pub fn mode(&self) -> TokenMode {
        self.mode
    }

    /// Change the token mode.
    pub fn set_mode(&mut self, mode: TokenMode) {
        self.mode = mode;
    }

    /// The position in the string at which the last token ends and next token
    /// will start.
    pub fn pos(&self) -> Pos {
        self.s.index().into()
    }

    /// The underlying scanner.
    pub fn scanner(&self) -> &Scanner<'s> {
        &self.s
    }
}

impl<'s> Iterator for Tokens<'s> {
    type Item = Token<'s>;

    /// Parse the next token in the source code.
    fn next(&mut self) -> Option<Self::Item> {
        let start = self.s.index();
        let c = self.s.eat()?;

        // This never loops. It just exists to allow breaking out of it.
        loop {
            // Common elements.
            return Some(match c {
                // Whitespace.
                c if c.is_whitespace() => self.whitespace(c, start),

                // Comments.
                '/' if self.s.eat_if('/') => self.line_comment(),
                '/' if self.s.eat_if('*') => self.block_comment(),
                '*' if self.s.eat_if('/') => Token::StarSlash,

                // Functions and blocks.
                '[' => Token::LeftBracket,
                ']' => Token::RightBracket,
                '{' => Token::LeftBrace,
                '}' => Token::RightBrace,

                _ => break,
            });
        }

        Some(match self.mode {
            Body => match c {
                // Markup.
                '*' => Token::Star,
                '_' => Token::Underscore,
                '~' => Token::Tilde,
                '#' => Token::Hashtag,
                '`' => self.raw(),

                // Escape sequences.
                '\\' => self.escaped(),

                // Plain text.
                _ => self.text(start),
            },

            Header => match c {
                // Syntactic elements in headers.
                '(' => Token::LeftParen,
                ')' => Token::RightParen,
                ':' => Token::Colon,
                ',' => Token::Comma,
                '|' => Token::Pipe,
                '+' => Token::Plus,
                '-' => Token::Hyphen,
                '*' => Token::Star,
                '/' => Token::Slash,

                // Expressions in headers.
                '#' => self.hex(),
                '"' => self.string(),

                // Expressions.
                c => self.expr(c, start),
            },
        })
    }
}

impl<'s> Tokens<'s> {
    fn whitespace(&mut self, first: char, start: usize) -> Token<'s> {
        // Fast path for just a single space
        if first == ' ' && !self.s.check(|c| c.is_whitespace()) {
            return Token::Space(0);
        }

        self.s.jump(start);

        // Count the number of newlines.
        let mut newlines = 0;
        while let Some(c) = self.s.eat_merging_crlf() {
            if !c.is_whitespace() {
                self.s.uneat();
                break;
            }

            if is_newline(c) {
                newlines += 1;
            }
        }

        Token::Space(newlines)
    }

    fn line_comment(&mut self) -> Token<'s> {
        Token::LineComment(self.s.eat_until(is_newline))
    }

    fn block_comment(&mut self) -> Token<'s> {
        let start = self.s.index();

        let mut state = '_';
        let mut depth = 1;

        // Find the first `*/` that does not correspond to a nested `/*`.
        while let Some(c) = self.s.eat() {
            state = match (state, c) {
                ('*', '/') => {
                    depth -= 1;
                    if depth == 0 {
                        break;
                    }
                    '_'
                }
                ('/', '*') => {
                    depth += 1;
                    '_'
                }
                _ => c,
            }
        }

        let terminated = depth == 0;
        let end = self.s.index() - if terminated { 2 } else { 0 };

        Token::BlockComment(self.s.get(start .. end))
    }

    fn raw(&mut self) -> Token<'s> {
        let mut backticks = 1;
        while self.s.eat_if('`') {
            backticks += 1;
        }

        let start = self.s.index();

        let mut found = 0;
        while found < backticks {
            match self.s.eat() {
                Some('`') => found += 1,
                Some(_) => found = 0,
                None => break,
            }
        }

        let terminated = found == backticks;
        let end = self.s.index() - if terminated { found } else { 0 };

        Token::Raw(TokenRaw {
            text: self.s.get(start .. end),
            backticks,
            terminated,
        })
    }

    fn escaped(&mut self) -> Token<'s> {
        if let Some(c) = self.s.peek() {
            match c {
                // Backslash and comments.
                '\\' | '/' |
                // Parenthesis.
                '[' | ']' | '{' | '}' |
                // Markup.
                '*' | '_' |  '~' | '#' | '`' => {
                    let start = self.s.index();
                    self.s.eat_assert(c);
                    Token::Text(&self.s.eaten_from(start))
                }
                'u' if self.s.peek_nth(1) == Some('{') => {
                    self.s.eat_assert('u');
                    self.s.eat_assert('{');
                    Token::UnicodeEscape(TokenUnicodeEscape {
                        // Allow more than `ascii_hexdigit` for better error recovery.
                        sequence: self.s.eat_while(|c| c.is_ascii_alphanumeric()),
                        terminated: self.s.eat_if('}'),
                    })
                }
                c if c.is_whitespace() => Token::Backslash,
                _ => Token::Text("\\"),
            }
        } else {
            Token::Backslash
        }
    }

    fn text(&mut self, start: usize) -> Token<'s> {
        while let Some(c) = self.s.eat() {
            if match c {
                // Whitespace.
                c if c.is_whitespace() => true,
                // Comments.
                '/' if self.s.check(|c| c == '/' || c == '*') => true,
                // Parenthesis.
                '[' | ']' | '{' | '}' => true,
                // Markup.
                '*' | '_' | '#' | '~' | '`' => true,
                // Escaping.
                '\\' => true,
                _ => false,
            } {
                self.s.uneat();
                break;
            }
        }

        Token::Text(self.s.eaten_from(start))
    }

    fn hex(&mut self) -> Token<'s> {
        // Allow more than `ascii_hexdigit` for better error recovery.
        Token::Hex(self.s.eat_while(|c| c.is_ascii_alphanumeric()))
    }

    fn string(&mut self) -> Token<'s> {
        let mut escaped = false;
        Token::Str(TokenStr {
            string: self.s.eat_until(|c| {
                if c == '"' && !escaped {
                    true
                } else {
                    escaped = c == '\\' && !escaped;
                    false
                }
            }),
            terminated: self.s.eat_if('"'),
        })
    }

    fn expr(&mut self, first: char, start: usize) -> Token<'s> {
        if is_id_start(first) {
            self.ident(start)
        } else if first.is_ascii_digit()
            || (first == '.' && self.s.check(|c| c.is_ascii_digit()))
        {
            self.number(start)
        } else {
            Token::Invalid(self.s.eaten_from(start))
        }
    }

    fn ident(&mut self, start: usize) -> Token<'s> {
        self.s.eat_while(is_id_continue);
        let string = self.s.eaten_from(start);
        match string {
            "true" => Token::Bool(true),
            "false" => Token::Bool(false),
            _ => Token::Ident(string),
        }
    }

    fn number(&mut self, start: usize) -> Token<'s> {
        self.s.jump(start);

        // Read the integer part.
        self.s.eat_while(|c| c.is_ascii_digit());

        // Read the fractional part if present.
        if self.s.eat_if('.') {
            self.s.eat_while(|c| c.is_ascii_digit());
        }

        // Read the exponent.
        if self.s.eat_if('e') || self.s.eat_if('E') {
            let _ = self.s.eat_if('+') || self.s.eat_if('-');
            self.s.eat_while(|c| c.is_ascii_digit());
        }

        // Read the suffix.
        if !self.s.eat_if('%') {
            self.s.eat_while(|c| c.is_ascii_alphanumeric());
        }

        // Parse into one of the suitable types.
        let string = self.s.eaten_from(start);
        if let Some(percent) = parse_percent(string) {
            Token::Percent(percent)
        } else if let Some((val, unit)) = parse_length(string) {
            Token::Length(val, unit)
        } else if let Ok(int) = string.parse::<i64>() {
            Token::Int(int)
        } else if let Ok(float) = string.parse::<f64>() {
            Token::Float(float)
        } else {
            Token::Invalid(string)
        }
    }
}

impl Debug for Tokens<'_> {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        write!(f, "Tokens({}|{})", self.s.eaten(), self.s.rest())
    }
}

fn parse_percent(string: &str) -> Option<f64> {
    string.strip_suffix('%').and_then(|prefix| prefix.parse::<f64>().ok())
}

fn parse_length(string: &str) -> Option<(f64, Unit)> {
    let len = string.len();

    // We need at least some number and the unit.
    if len <= 2 {
        return None;
    }

    // We can view the string as bytes since a multibyte UTF-8 char cannot
    // have valid ASCII chars as subbytes.
    let split = len - 2;
    let bytes = string.as_bytes();
    let unit = match &bytes[split ..] {
        b"pt" => Unit::Pt,
        b"mm" => Unit::Mm,
        b"cm" => Unit::Cm,
        b"in" => Unit::In,
        _ => return None,
    };

    string[.. split].parse::<f64>().ok().map(|val| (val, unit))
}

#[cfg(test)]
#[allow(non_snake_case)]
mod tests {
    use super::*;
    use crate::parse::tests::check;

    use Token::{
        BlockComment as BC, Ident as Id, LeftBrace as LB, LeftBracket as L,
        LeftParen as LP, LineComment as LC, RightBrace as RB, RightBracket as R,
        RightParen as RP, Space as S, Text as T, *,
    };
    use Unit::*;

    fn Str(string: &str, terminated: bool) -> Token {
        Token::Str(TokenStr { string, terminated })
    }

    fn Raw(text: &str, backticks: usize, terminated: bool) -> Token {
        Token::Raw(TokenRaw { text, backticks, terminated })
    }

    fn UE(sequence: &str, terminated: bool) -> Token {
        Token::UnicodeEscape(TokenUnicodeEscape { sequence, terminated })
    }

    /// Building blocks for suffix testing.
    ///
    /// We extend each test case with a collection of different suffixes to make
    /// sure tokens end at the correct position. These suffixes are split into
    /// blocks, which can be disabled/enabled per test case. For example, when
    /// testing identifiers we disable letter suffixes because these would
    /// mingle with the identifiers.
    ///
    /// Suffix blocks:
    /// - ' ': spacing
    /// - 'a': letters
    /// - '1': numbers
    /// - '/': symbols
    const BLOCKS: &str = " a1/";

    /// Suffixes described by four-tuples of:
    ///
    /// - block the suffix is part of
    /// - mode in which the suffix is applicable
    /// - the suffix string
    /// - the resulting suffix token
    const SUFFIXES: &[(char, Option<TokenMode>, &str, Token)] = &[
        // Whitespace suffixes.
        (' ', None, " ", S(0)),
        (' ', None, "\n", S(1)),
        (' ', None, "\r", S(1)),
        (' ', None, "\r\n", S(1)),
        // Letter suffixes.
        ('a', Some(Body), "hello", T("hello")),
        ('a', Some(Body), "💚", T("💚")),
        ('a', Some(Header), "val", Id("val")),
        ('a', Some(Header), "α", Id("α")),
        ('a', Some(Header), "_", Id("_")),
        // Number suffixes.
        ('1', Some(Header), "2", Int(2)),
        ('1', Some(Header), ".2", Float(0.2)),
        // Symbol suffixes.
        ('/', None, "[", L),
        ('/', None, "//", LC("")),
        ('/', None, "/**/", BC("")),
        ('/', Some(Body), "*", Star),
        ('/', Some(Body), "_", Underscore),
        ('/', Some(Body), r"\\", T(r"\")),
        ('/', Some(Header), "(", LP),
        ('/', Some(Header), ":", Colon),
        ('/', Some(Header), "+", Plus),
        ('/', Some(Header), "#123", Hex("123")),
    ];

    macro_rules! t {
        (Both $($tts:tt)*) => {
            t!(Body $($tts)*);
            t!(Header $($tts)*);
        };
        ($mode:ident $([$blocks:literal])?: $src:expr => $($token:expr),*) => {{
            // Test without suffix.
            t!(@$mode: $src => $($token),*);

            // Test with each applicable suffix.
            for &(block, mode, suffix, token) in SUFFIXES {
                let src = $src;
                #[allow(unused)]
                let mut blocks = BLOCKS;
                $(blocks = $blocks;)?
                assert!(!blocks.contains(|c| !BLOCKS.contains(c)));
                if (mode.is_none() || mode == Some($mode)) && blocks.contains(block) {
                    t!(@$mode: format!("{}{}", src, suffix) => $($token,)* token);
                }
            }
        }};
        (@$mode:ident: $src:expr => $($token:expr),*) => {{
            let src = $src;
            let exp = vec![$($token),*];
            let found = Tokens::new(&src, $mode).collect::<Vec<_>>();
            check(&src, exp, found, false);
        }};
    }

    #[test]
    fn test_length_from_str_parses_correct_value_and_unit() {
        assert_eq!(parse_length("2.5cm"), Some((2.5, Cm)));
        assert_eq!(parse_length("1.e+2cm"), Some((100.0, Cm)));
    }

    #[test]
    fn test_length_from_str_works_with_non_ascii_chars() {
        assert_eq!(parse_length("123🚚"), None);
    }

    #[test]
    fn test_tokenize_whitespace() {
        // Test basic whitespace.
        t!(Both["a1/"]: ""         => );
        t!(Both["a1/"]: " "        => S(0));
        t!(Both["a1/"]: "    "     => S(0));
        t!(Both["a1/"]: "\t"       => S(0));
        t!(Both["a1/"]: "  \t"     => S(0));
        t!(Both["a1/"]: "\u{202F}" => S(0));

        // Test newline counting.
        t!(Both["a1/"]: "\n"           => S(1));
        t!(Both["a1/"]: "\n "          => S(1));
        t!(Both["a1/"]: "  \n"         => S(1));
        t!(Both["a1/"]: "  \n   "      => S(1));
        t!(Both["a1/"]: "\r\n"         => S(1));
        t!(Both["a1/"]: "  \n\t \n  "  => S(2));
        t!(Both["a1/"]: "\n\r"         => S(2));
        t!(Both["a1/"]: " \r\r\n \x0D" => S(3));
    }

    #[test]
    fn test_tokenize_line_comments() {
        // Test line comment with no trailing newline.
        t!(Both[""]: "//" => LC(""));

        // Test line comment ends at newline.
        t!(Both["a1/"]: "//bc\n"   => LC("bc"), S(1));
        t!(Both["a1/"]: "// bc \n" => LC(" bc "), S(1));
        t!(Both["a1/"]: "//bc\r\n" => LC("bc"), S(1));

        // Test nested line comments.
        t!(Both["a1/"]: "//a//b\n" => LC("a//b"), S(1));
    }

    #[test]
    fn test_tokenize_block_comments() {
        // Test basic block comments.
        t!(Both[""]: "/*" => BC(""));
        t!(Both: "/**/"   => BC(""));
        t!(Both: "/*🏞*/" => BC("🏞"));
        t!(Both: "/*\n*/" => BC("\n"));

        // Test depth 1 and 2 nested block comments.
        t!(Both: "/* /* */ */"  => BC(" /* */ "));
        t!(Both: "/*/*/**/*/*/" => BC("/*/**/*/"));

        // Test two nested, one unclosed block comments.
        t!(Both[""]: "/*/*/**/*/" => BC("/*/**/*/"));

        // Test all combinations of up to two following slashes and stars.
        t!(Both[""]: "/*"   => BC(""));
        t!(Both[""]: "/*/"  => BC("/"));
        t!(Both[""]: "/**"  => BC("*"));
        t!(Both[""]: "/*//" => BC("//"));
        t!(Both[""]: "/*/*" => BC("/*"));
        t!(Both[""]: "/**/" => BC(""));
        t!(Both[""]: "/***" => BC("**"));
    }

    #[test]
    fn test_tokenize_body_tokens() {
        // Test parentheses.
        t!(Body: "[" => L);
        t!(Body: "]" => R);
        t!(Body: "{" => LB);
        t!(Body: "}" => RB);

        // Test markup tokens.
        t!(Body[" a1"]: "*" => Star);
        t!(Body: "_"        => Underscore);
        t!(Body: "~"        => Tilde);
        t!(Body: "#"        => Hashtag);
        t!(Body[" "]: r"\"  => Backslash);

        // Test header symbols.
        t!(Body[" /"]: ":,=|/+-" => T(":,=|/+-"));
    }

    #[test]
    fn test_tokenize_raw_blocks() {
        // Test basic raw block.
        t!(Body: "`raw`"  => Raw("raw", 1, true));
        t!(Body[""]: "`]" => Raw("]", 1, false));

        // Test special symbols in raw block.
        t!(Body: "`[func]`"   => Raw("[func]", 1, true));
        t!(Body[""]: r"`\`` " => Raw(r"\", 1, true), Raw(" ", 1, false));

        // Test more backticks.
        t!(Body: "````🚀````"           => Raw("🚀", 4, true));
        t!(Body[""]: "````👩‍🚀``noend"    => Raw("👩‍🚀``noend", 4, false));
        t!(Body[""]: "````raw``````new" => Raw("raw", 4, true), Raw("new", 2, false));

        // Test separated closing backticks.
        t!(Body: "```not `y`e`t```" => Raw("not `y`e`t", 3, true));
    }

    #[test]
    fn test_tokenize_escape_sequences() {
        // Test escapable symbols.
        t!(Body: r"\\" => T(r"\"));
        t!(Body: r"\/" => T("/"));
        t!(Body: r"\[" => T("["));
        t!(Body: r"\]" => T("]"));
        t!(Body: r"\{" => T("{"));
        t!(Body: r"\}" => T("}"));
        t!(Body: r"\*" => T("*"));
        t!(Body: r"\_" => T("_"));
        t!(Body: r"\#" => T("#"));
        t!(Body: r"\~" => T("~"));
        t!(Body: r"\`" => T("`"));

        // Test unescapable symbols.
        t!(Body[" /"]: r"\a"   => T(r"\"), T("a"));
        t!(Body[" /"]: r"\u"   => T(r"\"), T("u"));
        t!(Body[" /"]: r"\1"   => T(r"\"), T("1"));
        t!(Body[" /"]: r"\:"   => T(r"\"), T(":"));
        t!(Body[" /"]: r"\="   => T(r"\"), T("="));
        t!(Body[" /"]: r#"\""# => T(r"\"), T("\""));

        // Test basic unicode escapes.
        t!(Body: r"\u{}"     => UE("", true));
        t!(Body: r"\u{2603}" => UE("2603", true));
        t!(Body: r"\u{P}"    => UE("P", true));

        // Test unclosed unicode escapes.
        t!(Body[" /"]: r"\u{"     => UE("", false));
        t!(Body[" /"]: r"\u{1"    => UE("1", false));
        t!(Body[" /"]: r"\u{26A4" => UE("26A4", false));
        t!(Body[" /"]: r"\u{1Q3P" => UE("1Q3P", false));
        t!(Body: r"\u{1🏕}"       => UE("1", false), T("🏕"), RB);
    }

    #[test]
    fn test_tokenize_text() {
        // Test basic text.
        t!(Body[" /"]: "hello"       => T("hello"));
        t!(Body[" /"]: "hello-world" => T("hello-world"));

        // Test header symbols in text.
        t!(Body[" /"]: "a():\"b" => T("a():\"b"));

        // Test text ends.
        t!(Body[""]: "hello " => T("hello"), S(0));
        t!(Body[""]: "hello~" => T("hello"), Tilde);
    }

    #[test]
    fn test_tokenize_header_tokens() {
        // Test parentheses.
        t!(Header: "[" => L);
        t!(Header: "]" => R);
        t!(Header: "{" => LB);
        t!(Header: "}" => RB);
        t!(Header: "(" => LP);
        t!(Header: ")" => RP);

        // Test structural tokens.
        t!(Header: ":"        => Colon);
        t!(Header: ","        => Comma);
        t!(Header: "|"        => Pipe);
        t!(Header: "+"        => Plus);
        t!(Header: "-"        => Hyphen);
        t!(Header[" a1"]: "*" => Star);
        t!(Header[" a1"]: "/" => Slash);

        // Test hyphen parsed as symbol.
        t!(Header[" /"]: "-1"   => Hyphen, Int(1));
        t!(Header[" /"]: "-a"   => Hyphen, Id("a"));
        t!(Header[" /"]: "--1"  => Hyphen, Hyphen, Int(1));
        t!(Header[" /"]: "--_a" => Hyphen, Hyphen, Id("_a"));
        t!(Header[" /"]: "a-b"  => Id("a-b"));

        // Test some operations.
        t!(Header[" /"]: "1+3" => Int(1), Plus, Int(3));
        t!(Header[" /"]: "1*3" => Int(1), Star, Int(3));
        t!(Header[" /"]: "1/3" => Int(1), Slash, Int(3));
    }

    #[test]
    fn test_tokenize_idents() {
        // Test valid identifiers.
        t!(Header[" /"]: "x"           => Id("x"));
        t!(Header[" /"]: "value"       => Id("value"));
        t!(Header[" /"]: "__main__"    => Id("__main__"));
        t!(Header[" /"]: "_snake_case" => Id("_snake_case"));

        // Test non-ascii.
        t!(Header[" /"]: "α"    => Id("α"));
        t!(Header[" /"]: "ម្តាយ" => Id("ម្តាយ"));

        // Test hyphen parsed as identifier.
        t!(Header[" /"]: "kebab-case" => Id("kebab-case"));
        t!(Header[" /"]: "one-10"     => Id("one-10"));
    }

    #[test]
    fn test_tokenize_bools() {
        // Test valid bools.
        t!(Header[" /"]: "false" => Bool(false));
        t!(Header[" /"]: "true"  => Bool(true));

        // Test invalid bools.
        t!(Header[" /"]: "True"   => Id("True"));
        t!(Header[" /"]: "falser" => Id("falser"));
    }

    #[test]
    fn test_tokenize_numeric_values() {
        let ints = [("7", 7), ("012", 12)];
        let floats = [
            (".3", 0.3),
            ("0.3", 0.3),
            ("3.", 3.0),
            ("3.0", 3.0),
            ("14.3", 14.3),
            ("10e2", 1000.0),
            ("10e+0", 10.0),
            ("10e+1", 100.0),
            ("10e-2", 0.1),
            ("10.e1", 100.0),
            ("10.e-1", 1.0),
            (".1e1", 1.0),
            ("10E2", 1000.0),
        ];

        // Test integers.
        for &(s, v) in &ints {
            t!(Header[" /"]: s => Int(v));
        }

        // Test floats.
        for &(s, v) in &floats {
            t!(Header[" /"]: s => Float(v));
        }

        // Test attached numbers.
        t!(Header[" /"]: "1.2.3"  => Float(1.2), Float(0.3));
        t!(Header[" /"]: "1e-2+3" => Float(0.01), Plus, Int(3));

        // Test float from too large integer.
        let large = i64::MAX as f64 + 1.0;
        t!(Header[" /"]: large.to_string() => Float(large));

        // Combined integers and floats.
        let nums = ints.iter().map(|&(k, v)| (k, v as f64)).chain(floats.iter().copied());

        // Test percentages.
        for (s, v) in nums.clone() {
            t!(Header[" /"]: format!("{}%", s) => Percent(v));
        }

        // Test lengths.
        for &unit in &[Unit::Mm, Unit::Pt, Unit::Cm, Unit::In] {
            for (s, v) in nums.clone() {
                t!(Header[" /"]: format!("{}{}", s, unit) => Length(v, unit));
            }
        }
    }

    #[test]
    fn test_tokenize_hex() {
        // Test basic hex expressions.
        t!(Header[" /"]: "#6ae6dd" => Hex("6ae6dd"));
        t!(Header[" /"]: "#8A083c" => Hex("8A083c"));

        // Test with non-hex letters.
        t!(Header[" /"]: "#PQ" => Hex("PQ"));
    }

    #[test]
    fn test_tokenize_strings() {
        // Test basic strings.
        t!(Header: "\"hi\""        => Str("hi", true));
        t!(Header: "\"hi\nthere\"" => Str("hi\nthere", true));
        t!(Header: "\"🌎\""        => Str("🌎", true));
        t!(Header[""]: "\"hi"      => Str("hi", false));

        // Test escaped quote.
        t!(Header: r#""a\"bc""# => Str(r#"a\"bc"#, true));
        t!(Header[""]: r#""\""# => Str(r#"\""#, false));
    }

    #[test]
    fn test_tokenize_invalid() {
        // Test invalidly closed block comments.
        t!(Both: "*/"     => StarSlash);
        t!(Both: "/**/*/" => BC(""), StarSlash);

        // Test invalid expressions.
        t!(Header: r"\"          => Invalid(r"\"));
        t!(Header: "🌓"          => Invalid("🌓"));
        t!(Header: r"\:"         => Invalid(r"\"), Colon);
        t!(Header: "meal⌚"      => Id("meal"), Invalid("⌚"));
        t!(Header[" /"]: r"\a"   => Invalid(r"\"), Id("a"));
        t!(Header[" /"]: ">main" => Invalid(">"), Id("main"));

        // Test invalid number suffixes.
        t!(Header[" /"]: "1foo" => Invalid("1foo"));
        t!(Header: "1p%"        => Invalid("1p"), Invalid("%"));
        t!(Header: "1%%"        => Percent(1.0), Invalid("%"));
    }
}
