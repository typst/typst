//! Tokenization.

use std::iter::Peekable;
use std::str::Chars;
use unicode_xid::UnicodeXID;

use crate::length::Length;
use super::span::{Pos, Span, Spanned};

use Token::*;
use TokenMode::*;
/// A minimal semantic entity of source code.
#[derive(Debug, Copy, Clone, PartialEq)]
pub enum Token<'s> {
    /// One or more whitespace characters. The contained `usize` denotes the
    /// number of newlines that were contained in the whitespace.
    Space(usize),

    /// A line comment with inner string contents `//<str>\n`.
    LineComment(&'s str),
    /// A block comment with inner string contents `/*<str>*/`. The comment
    /// can contain nested block comments.
    BlockComment(&'s str),

    /// A function invocation.
    Function {
        /// The header string:
        /// ```typst
        /// [header: args][body]
        ///  ^^^^^^^^^^^^
        /// ```
        header: &'s str,
        /// The spanned body string:
        /// ```typst
        /// [header][hello *world*]
        ///          ^^^^^^^^^^^^^
        /// ^-- The span is relative to right before this bracket
        /// ```
        body: Option<Spanned<&'s str>>,
        /// Whether the last closing bracket was present.
        /// - `[func]` or `[func][body]` => terminated
        /// - `[func` or `[func][body` => not terminated
        terminated: bool,
    },

    /// A left parenthesis in a function header: `(`.
    LeftParen,
    /// A right parenthesis in a function header: `)`.
    RightParen,
    /// A left brace in a function header: `{`.
    LeftBrace,
    /// A right brace in a function header: `}`.
    RightBrace,

    /// A colon in a function header: `:`.
    Colon,
    /// A comma in a function header: `,`.
    Comma,
    /// An equals sign in a function header: `=`.
    Equals,

    /// An identifier in a function header: `center`.
    Ident(&'s str),
    /// A quoted string in a function header: `"..."`.
    Str {
        /// The string inside the quotes.
        ///
        /// _Note_: If the string contains escape sequences these are not yet
        /// applied to be able to just store a string slice here instead of
        /// a String. The escaping is done later in the parser.
        string: &'s str,
        /// Whether the closing quote was present.
        terminated: bool,
    },
    /// A boolean in a function header: `true | false`.
    Bool(bool),
    /// A number in a function header: `3.14`.
    Number(f64),
    /// A length in a function header: `12pt`.
    Length(Length),
    /// A hex value in a function header: `#20d82a`.
    Hex(&'s str),
    /// A plus in a function header, signifying the addition of expressions.
    Plus,
    /// A hyphen in a function header, signifying the subtraction of
    /// expressions.
    Hyphen,
    /// A slash in a function header, signifying the division of expressions.
    Slash,

    /// A star. It can appear in a function header where it signifies the
    /// multiplication of expressions or the body where it modifies the styling.
    Star,
    /// An underscore in body-text.
    Underscore,

    /// A backslash followed by whitespace in text.
    Backslash,

    /// Raw text.
    Raw {
        /// The raw text (not yet unescaped as for strings).
        raw: &'s str,
        /// Whether the closing backtick was present.
        terminated: bool,
    },

    /// Any other consecutive string.
    Text(&'s str),

    /// Things that are not valid in the context they appeared in.
    Invalid(&'s str),
}

impl<'s> Token<'s> {
    /// The natural-language name for this token for use in error messages.
    pub fn name(self) -> &'static str {
        match self {
            Space(_) => "space",
            LineComment(_) => "line comment",
            BlockComment(_) => "block comment",
            Function { .. } => "function",
            LeftParen => "opening paren",
            RightParen => "closing paren",
            LeftBrace => "opening brace",
            RightBrace => "closing brace",
            Colon => "colon",
            Comma => "comma",
            Equals => "equals sign",
            Ident(_) => "identifier",
            Str { .. } => "string",
            Bool(_) => "bool",
            Number(_) => "number",
            Length(_) => "length",
            Hex(_) => "hex value",
            Plus => "plus",
            Hyphen => "minus",
            Slash => "slash",
            Star => "star",
            Underscore => "underscore",
            Backslash => "backslash",
            Raw { .. } => "raw text",
            Text(_) => "text",
            Invalid("]") => "closing bracket",
            Invalid("*/") => "end of block comment",
            Invalid(_) => "invalid token",
        }
    }
}

/// An iterator over the tokens of a string of source code.
#[derive(Debug)]
pub struct Tokens<'s> {
    src: &'s str,
    mode: TokenMode,
    iter: Peekable<Chars<'s>>,
    pos: Pos,
    index: usize,
}

/// Whether to tokenize in header mode which yields expression, comma and
/// similar tokens or in body mode which yields text and star, underscore,
/// backtick tokens.
#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
pub enum TokenMode {
    Header,
    Body,
}

impl<'s> Tokens<'s> {
    /// Create a new token iterator with the given mode.
    ///
    /// The first token's span starts an the given `offset` position instead of
    /// the zero position.
    pub fn new(src: &'s str, offset: Pos, mode: TokenMode) -> Self {
        Self {
            src,
            mode,
            iter: src.chars().peekable(),
            pos: offset,
            index: 0,
        }
    }

    /// The index in the string at which the last token ends and next token will
    /// start.
    pub fn index(&self) -> usize {
        self.index
    }

    /// The line-colunn position in the source at which the last token ends and
    /// next token will start.
    pub fn pos(&self) -> Pos {
        self.pos
    }
}

impl<'s> Iterator for Tokens<'s> {
    type Item = Spanned<Token<'s>>;

    /// Parse the next token in the source code.
    fn next(&mut self) -> Option<Self::Item> {
        let start = self.pos();
        let first = self.eat()?;

        let token = match first {
            // Comments.
            '/' if self.peek() == Some('/') => self.read_line_comment(),
            '/' if self.peek() == Some('*') => self.read_block_comment(),
            '*' if self.peek() == Some('/') => { self.eat(); Invalid("*/") }

            // Whitespace.
            c if c.is_whitespace() => self.read_whitespace(start),

            // Functions.
            '[' => self.read_function(start),
            ']' => Invalid("]"),

            // Syntactic elements in function headers.
            '(' if self.mode == Header => LeftParen,
            ')' if self.mode == Header => RightParen,
            '{' if self.mode == Header => LeftBrace,
            '}' if self.mode == Header => RightBrace,
            ':' if self.mode == Header => Colon,
            ',' if self.mode == Header => Comma,
            '=' if self.mode == Header => Equals,

            // Expression operators.
            '+' if self.mode == Header => Plus,
            '-' if self.mode == Header => Hyphen,
            '/' if self.mode == Header => Slash,

            // Star serves a double purpose as a style modifier
            // and a expression operator in the header.
            '*' => Star,

            // A hex expression.
            '#' if self.mode == Header => self.read_hex(),

            // String values.
            '"' if self.mode == Header => self.read_string(),

            // Style toggles.
            '_' if self.mode == Body => Underscore,
            '`' if self.mode == Body => self.read_raw(),

            // An escaped thing.
            '\\' if self.mode == Body => self.read_escaped(),

            // Expressions or just strings.
            c => {
                let body = self.mode == Body;

                let mut last_was_e = false;
                let text = self.read_string_until(|n| {
                    let val = match n {
                        c if c.is_whitespace() => true,
                        '['  | ']' | '/' | '*' => true,
                        '\\' | '_' | '`' if body => true,
                        ':'  | '=' | ',' | '"' |
                        '('  | ')' | '{' | '}' if !body => true,
                        '+'  | '-' if !body && !last_was_e => true,
                        _ => false,
                    };

                    last_was_e = n == 'e' || n == 'E';
                    val
                }, false, -(c.len_utf8() as isize), 0).0;

                if self.mode == Header {
                    self.read_expr(text)
                } else {
                    Text(text)
                }
            }
        };

        let end = self.pos();
        let span = Span { start, end };

        Some(Spanned { v: token, span })
    }
}

impl<'s> Tokens<'s> {
    fn read_line_comment(&mut self) -> Token<'s> {
        LineComment(self.read_string_until(is_newline_char, false, 1, 0).0)
    }

    fn read_block_comment(&mut self) -> Token<'s> {
        enum Last { Slash, Star, Other }

        self.eat();

        let mut depth = 0;
        let mut last = Last::Other;

        // Find the first `*/` that does not correspond to a nested `/*`.
        // Remove the last two bytes to obtain the raw inner text without `*/`.
        BlockComment(self.read_string_until(|n| {
            match n {
                '/' => match last {
                    Last::Star if depth == 0 => return true,
                    Last::Star => depth -= 1,
                    _ => last = Last::Slash
                }
                '*' => match last {
                    Last::Slash => depth += 1,
                    _ => last = Last::Star,
                }
                _ => last = Last::Other,
            }

            false
        }, true, 0, -2).0)
    }

    fn read_whitespace(&mut self, start: Pos) -> Token<'s> {
        self.read_string_until(|n| !n.is_whitespace(), false, 0, 0);
        let end = self.pos();

        Space(end.line - start.line)
    }

    fn read_function(&mut self, start: Pos) -> Token<'s> {
        let (header, terminated) = self.read_function_part(Header);
        self.eat();

        if self.peek() != Some('[') {
            return Function { header, body: None, terminated };
        }

        self.eat();

        let body_start = self.pos() - start;
        let (body, terminated) = self.read_function_part(Body);
        let body_end = self.pos() - start;
        let span = Span::new(body_start, body_end);

        self.eat();

        Function { header, body: Some(Spanned { v: body, span }), terminated }
    }

    fn read_function_part(&mut self, mode: TokenMode) -> (&'s str, bool) {
        let start = self.index();
        let mut terminated = false;

        while let Some(n) = self.peek() {
            if n == ']' {
                terminated = true;
                break;
            }

            self.eat();
            match n {
                '[' => { self.read_function(Pos::ZERO); }
                '/' if self.peek() == Some('/') => { self.read_line_comment(); }
                '/' if self.peek() == Some('*') => { self.read_block_comment(); }
                '"' if mode == Header => { self.read_string(); }
                '`' if mode == Body => { self.read_raw(); }
                '\\' => { self.eat(); }
                _ => {}
            }
        }

        let end = self.index();
        (&self.src[start..end], terminated)
    }

    fn read_string(&mut self) -> Token<'s> {
        let (string, terminated) = self.read_until_unescaped('"');
        Str { string, terminated }
    }

    fn read_raw(&mut self) -> Token<'s> {
        let (raw, terminated) = self.read_until_unescaped('`');
        Raw { raw, terminated }
    }

    fn read_until_unescaped(&mut self, c: char) -> (&'s str, bool) {
        let mut escaped = false;
        self.read_string_until(|n| {
            match n {
                n if n == c && !escaped => return true,
                '\\' => escaped = !escaped,
                _ => escaped = false,
            }

            false
        }, true, 0, -1)
    }

    fn read_escaped(&mut self) -> Token<'s> {
        fn is_escapable(c: char) -> bool {
            match c {
                '[' | ']' | '\\' | '/' | '*' | '_' | '`' | '"' => true,
                _ => false,
            }
        }

        match self.peek() {
            Some(c) if is_escapable(c) => {
                let index = self.index();
                self.eat();
                Text(&self.src[index..index + c.len_utf8()])
            }
            Some(c) if c.is_whitespace() => Backslash,
            Some(_) => Text("\\"),
            None => Backslash,
        }
    }

    fn read_hex(&mut self) -> Token<'s> {
        // This will parse more than the permissable 0-9, a-f, A-F character
        // ranges to provide nicer error messages later.
        Hex(self.read_string_until(
            |n| !n.is_ascii_alphanumeric(),
            false, 0, 0
        ).0)
    }

    fn read_expr(&mut self, text: &'s str) -> Token<'s> {
        if let Ok(b) = text.parse::<bool>() {
            Bool(b)
        } else if let Ok(num) = text.parse::<f64>() {
            Number(num)
        } else if let Some(num) = parse_percentage(text) {
            Number(num / 100.0)
        } else if let Ok(length) = text.parse::<Length>() {
            Length(length)
        } else if is_identifier(text) {
            Ident(text)
        } else {
            Invalid(text)
        }
    }

    /// Will read the input stream until `f` evaluates to `true`. When
    /// `eat_match` is true, the token for which `f` was true is consumed.
    /// Returns the string from the index where this was called offset by
    /// `offset_start` to the end offset by `offset_end`. The end is before or
    /// after the match depending on `eat_match`.
    fn read_string_until(
        &mut self,
        mut f: impl FnMut(char) -> bool,
        eat_match: bool,
        offset_start: isize,
        offset_end: isize,
    ) -> (&'s str, bool) {
        let start = ((self.index() as isize) + offset_start) as usize;
        let mut matched = false;

        while let Some(c) = self.peek() {
            if f(c) {
                matched = true;
                if eat_match {
                    self.eat();
                }
                break;
            }

            self.eat();
        }

        let mut end = self.index();
        if matched {
            end = ((end as isize) + offset_end) as usize;
        }

        (&self.src[start..end], matched)
    }

    fn eat(&mut self) -> Option<char> {
        let c = self.iter.next()?;
        self.index += c.len_utf8();

        if is_newline_char(c) && !(c == '\r' && self.peek() == Some('\n')) {
            self.pos.line += 1;
            self.pos.column = 0;
        } else {
            self.pos.column += 1;
        }

        Some(c)
    }

    fn peek(&mut self) -> Option<char> {
        self.iter.peek().copied()
    }
}

fn parse_percentage(text: &str) -> Option<f64> {
    if text.ends_with('%') {
        text[..text.len() - 1].parse::<f64>().ok()
    } else {
        None
    }
}

/// Whether this character denotes a newline.
pub fn is_newline_char(character: char) -> bool {
    match character {
        // Line Feed, Vertical Tab, Form Feed, Carriage Return.
        '\x0A'..='\x0D' => true,
        // Next Line, Line Separator, Paragraph Separator.
        '\u{0085}' | '\u{2028}' | '\u{2029}' => true,
        _ => false,
    }
}

/// Whether this word is a valid identifier.
pub fn is_identifier(string: &str) -> bool {
    fn is_extra_allowed(c: char) -> bool {
        c == '.' || c == '-' || c == '_'
    }

    let mut chars = string.chars();
    match chars.next() {
        Some(c) if UnicodeXID::is_xid_start(c) || is_extra_allowed(c) => {}
        _ => return false,
    }

    while let Some(c) = chars.next() {
        match c {
            c if UnicodeXID::is_xid_continue(c) || is_extra_allowed(c) => {}
            _ => return false,
        }
    }

    true
}

#[cfg(test)]
#[allow(non_snake_case)]
mod tests {
    use crate::length::Length;
    use crate::syntax::tests::*;
    use super::*;
    use Token::{
        Space as S,
        LineComment as LC, BlockComment as BC,
        LeftParen as LP, RightParen as RP,
        LeftBrace as LB, RightBrace as RB,
        Ident as Id,
        Bool,
        Number as Num,
        Length as Len,
        Hex,
        Plus,
        Hyphen as Min,
        Slash,
        Star,
        Text as T,
    };

    fn Str(string: &str, terminated: bool) -> Token { Token::Str { string, terminated } }
    fn Raw(raw: &str, terminated: bool) -> Token { Token::Raw { raw, terminated } }

    macro_rules! F {
        ($h:expr, None, $t:expr) => {
            Token::Function { header: $h, body: None, terminated: $t }
        };
        ($h:expr, $b:expr, $t:expr) => {
            Token::Function {
                header: $h,
                body: Some(Into::<Spanned<&str>>::into($b)),
                terminated: $t,
            }
        };
    }

    macro_rules! t { ($($tts:tt)*) => {test!(@spans=false, $($tts)*)} }
    macro_rules! ts { ($($tts:tt)*) => {test!(@spans=true, $($tts)*)} }
    macro_rules! test {
        (@spans=$spans:expr, $mode:expr, $src:expr => $($token:expr),*) => {
            let exp = vec![$(Into::<Spanned<Token>>::into($token)),*];
            let found = Tokens::new($src, Pos::ZERO, $mode).collect::<Vec<_>>();
            check($src, exp, found, $spans);
        }
    }

    #[test]
    fn tokenize_whitespace() {
        t!(Body, ""             => );
        t!(Body, " "            => S(0));
        t!(Body, "    "         => S(0));
        t!(Body, "\t"           => S(0));
        t!(Body, "  \t"         => S(0));
        t!(Body, "\n"           => S(1));
        t!(Body, "\n "          => S(1));
        t!(Body, "  \n"         => S(1));
        t!(Body, "  \n   "      => S(1));
        t!(Body, "\r\n"         => S(1));
        t!(Body, "  \n\t \n  "  => S(2));
        t!(Body, "\n\r"         => S(2));
        t!(Body, " \r\r\n \x0D" => S(3));
    }

    #[test]
    fn tokenize_comments() {
        t!(Body, "a // bc\n "        => T("a"), S(0), LC(" bc"),  S(1));
        t!(Body, "a //a//b\n "       => T("a"), S(0), LC("a//b"), S(1));
        t!(Body, "a //a//b\r\n"      => T("a"), S(0), LC("a//b"), S(1));
        t!(Body, "a //a//b\n\nhello" => T("a"), S(0), LC("a//b"), S(2), T("hello"));
        t!(Body, "/**/"              => BC(""));
        t!(Body, "_/*_/*a*/*/"       => Underscore, BC("_/*a*/"));
        t!(Body, "/*/*/"             => BC("/*/"));
        t!(Body, "abc*/"             => T("abc"), Invalid("*/"));
        t!(Body, "/***/"             => BC("*"));
        t!(Body, "/**\\****/*/*/"    => BC("*\\***"), Invalid("*/"), Invalid("*/"));
        t!(Body, "/*abc"             => BC("abc"));
    }

    #[test]
    fn tokenize_body_only_tokens() {
        t!(Body, "_*"            => Underscore, Star);
        t!(Body, "***"           => Star, Star, Star);
        t!(Body, "[func]*bold*"  => F!("func", None, true), Star, T("bold"), Star);
        t!(Body, "hi_you_ there" => T("hi"), Underscore, T("you"), Underscore, S(0), T("there"));
        t!(Body, "`raw`"         => Raw("raw", true));
        t!(Body, "`[func]`"      => Raw("[func]", true));
        t!(Body, "`]"            => Raw("]", false));
        t!(Body, "`\\``"         => Raw("\\`", true));
        t!(Body, "\\ "           => Backslash, S(0));
        t!(Header, "_`"          => Invalid("_`"));
    }

    #[test]
    fn tokenize_header_only_tokens() {
        t!(Body, "a: b"                => T("a:"), S(0), T("b"));
        t!(Body, "c=d, "               => T("c=d,"), S(0));
        t!(Header, "(){}:=,"           => LP, RP, LB, RB, Colon, Equals, Comma);
        t!(Header, "a:b"               => Id("a"), Colon, Id("b"));
        t!(Header, "#6ae6dd"           => Hex("6ae6dd"));
        t!(Header, "#8A083c"           => Hex("8A083c"));
        t!(Header, "a: true, x=1"      => Id("a"), Colon, S(0), Bool(true), Comma, S(0),
                                          Id("x"), Equals, Num(1.0));
        t!(Header, "=3.14"             => Equals, Num(3.14));
        t!(Header, "12.3e5"            => Num(12.3e5));
        t!(Header, "120%"              => Num(1.2));
        t!(Header, "12e4%"             => Num(1200.0));
        t!(Header, "__main__"          => Id("__main__"));
        t!(Header, ".func.box"         => Id(".func.box"));
        t!(Header, "arg, _b, _1"       => Id("arg"), Comma, S(0), Id("_b"), Comma, S(0), Id("_1"));
        t!(Header, "12_pt, 12pt"       => Invalid("12_pt"), Comma, S(0), Len(Length::pt(12.0)));
        t!(Header, "1e5in"             => Len(Length::inches(100000.0)));
        t!(Header, "2.3cm"             => Len(Length::cm(2.3)));
        t!(Header, "12e-3in"           => Len(Length::inches(12e-3)));
        t!(Header, "6.1cm + 4pt,a=1*2" => Len(Length::cm(6.1)), S(0), Plus, S(0), Len(Length::pt(4.0)),
                                          Comma, Id("a"), Equals, Num(1.0), Star, Num(2.0));
        t!(Header, "(5 - 1) / 2.1"     => LP, Num(5.0), S(0), Min, S(0), Num(1.0), RP,
                                          S(0), Slash, S(0), Num(2.1));
        t!(Header, "-1"                => Min, Num(1.0));
        t!(Header, "--1"               => Min, Min, Num(1.0));
        t!(Header, "- 1"               => Min, S(0), Num(1.0));
        t!(Header, "02.4mm"            => Len(Length::mm(2.4)));
        t!(Header, "2.4.cm"            => Invalid("2.4.cm"));
        t!(Header, "(1,2)"             => LP, Num(1.0), Comma, Num(2.0), RP);
        t!(Header, "{abc}"             => LB, Id("abc"), RB);
        t!(Header, "🌓, 🌍,"          => Invalid("🌓"), Comma, S(0), Invalid("🌍"), Comma);
    }

    #[test]
    fn tokenize_strings() {
        t!(Body, "a \"hi\" string"           => T("a"), S(0), T("\"hi\""), S(0), T("string"));
        t!(Header, "\"hello"                 => Str("hello", false));
        t!(Header, "\"hello world\""         => Str("hello world", true));
        t!(Header, "\"hello\nworld\""        => Str("hello\nworld", true));
        t!(Header, r#"1"hello\nworld"false"# => Num(1.0), Str("hello\\nworld", true), Bool(false));
        t!(Header, r#""a\"bc""#              => Str(r#"a\"bc"#, true));
        t!(Header, r#""a\\"bc""#             => Str(r#"a\\"#, true), Id("bc"), Str("", false));
        t!(Header, r#""a\tbc"#               => Str("a\\tbc", false));
        t!(Header, "\"🌎\""                  => Str("🌎", true));
    }

    #[test]
    fn tokenize_functions() {
        t!(Body, "a[f]"           => T("a"), F!("f", None, true));
        t!(Body, "[f]a"           => F!("f", None, true), T("a"));
        t!(Body, "\n\n[f][ ]"     => S(2), F!("f", " ", true));
        t!(Body, "abc [f][ ]a"    => T("abc"), S(0), F!("f", " ", true), T("a"));
        t!(Body, "[f: [=][*]]"    => F!("f: [=][*]", None, true));
        t!(Body, "[_][[,],],"     => F!("_", "[,],", true), T(","));
        t!(Body, "[=][=][=]"      => F!("=", "=", true), F!("=", None, true));
        t!(Body, "[=][[=][=][=]]" => F!("=", "[=][=][=]", true));
        t!(Header, "["            => F!("", None, false));
        t!(Header, "]"            => Invalid("]"));
    }

    #[test]
    fn tokenize_correct_end_of_function() {
        // End of function with strings and carets in headers
        t!(Body, r#"[f: "]"#      => F!(r#"f: "]"#, None, false));
        t!(Body, "[f: \"s\"]"     => F!("f: \"s\"", None, true));
        t!(Body, r#"[f: \"\"\"]"# => F!(r#"f: \"\"\""#, None, true));
        t!(Body, "[f: `]"         => F!("f: `", None, true));

        // End of function with strings and carets in bodies
        t!(Body, "[f][\"]"        => F!("f", s(0,4, 0,5, "\""), true));
        t!(Body, r#"[f][\"]"#     => F!("f", s(0,4, 0,6, r#"\""#), true));
        t!(Body, "[f][`]"         => F!("f", s(0,4, 0,6, "`]"), false));
        t!(Body, "[f][\\`]"       => F!("f", s(0,4, 0,6, "\\`"), true));
        t!(Body, "[f][`raw`]"     => F!("f", s(0,4, 0,9, "`raw`"), true));
        t!(Body, "[f][`raw]"      => F!("f", s(0,4, 0,9, "`raw]"), false));
        t!(Body, "[f][`raw]`]"    => F!("f", s(0,4, 0,10, "`raw]`"), true));
        t!(Body, "[f][`\\`]"      => F!("f", s(0,4, 0,8, "`\\`]"), false));
        t!(Body, "[f][`\\\\`]"    => F!("f", s(0,4, 0,8, "`\\\\`"), true));

        // End of function with comments
        t!(Body, "[f][/*]"        => F!("f", s(0,4, 0,7, "/*]"), false));
        t!(Body, "[f][/*`*/]"     => F!("f", s(0,4, 0,9, "/*`*/"), true));
        t!(Body, "[f: //]\n]"     => F!("f: //]\n", None, true));
        t!(Body, "[f: \"//]\n]"   => F!("f: \"//]\n]", None, false));

        // End of function with escaped brackets
        t!(Body, "[f][\\]]"       => F!("f", s(0,4, 0,6, "\\]"), true));
        t!(Body, "[f][\\[]"       => F!("f", s(0,4, 0,6, "\\["), true));
    }

    #[test]
    fn tokenize_escaped_symbols() {
        t!(Body, r"\\"   => T(r"\"));
        t!(Body, r"\["   => T("["));
        t!(Body, r"\]"   => T("]"));
        t!(Body, r"\*"   => T("*"));
        t!(Body, r"\_"   => T("_"));
        t!(Body, r"\`"   => T("`"));
        t!(Body, r"\/"   => T("/"));
        t!(Body, r#"\""# => T("\""));
    }

    #[test]
    fn tokenize_unescapable_symbols() {
        t!(Body, r"\a"     => T("\\"), T("a"));
        t!(Body, r"\:"     => T(r"\"), T(":"));
        t!(Body, r"\="     => T(r"\"), T("="));
        t!(Header, r"\\\\" => Invalid(r"\\\\"));
        t!(Header, r"\a"   => Invalid(r"\a"));
        t!(Header, r"\:"   => Invalid(r"\"), Colon);
        t!(Header, r"\="   => Invalid(r"\"), Equals);
        t!(Header, r"\,"   => Invalid(r"\"), Comma);
    }

    #[test]
    fn tokenize_with_spans() {
        ts!(Body, "hello"          => s(0,0, 0,5, T("hello")));
        ts!(Body, "ab\r\nc"        => s(0,0, 0,2, T("ab")), s(0,2, 1,0, S(1)), s(1,0, 1,1, T("c")));
        ts!(Body, "[x = \"(1)\"]*" => s(0,0, 0,11, F!("x = \"(1)\"", None, true)), s(0,11, 0,12, Star));
        ts!(Body, "// ab\r\n\nf"   => s(0,0, 0,5, LC(" ab")), s(0,5, 2,0, S(2)), s(2,0, 2,1, T("f")));
        ts!(Body, "/*b*/_"         => s(0,0, 0,5, BC("b")), s(0,5, 0,6, Underscore));
        ts!(Header, "a=10"         => s(0,0, 0,1, Id("a")), s(0,1, 0,2, Equals), s(0,2, 0,4, Num(10.0)));
    }
}
