//! Tokenization.

use std::fmt::{self, Debug, Formatter};

use super::{is_newline, Scanner};
use crate::length::Length;
use crate::syntax::token::*;
use crate::syntax::{is_ident, Pos};

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

    /// Jump to a position in the source string.
    pub fn jump(&mut self, pos: Pos) {
        self.s.jump(pos.to_usize());
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
        Some(match self.s.eat()? {
            // Whitespace with fast path for just a single space.
            ' ' if !self.s.check(|c| c.is_whitespace()) => Token::Space(0),
            c if c.is_whitespace() => {
                self.s.jump(start);
                self.read_whitespace()
            }

            // Comments.
            '/' if self.s.eat_if('/') => self.read_line_comment(),
            '/' if self.s.eat_if('*') => self.read_block_comment(),
            '*' if self.s.eat_if('/') => Token::Invalid("*/"),

            // Functions.
            '[' => Token::LeftBracket,
            ']' => Token::RightBracket,
            '{' => Token::LeftBrace,
            '}' => Token::RightBrace,

            // Syntactic elements in body text.
            '*' if self.mode == Body => Token::Star,
            '_' if self.mode == Body => Token::Underscore,
            '#' if self.mode == Body => Token::Hashtag,
            '~' if self.mode == Body => Token::NonBreakingSpace,
            '`' if self.mode == Body => self.read_raw(),
            '\\' if self.mode == Body => self.read_escaped(),

            // Syntactic elements in headers.
            '(' if self.mode == Header => Token::LeftParen,
            ')' if self.mode == Header => Token::RightParen,
            ':' if self.mode == Header => Token::Colon,
            ',' if self.mode == Header => Token::Comma,
            '=' if self.mode == Header => Token::Equals,
            '>' if self.mode == Header && self.s.eat_if('>') => Token::Chain,
            '+' if self.mode == Header => Token::Plus,
            '-' if self.mode == Header => Token::Hyphen,
            '*' if self.mode == Header => Token::Star,
            '/' if self.mode == Header => Token::Slash,

            // Expressions in headers.
            '#' if self.mode == Header => self.read_hex(),
            '"' if self.mode == Header => self.read_string(),

            // Expressions or just plain text.
            _ => self.read_text_or_expr(start),
        })
    }
}

impl<'s> Tokens<'s> {
    fn read_whitespace(&mut self) -> Token<'s> {
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

    fn read_line_comment(&mut self) -> Token<'s> {
        Token::LineComment(self.s.eat_until(is_newline))
    }

    fn read_block_comment(&mut self) -> Token<'s> {
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

    fn read_raw(&mut self) -> Token<'s> {
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

    fn read_escaped(&mut self) -> Token<'s> {
        if let Some(c) = self.s.peek() {
            match c {
                '[' | ']' | '\\' | '/' | '*' | '_' | '`' | '"' | '#' | '~' => {
                    let start = self.s.index();
                    self.s.eat_assert(c);
                    Token::Text(&self.s.eaten_from(start))
                }
                'u' if self.s.peek_nth(1) == Some('{') => {
                    self.s.eat_assert('u');
                    self.s.eat_assert('{');
                    Token::UnicodeEscape(TokenUnicodeEscape {
                        sequence: self.s.eat_while(|c| c.is_ascii_hexdigit()),
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

    fn read_hex(&mut self) -> Token<'s> {
        // This parses more than the permissable 0-9, a-f, A-F character ranges
        // to provide nicer error messages later.
        Token::Hex(self.s.eat_while(|c| c.is_ascii_alphanumeric()))
    }

    fn read_string(&mut self) -> Token<'s> {
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

    fn read_text_or_expr(&mut self, start: usize) -> Token<'s> {
        let body = self.mode == Body;
        let header = self.mode == Header;

        let mut last_was_e = false;
        self.s.eat_until(|c| {
            let end = match c {
                c if c.is_whitespace() => true,
                '[' | ']' | '{' | '}' | '*' | '/' | '#' => true,
                '_' | '`' | '~' | '\\' if body => true,
                '(' | ')' | ':' | ',' | '=' | '"' if header => true,
                '+' | '-' if header && !last_was_e => true,
                _ => false,
            };
            last_was_e = c == 'e' || c == 'E';
            end
        });

        let read = self.s.eaten_from(start);
        if self.mode == Header {
            parse_expr(read)
        } else {
            Token::Text(read)
        }
    }
}

impl Debug for Tokens<'_> {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        write!(f, "Tokens({}|{})", self.s.eaten(), self.s.rest())
    }
}

fn parse_expr(text: &str) -> Token<'_> {
    if let Ok(b) = text.parse::<bool>() {
        Token::Bool(b)
    } else if let Ok(int) = text.parse::<i64>() {
        Token::Int(int)
    } else if let Ok(num) = text.parse::<f64>() {
        Token::Float(num)
    } else if let Some(percent) = parse_percent(text) {
        Token::Percent(percent)
    } else if let Ok(length) = text.parse::<Length>() {
        Token::Length(length)
    } else if is_ident(text) {
        Token::Ident(text)
    } else {
        Token::Invalid(text)
    }
}

fn parse_percent(text: &str) -> Option<f64> {
    text.strip_suffix('%').and_then(|num| num.parse::<f64>().ok())
}

#[cfg(test)]
#[allow(non_snake_case)]
mod tests {
    use super::*;
    use crate::length::Length;
    use crate::parse::tests::check;

    use Token::{
        BlockComment as BC, Bool, Chain, Float, Hex, Hyphen as Min, Ident as Id, Int,
        LeftBrace as LB, LeftBracket as L, LeftParen as LP, Length as Len,
        LineComment as LC, NonBreakingSpace as Nbsp, Percent, Plus, RightBrace as RB,
        RightBracket as R, RightParen as RP, Slash, Space as S, Star, Text as T, *,
    };

    fn Str(string: &str, terminated: bool) -> Token {
        Token::Str(TokenStr { string, terminated })
    }
    fn Raw(text: &str, backticks: usize, terminated: bool) -> Token {
        Token::Raw(TokenRaw { text, backticks, terminated })
    }
    fn UE(sequence: &str, terminated: bool) -> Token {
        Token::UnicodeEscape(TokenUnicodeEscape { sequence, terminated })
    }

    macro_rules! t {
        ($mode:expr, $src:expr => $($token:expr),*) => {
            let exp = vec![$($token),*];
            let found = Tokens::new($src, $mode).collect::<Vec<_>>();
            check($src, exp, found, false);
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
        t!(Body, "a~b"          => T("a"), Nbsp, T("b"));
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
        t!(Body, "a_*"           => T("a"), Underscore, Star);
        t!(Body, "a***"          => T("a"), Star, Star, Star);
        t!(Body, "[func]*bold*"  => L, T("func"), R, Star, T("bold"), Star);
        t!(Body, "hi_you_ there" => T("hi"), Underscore, T("you"), Underscore, S(0), T("there"));
        t!(Body, "# hi"          => Hashtag, S(0), T("hi"));
        t!(Body, "ab# hi"        => T("ab"), Hashtag, S(0), T("hi"));
        t!(Body, "#{}"           => Hashtag, LB, RB);
        t!(Body, "{text}"        => LB, Text("text"), RB);
        t!(Header, "_`"          => Invalid("_`"));
    }

    #[test]
    fn test_tokenize_raw() {
        // Basics.
        t!(Body, "a`raw`"   => T("a"), Raw("raw", 1, true));
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
    fn tokenize_header_tokens() {
        t!(Header, "__main__"     => Id("__main__"));
        t!(Header, "_func_box"    => Id("_func_box"));
        t!(Header, ">main"        => Invalid(">main"));
        t!(Header, "🌓, 🌍,"     => Invalid("🌓"), Comma, S(0), Invalid("🌍"), Comma);
        t!(Header, "{abc}"        => LB, Id("abc"), RB);
        t!(Header, "(1,2)"        => LP, Int(1), Comma, Int(2), RP);
        t!(Header, "12_pt, 12pt"  => Invalid("12_pt"), Comma, S(0), Len(Length::pt(12.0)));
        t!(Header, "f: arg >> g"  => Id("f"), Colon, S(0), Id("arg"), S(0), Chain, S(0), Id("g"));
        t!(Header, "=3.14"        => Equals, Float(3.14));
        t!(Header, "arg, _b, _1"  => Id("arg"), Comma, S(0), Id("_b"), Comma, S(0), Id("_1"));
        t!(Header, "a:b"          => Id("a"), Colon, Id("b"));
        t!(Header, "(){}:=,"      => LP, RP, LB, RB, Colon, Equals, Comma);
        t!(Body,   "c=d, "        => T("c=d,"), S(0));
        t!(Body,   "a: b"         => T("a:"), S(0), T("b"));
        t!(Header, "a: true, x=1" => Id("a"), Colon, S(0), Bool(true), Comma, S(0),
                                     Id("x"), Equals, Int(1));
    }

    #[test]
    fn tokenize_numeric_values() {
        t!(Header, "12.3e5"  => Float(12.3e5));
        t!(Header, "120%"    => Percent(120.0));
        t!(Header, "12e4%"   => Percent(120000.0));
        t!(Header, "1e5in"   => Len(Length::inches(100000.0)));
        t!(Header, "2.3cm"   => Len(Length::cm(2.3)));
        t!(Header, "02.4mm"  => Len(Length::mm(2.4)));
        t!(Header, "2.4.cm"  => Invalid("2.4.cm"));
        t!(Header, "#6ae6dd" => Hex("6ae6dd"));
        t!(Header, "#8A083c" => Hex("8A083c"));
    }

    #[test]
    fn tokenize_strings() {
        t!(Body, "a \"hi\" string"           => T("a"), S(0), T("\"hi\""), S(0), T("string"));
        t!(Header, "\"hello"                 => Str("hello", false));
        t!(Header, "\"hello world\""         => Str("hello world", true));
        t!(Header, "\"hello\nworld\""        => Str("hello\nworld", true));
        t!(Header, r#"1"hello\nworld"false"# => Int(1), Str("hello\\nworld", true), Bool(false));
        t!(Header, r#""a\"bc""#              => Str(r#"a\"bc"#, true));
        t!(Header, r#""a\\"bc""#             => Str(r#"a\\"#, true), Id("bc"), Str("", false));
        t!(Header, r#""a\tbc"#               => Str("a\\tbc", false));
        t!(Header, "\"🌎\""                  => Str("🌎", true));
    }

    #[test]
    fn tokenize_math() {
        t!(Header, "12e-3in"           => Len(Length::inches(12e-3)));
        t!(Header, "-1"                => Min, Int(1));
        t!(Header, "--1"               => Min, Min, Int(1));
        t!(Header, "- 1"               => Min, S(0), Int(1));
        t!(Header, "6.1cm + 4pt,a=1*2" => Len(Length::cm(6.1)), S(0), Plus, S(0), Len(Length::pt(4.0)),
                                          Comma, Id("a"), Equals, Int(1), Star, Int(2));
        t!(Header, "(5 - 1) / 2.1"     => LP, Int(5), S(0), Min, S(0), Int(1), RP,
                                          S(0), Slash, S(0), Float(2.1));
    }
}
