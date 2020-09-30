//! Tokenization.

use super::{is_newline_char, CharParser};
use crate::length::Length;
use crate::syntax::{Ident, Pos, Span, SpanWith, Spanned, Token};

use TokenMode::*;

/// An iterator over the tokens of a string of source code.
#[derive(Debug)]
pub struct Tokens<'s> {
    p: CharParser<'s>,
    mode: TokenMode,
    stack: Vec<TokenMode>,
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
    pub fn new(src: &'s str, mode: TokenMode) -> Self {
        Self {
            p: CharParser::new(src),
            mode,
            stack: vec![],
        }
    }

    /// Change the token mode and push the old one on a stack.
    pub fn push_mode(&mut self, mode: TokenMode) {
        self.stack.push(self.mode);
        self.mode = mode;
    }

    /// Pop the old token mode from the stack. This panics if there is no mode
    /// on the stack.
    pub fn pop_mode(&mut self) {
        self.mode = self.stack.pop().expect("no pushed mode");
    }

    /// The position in the string at which the last token ends and next token
    /// will start.
    pub fn pos(&self) -> Pos {
        self.p.index().into()
    }
}

impl<'s> Iterator for Tokens<'s> {
    type Item = Spanned<Token<'s>>;

    /// Parse the next token in the source code.
    fn next(&mut self) -> Option<Self::Item> {
        let start = self.p.index();
        let token = match self.p.eat()? {
            // Whitespace.
            c if c.is_whitespace() => self.read_whitespace(c),

            // Comments.
            '/' if self.p.eat_if('/') => self.read_line_comment(),
            '/' if self.p.eat_if('*') => self.read_block_comment(),
            '*' if self.p.eat_if('/') => Token::Invalid("*/"),

            // Functions.
            '[' => Token::LeftBracket,
            ']' => Token::RightBracket,
            '{' => Token::LeftBrace,
            '}' => Token::RightBrace,

            // Syntactic elements in body text.
            '_' if self.mode == Body => Token::Underscore,
            '`' if self.mode == Body => self.read_raw(),
            '#' if self.mode == Body => Token::Hashtag,
            '~' if self.mode == Body => Token::Text("\u{00A0}"),
            '\\' if self.mode == Body => self.read_escaped(),

            // Syntactic elements in headers.
            '(' if self.mode == Header => Token::LeftParen,
            ')' if self.mode == Header => Token::RightParen,
            ':' if self.mode == Header => Token::Colon,
            ',' if self.mode == Header => Token::Comma,
            '=' if self.mode == Header => Token::Equals,
            '>' if self.mode == Header && self.p.eat_if('>') => Token::Chain,

            // Expressions.
            '+' if self.mode == Header => Token::Plus,
            '-' if self.mode == Header => Token::Hyphen,
            '/' if self.mode == Header => Token::Slash,
            '#' if self.mode == Header => self.read_hex(),
            '"' if self.mode == Header => self.read_string(),

            // Star serves a double purpose as a style modifier
            // and a expression operator in the header.
            '*' => Token::Star,

            // Expressions or just plain text.
            _ => self.read_text_or_expr(start),
        };

        let end = self.p.index();
        Some(token.span_with(Span::new(start, end)))
    }
}

impl<'s> Tokens<'s> {
    fn read_whitespace(&mut self, first: char) -> Token<'s> {
        // Shortcut for common case of exactly one space.
        if first == ' ' && !self.p.check(|c| c.is_whitespace()) {
            return Token::Space(0);
        }

        // Uneat the first char if it's a newline, so it's counted in the loop.
        if is_newline_char(first) {
            self.p.uneat();
        }

        // Count the number of newlines.
        let mut newlines = 0;
        while let Some(c) = self.p.eat_merging_crlf() {
            if !c.is_whitespace() {
                self.p.uneat();
                break;
            }

            if is_newline_char(c) {
                newlines += 1;
            }
        }

        Token::Space(newlines)
    }

    fn read_line_comment(&mut self) -> Token<'s> {
        Token::LineComment(self.p.eat_until(is_newline_char))
    }

    fn read_block_comment(&mut self) -> Token<'s> {
        let start = self.p.index();

        let mut depth = 1;
        let mut state = ' ';

        // Find the first `*/` that does not correspond to a nested `/*`.
        while let Some(c) = self.p.eat() {
            state = match (state, c) {
                ('*', '/') if depth == 1 => {
                    depth = 0;
                    break;
                }
                ('*', '/') => {
                    depth -= 1;
                    ' '
                }
                ('/', '*') => {
                    depth += 1;
                    ' '
                }
                _ => c,
            }
        }

        let mut read = self.p.eaten_from(start);
        if depth == 0 {
            read = read.strip_suffix("*/").unwrap_or(read);
        }

        Token::BlockComment(read)
    }

    fn read_hex(&mut self) -> Token<'s> {
        // This parses more than the permissable 0-9, a-f, A-F character ranges
        // to provide nicer error messages later.
        Token::Hex(self.p.eat_while(|c| c.is_ascii_alphanumeric()))
    }

    fn read_string(&mut self) -> Token<'s> {
        let mut escaped = false;
        Token::Str {
            string: self.p.eat_until(|c| {
                if c == '"' && !escaped {
                    true
                } else {
                    escaped = c == '\\' && !escaped;
                    false
                }
            }),
            terminated: self.p.eat_if('"'),
        }
    }

    fn read_raw(&mut self) -> Token<'s> {
        let mut backticks = 1;
        while self.p.eat_if('`') {
            backticks += 1;
        }

        let start = self.p.index();
        let mut found = 0;
        while found < backticks {
            match self.p.eat() {
                Some('`') => found += 1,
                Some(_) => found = 0,
                None => break,
            }
        }

        let terminated = found == backticks;
        let end = self.p.index() - if terminated { found } else { 0 };

        Token::Raw {
            raw: self.p.get(start .. end),
            backticks,
            terminated,
        }
    }

    fn read_escaped(&mut self) -> Token<'s> {
        if let Some(c) = self.p.peek() {
            match c {
                '[' | ']' | '\\' | '/' | '*' | '_' | '`' | '"' | '#' | '~' => {
                    let start = self.p.index();
                    self.p.eat_assert(c);
                    Token::Text(&self.p.eaten_from(start))
                }
                'u' if self.p.peek_nth(1) == Some('{') => {
                    self.p.eat_assert('u');
                    self.p.eat_assert('{');
                    Token::UnicodeEscape {
                        sequence: self.p.eat_while(|c| c.is_ascii_hexdigit()),
                        terminated: self.p.eat_if('}'),
                    }
                }
                c if c.is_whitespace() => Token::Backslash,
                _ => Token::Text("\\"),
            }
        } else {
            Token::Backslash
        }
    }

    fn read_text_or_expr(&mut self, start: usize) -> Token<'s> {
        let body = self.mode == Body;
        let header = self.mode == Header;

        let mut last_was_e = false;
        self.p.eat_until(|c| {
            let end = match c {
                c if c.is_whitespace() => true,
                '[' | ']' | '*' | '/' => true,
                '_' | '`' | '~' | '\\' if body => true,
                '(' | ')' | '{' | '}' | ':' | ',' | '=' | '"' | '#' if header => true,
                '+' | '-' if header && !last_was_e => true,
                _ => false,
            };
            last_was_e = c == 'e' || c == 'E';
            end
        });

        let read = self.p.eaten_from(start);
        if self.mode == Header {
            parse_expr(read)
        } else {
            Token::Text(read)
        }
    }
}

fn parse_expr(text: &str) -> Token<'_> {
    if let Ok(b) = text.parse::<bool>() {
        Token::Bool(b)
    } else if let Ok(num) = text.parse::<f64>() {
        Token::Number(num)
    } else if let Some(num) = parse_percent(text) {
        Token::Number(num / 100.0)
    } else if let Ok(length) = text.parse::<Length>() {
        Token::Length(length)
    } else if Ident::is_ident(text) {
        Token::Ident(text)
    } else {
        Token::Invalid(text)
    }
}

fn parse_percent(text: &str) -> Option<f64> {
    if text.ends_with('%') {
        text[.. text.len() - 1].parse::<f64>().ok()
    } else {
        None
    }
}

#[cfg(test)]
#[allow(non_snake_case)]
mod tests {
    use super::*;
    use crate::length::Length;
    use crate::parse::tests::{check, s};

    use Token::{
        BlockComment as BC, Bool, Chain, Hex, Hyphen as Min, Ident as Id,
        LeftBrace as LB, LeftBracket as L, LeftParen as LP, Length as Len,
        LineComment as LC, Number as Num, Plus, RightBrace as RB, RightBracket as R,
        RightParen as RP, Slash, Space as S, Star, Text as T, *,
    };

    fn Str(string: &str, terminated: bool) -> Token {
        Token::Str { string, terminated }
    }
    fn Raw(raw: &str, backticks: usize, terminated: bool) -> Token {
        Token::Raw { raw, backticks, terminated }
    }
    fn UE(sequence: &str, terminated: bool) -> Token {
        Token::UnicodeEscape { sequence, terminated }
    }

    macro_rules! t { ($($tts:tt)*) => {test!(@spans=false, $($tts)*)} }
    macro_rules! ts { ($($tts:tt)*) => {test!(@spans=true, $($tts)*)} }
    macro_rules! test {
        (@spans=$spans:expr, $mode:expr, $src:expr => $($token:expr),*) => {
            let exp = vec![$(Into::<Spanned<Token>>::into($token)),*];
            let found = Tokens::new($src, $mode).collect::<Vec<_>>();
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
        t!(Body, "a~b"          => T("a"), T("\u{00A0}"), T("b"));
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
        t!(Body, "/*/*abc*/"         => BC("/*abc*/"));
    }

    #[test]
    fn tokenize_body_tokens() {
        t!(Body, "_*"            => Underscore, Star);
        t!(Body, "***"           => Star, Star, Star);
        t!(Body, "[func]*bold*"  => L, T("func"), R, Star, T("bold"), Star);
        t!(Body, "hi_you_ there" => T("hi"), Underscore, T("you"), Underscore, S(0), T("there"));
        t!(Body, "# hi"          => Hashtag, S(0), T("hi"));
        t!(Body, "#()"           => Hashtag, T("()"));
        t!(Header, "_`"          => Invalid("_`"));
    }

    #[test]
    fn test_tokenize_raw() {
        // Basics.
        t!(Body, "`raw`"    => Raw("raw", 1, true));
        t!(Body, "`[func]`" => Raw("[func]", 1, true));
        t!(Body, "`]"       => Raw("]", 1, false));
        t!(Body, r"`\`` "   => Raw(r"\", 1, true), Raw(" ", 1, false));

        // Language tag.
        t!(Body, "``` hi```"     => Raw(" hi", 3, true));
        t!(Body, "```rust hi```" => Raw("rust hi", 3, true));
        t!(Body, r"``` hi\````"  => Raw(r" hi\", 3, true), Raw("", 1, false));
        t!(Body, "``` not `y`e`t finished```" => Raw(" not `y`e`t finished", 3, true));
        t!(Body, "```js   \r\n  document.write(\"go\")`"
            => Raw("js   \r\n  document.write(\"go\")`", 3, false));

        // More backticks.
        t!(Body, "`````` ``````hi"  => Raw(" ", 6, true), T("hi"));
        t!(Body, "````\n```js\nalert()\n```\n````" => Raw("\n```js\nalert()\n```\n", 4, true));
    }

    #[test]
    fn tokenize_header_tokens() {
        t!(Header, "__main__"          => Id("__main__"));
        t!(Header, "_func_box"         => Id("_func_box"));
        t!(Header, ">main"             => Invalid(">main"));
        t!(Header, "🌓, 🌍,"          => Invalid("🌓"), Comma, S(0), Invalid("🌍"), Comma);
        t!(Header, "{abc}"             => LB, Id("abc"), RB);
        t!(Header, "(1,2)"             => LP, Num(1.0), Comma, Num(2.0), RP);
        t!(Header, "12_pt, 12pt"       => Invalid("12_pt"), Comma, S(0), Len(Length::pt(12.0)));
        t!(Header, "f: arg >> g"       => Id("f"), Colon, S(0), Id("arg"), S(0), Chain, S(0), Id("g"));
        t!(Header, "=3.14"             => Equals, Num(3.14));
        t!(Header, "arg, _b, _1"       => Id("arg"), Comma, S(0), Id("_b"), Comma, S(0), Id("_1"));
        t!(Header, "a:b"               => Id("a"), Colon, Id("b"));
        t!(Header, "(){}:=,"           => LP, RP, LB, RB, Colon, Equals, Comma);
        t!(Body,   "c=d, "             => T("c=d,"), S(0));
        t!(Body,   "a: b"              => T("a:"), S(0), T("b"));
        t!(Header, "a: true, x=1"      => Id("a"), Colon, S(0), Bool(true), Comma, S(0),
                                          Id("x"), Equals, Num(1.0));
    }

    #[test]
    fn tokenize_numeric_values() {
        t!(Header, "12.3e5"            => Num(12.3e5));
        t!(Header, "120%"              => Num(1.2));
        t!(Header, "12e4%"             => Num(1200.0));
        t!(Header, "1e5in"             => Len(Length::inches(100000.0)));
        t!(Header, "2.3cm"             => Len(Length::cm(2.3)));
        t!(Header, "02.4mm"            => Len(Length::mm(2.4)));
        t!(Header, "2.4.cm"            => Invalid("2.4.cm"));
        t!(Header, "#6ae6dd"           => Hex("6ae6dd"));
        t!(Header, "#8A083c"           => Hex("8A083c"));
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
    fn tokenize_math() {
        t!(Header, "12e-3in"           => Len(Length::inches(12e-3)));
        t!(Header, "-1"                => Min, Num(1.0));
        t!(Header, "--1"               => Min, Min, Num(1.0));
        t!(Header, "- 1"               => Min, S(0), Num(1.0));
        t!(Header, "6.1cm + 4pt,a=1*2" => Len(Length::cm(6.1)), S(0), Plus, S(0), Len(Length::pt(4.0)),
                                          Comma, Id("a"), Equals, Num(1.0), Star, Num(2.0));
        t!(Header, "(5 - 1) / 2.1"     => LP, Num(5.0), S(0), Min, S(0), Num(1.0), RP,
                                          S(0), Slash, S(0), Num(2.1));
    }

    #[test]
    fn tokenize_escaped_symbols() {
        t!(Body, r"\\"       => T(r"\"));
        t!(Body, r"\["       => T("["));
        t!(Body, r"\]"       => T("]"));
        t!(Body, r"\*"       => T("*"));
        t!(Body, r"\_"       => T("_"));
        t!(Body, r"\`"       => T("`"));
        t!(Body, r"\/"       => T("/"));
        t!(Body, r"\u{2603}" => UE("2603", true));
        t!(Body, r"\u{26A4"  => UE("26A4", false));
        t!(Body, r#"\""#     => T("\""));
    }

    #[test]
    fn tokenize_unescapable_symbols() {
        t!(Body, r"\a"      => T("\\"), T("a"));
        t!(Body, r"\:"      => T(r"\"), T(":"));
        t!(Body, r"\="      => T(r"\"), T("="));
        t!(Body, r"\u{2GA4" => UE("2", false), T("GA4"));
        t!(Body, r"\u{ "    => UE("", false), Space(0));
        t!(Body, r"\u"      => T("\\"), T("u"));
        t!(Header, r"\\\\"  => Invalid(r"\\\\"));
        t!(Header, r"\a"    => Invalid(r"\a"));
        t!(Header, r"\:"    => Invalid(r"\"), Colon);
        t!(Header, r"\="    => Invalid(r"\"), Equals);
        t!(Header, r"\,"    => Invalid(r"\"), Comma);
    }

    #[test]
    fn tokenize_with_spans() {
        ts!(Body, "hello"        => s(0, 5, T("hello")));
        ts!(Body, "ab\r\nc"      => s(0, 2, T("ab")), s(2, 4, S(1)), s(4, 5, T("c")));
        ts!(Body, "// ab\r\n\nf" => s(0, 5, LC(" ab")), s(5, 8, S(2)), s(8, 9, T("f")));
        ts!(Body, "/*b*/_"       => s(0, 5, BC("b")), s(5, 6, Underscore));
        ts!(Header, "a=10"       => s(0, 1, Id("a")), s(1, 2, Equals), s(2, 4, Num(10.0)));
    }
}
