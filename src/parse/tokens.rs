use std::fmt::{self, Debug, Formatter};

use super::{is_newline, Scanner};
use crate::geom::{AngularUnit, LengthUnit};
use crate::syntax::*;

/// An iterator over the tokens of a string of source code.
#[derive(Clone)]
pub struct Tokens<'s> {
    s: Scanner<'s>,
    mode: TokenMode,
}

/// What kind of tokens to emit.
#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum TokenMode {
    /// Text and markup.
    Markup,
    /// Blocks and expressions.
    Code,
}

impl<'s> Tokens<'s> {
    /// Create a new token iterator with the given mode.
    #[inline]
    pub fn new(src: &'s str, mode: TokenMode) -> Self {
        Self { s: Scanner::new(src), mode }
    }

    /// Get the current token mode.
    #[inline]
    pub fn mode(&self) -> TokenMode {
        self.mode
    }

    /// Change the token mode.
    #[inline]
    pub fn set_mode(&mut self, mode: TokenMode) {
        self.mode = mode;
    }

    /// The index in the string at which the last token ends and next token
    /// will start.
    #[inline]
    pub fn index(&self) -> usize {
        self.s.index()
    }

    /// Jump to the given index in the string.
    ///
    /// You need to know the correct column.
    #[inline]
    pub fn jump(&mut self, index: usize) {
        self.s.jump(index);
    }

    /// The underlying scanner.
    #[inline]
    pub fn scanner(&self) -> Scanner<'s> {
        self.s
    }
}

impl<'s> Iterator for Tokens<'s> {
    type Item = Token<'s>;

    /// Parse the next token in the source code.
    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        let start = self.s.index();
        let c = self.s.eat()?;
        Some(match c {
            // Blocks and templates.
            '[' => Token::LeftBracket,
            ']' => Token::RightBracket,
            '{' => Token::LeftBrace,
            '}' => Token::RightBrace,

            // Whitespace.
            ' ' if self.s.check_or(true, |c| !c.is_whitespace()) => Token::Space(0),
            c if c.is_whitespace() => self.whitespace(),

            // Comments with special case for URLs.
            '/' if self.s.eat_if('*') => self.block_comment(),
            '/' if !self.maybe_in_url() && self.s.eat_if('/') => self.line_comment(),
            '*' if self.s.eat_if('/') => Token::Invalid(self.s.eaten_from(start)),

            // Other things.
            _ => match self.mode {
                TokenMode::Markup => self.markup(start, c),
                TokenMode::Code => self.code(start, c),
            },
        })
    }
}

impl<'s> Tokens<'s> {
    #[inline]
    fn markup(&mut self, start: usize, c: char) -> Token<'s> {
        match c {
            // Escape sequences.
            '\\' => self.backslash(),

            // Keywords and identifiers.
            '#' => self.hash(),

            // Markup.
            '~' => Token::Tilde,
            '*' => Token::Star,
            '_' => Token::Underscore,
            '`' => self.raw(),
            '$' => self.math(),
            '-' => self.hyph(start),
            '=' if self.s.check_or(true, |c| c == '=' || c.is_whitespace()) => Token::Eq,
            c if c == '.' || c.is_ascii_digit() => self.numbering(start, c),

            // Plain text.
            _ => self.text(start),
        }
    }

    fn code(&mut self, start: usize, c: char) -> Token<'s> {
        match c {
            // Parens.
            '(' => Token::LeftParen,
            ')' => Token::RightParen,

            // Length two.
            '=' if self.s.eat_if('=') => Token::EqEq,
            '!' if self.s.eat_if('=') => Token::ExclEq,
            '<' if self.s.eat_if('=') => Token::LtEq,
            '>' if self.s.eat_if('=') => Token::GtEq,
            '+' if self.s.eat_if('=') => Token::PlusEq,
            '-' if self.s.eat_if('=') => Token::HyphEq,
            '*' if self.s.eat_if('=') => Token::StarEq,
            '/' if self.s.eat_if('=') => Token::SlashEq,
            '.' if self.s.eat_if('.') => Token::Dots,
            '=' if self.s.eat_if('>') => Token::Arrow,

            // Length one.
            ',' => Token::Comma,
            ';' => Token::Semicolon,
            ':' => Token::Colon,
            '+' => Token::Plus,
            '-' => Token::Hyph,
            '*' => Token::Star,
            '/' => Token::Slash,
            '!' => Token::Excl,
            '=' => Token::Eq,
            '<' => Token::Lt,
            '>' => Token::Gt,

            // Identifiers.
            c if is_id_start(c) => self.ident(start),

            // Numbers.
            c if c.is_ascii_digit()
                || (c == '.' && self.s.check_or(false, |n| n.is_ascii_digit())) =>
            {
                self.number(start, c)
            }

            // Strings.
            '"' => self.string(),

            _ => Token::Invalid(self.s.eaten_from(start)),
        }
    }

    #[inline]
    fn text(&mut self, start: usize) -> Token<'s> {
        macro_rules! table {
            ($($c:literal)|*) => {{
                let mut t = [false; 128];
                $(t[$c as usize] = true;)*
                t
            }}
        }

        const TABLE: [bool; 128] = table! {
            // Ascii whitespace.
            ' ' | '\t' | '\n' | '\x0b' | '\x0c' | '\r' |
            // Comments, parentheses, code.
            '/' | '[' | ']' | '{' | '}' | '#' |
            // Markup
            '~' | '*' | '_' | '`' | '$' | '-' | '\\'
        };

        self.s.eat_until(|c| {
            TABLE.get(c as usize).copied().unwrap_or_else(|| c.is_whitespace())
        });

        Token::Text(self.s.eaten_from(start))
    }

    fn whitespace(&mut self) -> Token<'s> {
        self.s.uneat();

        // Count the number of newlines.
        let mut newlines = 0;
        while let Some(c) = self.s.eat() {
            if !c.is_whitespace() {
                self.s.uneat();
                break;
            }

            if is_newline(c) {
                if c == '\r' {
                    self.s.eat_if('\n');
                }
                newlines += 1;
            }
        }

        Token::Space(newlines)
    }

    fn backslash(&mut self) -> Token<'s> {
        if let Some(c) = self.s.peek() {
            match c {
                // Backslash and comments.
                '\\' | '/' |
                // Parenthesis and hashtag.
                '[' | ']' | '{' | '}' | '#' |
                // Markup.
                '*' | '_' | '=' | '~' | '`' | '$' => {
                    let start = self.s.index();
                    self.s.eat_assert(c);
                    Token::Text(&self.s.eaten_from(start))
                }
                'u' if self.s.rest().starts_with("u{") => {
                    self.s.eat_assert('u');
                    self.s.eat_assert('{');
                    Token::UnicodeEscape(UnicodeEscapeToken {
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

    #[inline]
    fn hash(&mut self) -> Token<'s> {
        if self.s.check_or(false, is_id_start) {
            let read = self.s.eat_while(is_id_continue);
            if let Some(keyword) = keyword(read) {
                keyword
            } else {
                Token::Ident(read)
            }
        } else {
            Token::Invalid("#")
        }
    }

    fn hyph(&mut self, start: usize) -> Token<'s> {
        if self.s.eat_if('-') {
            if self.s.eat_if('-') {
                Token::HyphHyphHyph
            } else {
                Token::HyphHyph
            }
        } else if self.s.check_or(true, char::is_whitespace) {
            Token::Hyph
        } else {
            Token::Text(self.s.eaten_from(start))
        }
    }

    fn numbering(&mut self, start: usize, c: char) -> Token<'s> {
        let number = if c != '.' {
            self.s.eat_while(|c| c.is_ascii_digit());
            let read = self.s.eaten_from(start);
            if !self.s.eat_if('.') {
                return Token::Text(read);
            }
            read.parse().ok()
        } else {
            None
        };

        if self.s.check_or(true, char::is_whitespace) {
            Token::Numbering(number)
        } else {
            Token::Text(self.s.eaten_from(start))
        }
    }

    fn raw(&mut self) -> Token<'s> {
        let mut backticks = 1;
        while self.s.eat_if('`') {
            backticks += 1;
        }

        // Special case for empty inline block.
        if backticks == 2 {
            return Token::Raw(RawToken { text: "", backticks: 1, terminated: true });
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

        Token::Raw(RawToken {
            text: self.s.get(start .. end),
            backticks,
            terminated,
        })
    }

    fn math(&mut self) -> Token<'s> {
        let mut display = false;
        if self.s.eat_if('[') {
            display = true;
        }

        let start = self.s.index();

        let mut escaped = false;
        let mut dollar = !display;

        let terminated = loop {
            match self.s.eat() {
                Some('$') if !escaped && dollar => break true,
                Some(']') if !escaped => dollar = true,
                Some(c) => {
                    dollar = !display;
                    escaped = c == '\\' && !escaped;
                }
                None => break false,
            }
        };

        let end = self.s.index()
            - match (terminated, display) {
                (false, _) => 0,
                (true, false) => 1,
                (true, true) => 2,
            };

        Token::Math(MathToken {
            formula: self.s.get(start .. end),
            display,
            terminated,
        })
    }

    fn ident(&mut self, start: usize) -> Token<'s> {
        self.s.eat_while(is_id_continue);
        match self.s.eaten_from(start) {
            "none" => Token::None,
            "auto" => Token::Auto,
            "true" => Token::Bool(true),
            "false" => Token::Bool(false),
            id => keyword(id).unwrap_or(Token::Ident(id)),
        }
    }

    fn number(&mut self, start: usize, c: char) -> Token<'s> {
        // Read the first part (integer or fractional depending on `first`).
        self.s.eat_while(|c| c.is_ascii_digit());

        // Read the fractional part if not already done.
        // Make sure not to confuse a range for the decimal separator.
        if c != '.' && !self.s.rest().starts_with("..") && self.s.eat_if('.') {
            self.s.eat_while(|c| c.is_ascii_digit());
        }

        // Read the exponent.
        if self.s.eat_if('e') || self.s.eat_if('E') {
            let _ = self.s.eat_if('+') || self.s.eat_if('-');
            self.s.eat_while(|c| c.is_ascii_digit());
        }

        // Read the suffix.
        let suffix_start = self.s.index();
        if !self.s.eat_if('%') {
            self.s.eat_while(|c| c.is_ascii_alphanumeric());
        }

        let number = self.s.get(start .. suffix_start);
        let suffix = self.s.eaten_from(suffix_start);
        let all = self.s.eaten_from(start);

        // Find out whether it is a simple number.
        if suffix.is_empty() {
            if let Ok(int) = number.parse::<i64>() {
                return Token::Int(int);
            } else if let Ok(float) = number.parse::<f64>() {
                return Token::Float(float);
            }
        }

        // Otherwise parse into the fitting numeric type.
        let build = match suffix {
            "%" => Token::Percent,
            "fr" => Token::Fraction,
            "pt" => |x| Token::Length(x, LengthUnit::Pt),
            "mm" => |x| Token::Length(x, LengthUnit::Mm),
            "cm" => |x| Token::Length(x, LengthUnit::Cm),
            "in" => |x| Token::Length(x, LengthUnit::In),
            "rad" => |x| Token::Angle(x, AngularUnit::Rad),
            "deg" => |x| Token::Angle(x, AngularUnit::Deg),
            _ => return Token::Invalid(all),
        };

        if let Ok(float) = number.parse::<f64>() {
            build(float)
        } else {
            Token::Invalid(all)
        }
    }

    fn string(&mut self) -> Token<'s> {
        let mut escaped = false;
        Token::Str(StrToken {
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

    fn maybe_in_url(&self) -> bool {
        self.mode == TokenMode::Markup && self.s.eaten().ends_with(":/")
    }
}

impl Debug for Tokens<'_> {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        write!(f, "Tokens({}|{})", self.s.eaten(), self.s.rest())
    }
}

fn keyword(ident: &str) -> Option<Token<'static>> {
    Some(match ident {
        "not" => Token::Not,
        "and" => Token::And,
        "or" => Token::Or,
        "with" => Token::With,
        "let" => Token::Let,
        "if" => Token::If,
        "else" => Token::Else,
        "for" => Token::For,
        "in" => Token::In,
        "while" => Token::While,
        "break" => Token::Break,
        "continue" => Token::Continue,
        "return" => Token::Return,
        "import" => Token::Import,
        "include" => Token::Include,
        "from" => Token::From,
        _ => return None,
    })
}

#[cfg(test)]
#[allow(non_snake_case)]
mod tests {
    use super::*;

    use Option::None;
    use Token::{Ident, *};
    use TokenMode::{Code, Markup};

    const fn UnicodeEscape(sequence: &str, terminated: bool) -> Token {
        Token::UnicodeEscape(UnicodeEscapeToken { sequence, terminated })
    }

    const fn Raw(text: &str, backticks: usize, terminated: bool) -> Token {
        Token::Raw(RawToken { text, backticks, terminated })
    }

    const fn Math(formula: &str, display: bool, terminated: bool) -> Token {
        Token::Math(MathToken { formula, display, terminated })
    }

    const fn Str(string: &str, terminated: bool) -> Token {
        Token::Str(StrToken { string, terminated })
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
        (' ', None, " ", Space(0)),
        (' ', None, "\n", Space(1)),
        (' ', None, "\r", Space(1)),
        (' ', None, "\r\n", Space(1)),
        // Letter suffixes.
        ('a', Some(Markup), "hello", Text("hello")),
        ('a', Some(Markup), "💚", Text("💚")),
        ('a', Some(Code), "val", Ident("val")),
        ('a', Some(Code), "α", Ident("α")),
        ('a', Some(Code), "_", Ident("_")),
        // Number suffixes.
        ('1', Some(Code), "2", Int(2)),
        ('1', Some(Code), ".2", Float(0.2)),
        // Symbol suffixes.
        ('/', None, "[", LeftBracket),
        ('/', None, "//", LineComment("")),
        ('/', None, "/**/", BlockComment("")),
        ('/', Some(Markup), "*", Star),
        ('/', Some(Markup), "$ $", Math(" ", false, true)),
        ('/', Some(Markup), r"\\", Text(r"\")),
        ('/', Some(Markup), "#let", Let),
        ('/', Some(Code), "(", LeftParen),
        ('/', Some(Code), ":", Colon),
        ('/', Some(Code), "+=", PlusEq),
    ];

    macro_rules! t {
        (Both $($tts:tt)*) => {
            t!(Markup $($tts)*);
            t!(Code $($tts)*);
        };
        ($mode:ident $([$blocks:literal])?: $src:expr => $($token:expr),*) => {{
            // Test without suffix.
            t!(@$mode: $src => $($token),*);

            // Test with each applicable suffix.
            for &(block, mode, suffix, token) in SUFFIXES {
                let src = $src;
                #[allow(unused_variables)]
                let blocks = BLOCKS;
                $(let blocks = $blocks;)?
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
            check(&src, exp, found);
        }};
    }

    #[track_caller]
    fn check<T>(src: &str, exp: T, found: T)
    where
        T: Debug + PartialEq,
    {
        if exp != found {
            println!("source:   {:?}", src);
            println!("expected: {:#?}", exp);
            println!("found:    {:#?}", found);
            panic!("test failed");
        }
    }

    #[test]
    fn test_tokenize_brackets() {
        // Test in markup.
        t!(Markup: "["       => LeftBracket);
        t!(Markup: "]"       => RightBracket);
        t!(Markup: "{"       => LeftBrace);
        t!(Markup: "}"       => RightBrace);
        t!(Markup[" /"]: "(" => Text("("));
        t!(Markup[" /"]: ")" => Text(")"));

        // Test in code.
        t!(Code: "[" => LeftBracket);
        t!(Code: "]" => RightBracket);
        t!(Code: "{" => LeftBrace);
        t!(Code: "}" => RightBrace);
        t!(Code: "(" => LeftParen);
        t!(Code: ")" => RightParen);
    }

    #[test]
    fn test_tokenize_whitespace() {
        // Test basic whitespace.
        t!(Both["a1/"]: ""         => );
        t!(Both["a1/"]: " "        => Space(0));
        t!(Both["a1/"]: "    "     => Space(0));
        t!(Both["a1/"]: "\t"       => Space(0));
        t!(Both["a1/"]: "  \t"     => Space(0));
        t!(Both["a1/"]: "\u{202F}" => Space(0));

        // Test newline counting.
        t!(Both["a1/"]: "\n"           => Space(1));
        t!(Both["a1/"]: "\n "          => Space(1));
        t!(Both["a1/"]: "  \n"         => Space(1));
        t!(Both["a1/"]: "  \n   "      => Space(1));
        t!(Both["a1/"]: "\r\n"         => Space(1));
        t!(Both["a1/"]: "  \n\t \n  "  => Space(2));
        t!(Both["a1/"]: "\n\r"         => Space(2));
        t!(Both["a1/"]: " \r\r\n \x0D" => Space(3));
    }

    #[test]
    fn test_tokenize_text() {
        // Test basic text.
        t!(Markup[" /"]: "hello"       => Text("hello"));
        t!(Markup[" /"]: "hello-world" => Text("hello"), Text("-"), Text("world"));

        // Test code symbols in text.
        t!(Markup[" /"]: "a():\"b" => Text("a():\"b"));
        t!(Markup[" /"]: ";:,|/+"  => Text(";:,|"), Text("/+"));
        t!(Markup[" /"]: "=-a"     => Text("="), Text("-"), Text("a"));
        t!(Markup[" "]: "#123"     => Invalid("#"), Text("123"));

        // Test text ends.
        t!(Markup[""]: "hello " => Text("hello"), Space(0));
        t!(Markup[""]: "hello~" => Text("hello"), Tilde);
    }

    #[test]
    fn test_tokenize_escape_sequences() {
        // Test escapable symbols.
        t!(Markup: r"\\" => Text(r"\"));
        t!(Markup: r"\/" => Text("/"));
        t!(Markup: r"\[" => Text("["));
        t!(Markup: r"\]" => Text("]"));
        t!(Markup: r"\{" => Text("{"));
        t!(Markup: r"\}" => Text("}"));
        t!(Markup: r"\*" => Text("*"));
        t!(Markup: r"\_" => Text("_"));
        t!(Markup: r"\=" => Text("="));
        t!(Markup: r"\~" => Text("~"));
        t!(Markup: r"\`" => Text("`"));
        t!(Markup: r"\$" => Text("$"));
        t!(Markup: r"\#" => Text("#"));

        // Test unescapable symbols.
        t!(Markup[" /"]: r"\a"   => Text(r"\"), Text("a"));
        t!(Markup[" /"]: r"\u"   => Text(r"\"), Text("u"));
        t!(Markup[" /"]: r"\1"   => Text(r"\"), Text("1"));
        t!(Markup[" /"]: r#"\""# => Text(r"\"), Text("\""));

        // Test basic unicode escapes.
        t!(Markup: r"\u{}"     => UnicodeEscape("", true));
        t!(Markup: r"\u{2603}" => UnicodeEscape("2603", true));
        t!(Markup: r"\u{P}"    => UnicodeEscape("P", true));

        // Test unclosed unicode escapes.
        t!(Markup[" /"]: r"\u{"     => UnicodeEscape("", false));
        t!(Markup[" /"]: r"\u{1"    => UnicodeEscape("1", false));
        t!(Markup[" /"]: r"\u{26A4" => UnicodeEscape("26A4", false));
        t!(Markup[" /"]: r"\u{1Q3P" => UnicodeEscape("1Q3P", false));
        t!(Markup: r"\u{1🏕}"       => UnicodeEscape("1", false), Text("🏕"), RightBrace);
    }

    #[test]
    fn test_tokenize_markup_symbols() {
        // Test markup tokens.
        t!(Markup[" a1"]: "*"   => Star);
        t!(Markup: "_"          => Underscore);
        t!(Markup[""]: "==="    => Eq, Eq, Eq);
        t!(Markup["a1/"]: "= "  => Eq, Space(0));
        t!(Markup: "~"          => Tilde);
        t!(Markup[" "]: r"\"    => Backslash);
        t!(Markup["a "]: r"a--" => Text("a"), HyphHyph);
        t!(Markup["a1/"]: "- "  => Hyph, Space(0));
        t!(Markup[" "]: "."     => Numbering(None));
        t!(Markup[" "]: "1."    => Numbering(Some(1)));
        t!(Markup[" "]: "1.a"   => Text("1."), Text("a"));
        t!(Markup[" /"]: "a1."  => Text("a1."));
    }

    #[test]
    fn test_tokenize_code_symbols() {
        // Test all symbols.
        t!(Code: ","        => Comma);
        t!(Code: ";"        => Semicolon);
        t!(Code: ":"        => Colon);
        t!(Code: "+"        => Plus);
        t!(Code: "-"        => Hyph);
        t!(Code[" a1"]: "*" => Star);
        t!(Code[" a1"]: "/" => Slash);
        t!(Code: "="        => Eq);
        t!(Code: "=="       => EqEq);
        t!(Code: "!="       => ExclEq);
        t!(Code: "<"        => Lt);
        t!(Code: "<="       => LtEq);
        t!(Code: ">"        => Gt);
        t!(Code: ">="       => GtEq);
        t!(Code: "+="       => PlusEq);
        t!(Code: "-="       => HyphEq);
        t!(Code: "*="       => StarEq);
        t!(Code: "/="       => SlashEq);
        t!(Code: ".."       => Dots);
        t!(Code: "=>"       => Arrow);

        // Test combinations.
        t!(Code: "<=>"        => LtEq, Gt);
        t!(Code[" a/"]: "..." => Dots, Invalid("."));

        // Test hyphen as symbol vs part of identifier.
        t!(Code[" /"]: "-1"   => Hyph, Int(1));
        t!(Code[" /"]: "-a"   => Hyph, Ident("a"));
        t!(Code[" /"]: "--1"  => Hyph, Hyph, Int(1));
        t!(Code[" /"]: "--_a" => Hyph, Hyph, Ident("_a"));
        t!(Code[" /"]: "a-b"  => Ident("a-b"));
    }

    #[test]
    fn test_tokenize_keywords() {
        // A list of a few (not all) keywords.
        let list = [
            ("not", Not),
            ("let", Let),
            ("if", If),
            ("else", Else),
            ("for", For),
            ("in", In),
            ("import", Import),
        ];

        for &(s, t) in &list {
            t!(Markup[" "]: format!("#{}", s) => t);
            t!(Markup[" "]: format!("#{0}#{0}", s) => t, t);
            t!(Markup[" /"]: format!("# {}", s) => Token::Invalid("#"), Space(0), Text(s));
        }

        for &(s, t) in &list {
            t!(Code[" "]: s => t);
            t!(Markup[" /"]: s => Text(s));
        }

        // Test simple identifier.
        t!(Markup[" "]: "#letter" => Ident("letter"));
        t!(Code[" /"]: "falser"   => Ident("falser"));
        t!(Code[" /"]: "None"     => Ident("None"));
        t!(Code[" /"]: "True"     => Ident("True"));
    }

    #[test]
    fn test_tokenize_raw_blocks() {
        let empty = Raw("", 1, true);

        // Test basic raw block.
        t!(Markup: "``"     => empty);
        t!(Markup: "`raw`"  => Raw("raw", 1, true));
        t!(Markup[""]: "`]" => Raw("]", 1, false));

        // Test special symbols in raw block.
        t!(Markup: "`[brackets]`" => Raw("[brackets]", 1, true));
        t!(Markup[""]: r"`\`` "   => Raw(r"\", 1, true), Raw(" ", 1, false));

        // Test separated closing backticks.
        t!(Markup: "```not `y`e`t```" => Raw("not `y`e`t", 3, true));

        // Test more backticks.
        t!(Markup: "``nope``"             => empty, Text("nope"), empty);
        t!(Markup: "````🚀````"           => Raw("🚀", 4, true));
        t!(Markup[""]: "`````👩‍🚀````noend" => Raw("👩‍🚀````noend", 5, false));
        t!(Markup[""]: "````raw``````"    => Raw("raw", 4, true), empty);
    }

    #[test]
    fn test_tokenize_math_formulas() {
        // Test basic formula.
        t!(Markup: "$$"        => Math("", false, true));
        t!(Markup: "$x$"       => Math("x", false, true));
        t!(Markup: r"$\\$"     => Math(r"\\", false, true));
        t!(Markup: "$[x + y]$" => Math("x + y", true, true));
        t!(Markup: r"$[\\]$"   => Math(r"\\", true, true));

        // Test unterminated.
        t!(Markup[""]: "$x"      => Math("x", false, false));
        t!(Markup[""]: "$[x"     => Math("x", true, false));
        t!(Markup[""]: "$[x]\n$" => Math("x]\n$", true, false));

        // Test escape sequences.
        t!(Markup: r"$\$x$"       => Math(r"\$x", false, true));
        t!(Markup: r"$[\\\]$]$"   => Math(r"\\\]$", true, true));
        t!(Markup[""]: r"$[ ]\\$" => Math(r" ]\\$", true, false));
    }

    #[test]
    fn test_tokenize_idents() {
        // Test valid identifiers.
        t!(Code[" /"]: "x"           => Ident("x"));
        t!(Code[" /"]: "value"       => Ident("value"));
        t!(Code[" /"]: "__main__"    => Ident("__main__"));
        t!(Code[" /"]: "_snake_case" => Ident("_snake_case"));

        // Test non-ascii.
        t!(Code[" /"]: "α"    => Ident("α"));
        t!(Code[" /"]: "ម្តាយ" => Ident("ម្តាយ"));

        // Test hyphen parsed as identifier.
        t!(Code[" /"]: "kebab-case" => Ident("kebab-case"));
        t!(Code[" /"]: "one-10"     => Ident("one-10"));
    }

    #[test]
    fn test_tokenize_numeric() {
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
            t!(Code[" /"]: s => Int(v));
        }

        // Test floats.
        for &(s, v) in &floats {
            t!(Code[" /"]: s => Float(v));
        }

        // Test attached numbers.
        t!(Code[" /"]: ".2.3"  => Float(0.2), Float(0.3));
        t!(Code[" /"]: "1.2.3"  => Float(1.2), Float(0.3));
        t!(Code[" /"]: "1e-2+3" => Float(0.01), Plus, Int(3));

        // Test float from too large integer.
        let large = i64::MAX as f64 + 1.0;
        t!(Code[" /"]: large.to_string() => Float(large));

        // Combined integers and floats.
        let nums = ints.iter().map(|&(k, v)| (k, v as f64)).chain(floats);

        let suffixes = [
            ("%", Percent as fn(f64) -> Token<'static>),
            ("fr", Fraction as fn(f64) -> Token<'static>),
            ("mm", |x| Length(x, LengthUnit::Mm)),
            ("pt", |x| Length(x, LengthUnit::Pt)),
            ("cm", |x| Length(x, LengthUnit::Cm)),
            ("in", |x| Length(x, LengthUnit::In)),
            ("rad", |x| Angle(x, AngularUnit::Rad)),
            ("deg", |x| Angle(x, AngularUnit::Deg)),
        ];

        // Numeric types.
        for &(suffix, build) in &suffixes {
            for (s, v) in nums.clone() {
                t!(Code[" /"]: format!("{}{}", s, suffix) => build(v));
            }
        }

        // Multiple dots close the number.
        t!(Code[" /"]: "1..2"   => Int(1), Dots, Int(2));
        t!(Code[" /"]: "1..2.3" => Int(1), Dots, Float(2.3));
        t!(Code[" /"]: "1.2..3" => Float(1.2), Dots, Int(3));
    }

    #[test]
    fn test_tokenize_strings() {
        // Test basic strings.
        t!(Code: "\"hi\""        => Str("hi", true));
        t!(Code: "\"hi\nthere\"" => Str("hi\nthere", true));
        t!(Code: "\"🌎\""        => Str("🌎", true));

        // Test unterminated.
        t!(Code[""]: "\"hi"      => Str("hi", false));

        // Test escaped quote.
        t!(Code: r#""a\"bc""# => Str(r#"a\"bc"#, true));
        t!(Code[""]: r#""\""# => Str(r#"\""#, false));
    }

    #[test]
    fn test_tokenize_line_comments() {
        // Test line comment with no trailing newline.
        t!(Both[""]: "//" => LineComment(""));

        // Test line comment ends at newline.
        t!(Both["a1/"]: "//bc\n"   => LineComment("bc"), Space(1));
        t!(Both["a1/"]: "// bc \n" => LineComment(" bc "), Space(1));
        t!(Both["a1/"]: "//bc\r\n" => LineComment("bc"), Space(1));

        // Test nested line comments.
        t!(Both["a1/"]: "//a//b\n" => LineComment("a//b"), Space(1));
    }

    #[test]
    fn test_tokenize_block_comments() {
        // Test basic block comments.
        t!(Both[""]: "/*" => BlockComment(""));
        t!(Both: "/**/"   => BlockComment(""));
        t!(Both: "/*🏞*/" => BlockComment("🏞"));
        t!(Both: "/*\n*/" => BlockComment("\n"));

        // Test depth 1 and 2 nested block comments.
        t!(Both: "/* /* */ */"  => BlockComment(" /* */ "));
        t!(Both: "/*/*/**/*/*/" => BlockComment("/*/**/*/"));

        // Test two nested, one unclosed block comments.
        t!(Both[""]: "/*/*/**/*/" => BlockComment("/*/**/*/"));

        // Test all combinations of up to two following slashes and stars.
        t!(Both[""]: "/*"   => BlockComment(""));
        t!(Both[""]: "/*/"  => BlockComment("/"));
        t!(Both[""]: "/**"  => BlockComment("*"));
        t!(Both[""]: "/*//" => BlockComment("//"));
        t!(Both[""]: "/*/*" => BlockComment("/*"));
        t!(Both[""]: "/**/" => BlockComment(""));
        t!(Both[""]: "/***" => BlockComment("**"));
    }

    #[test]
    fn test_tokenize_invalid() {
        // Test invalidly closed block comments.
        t!(Both: "*/"     => Token::Invalid("*/"));
        t!(Both: "/**/*/" => BlockComment(""), Token::Invalid("*/"));

        // Test invalid expressions.
        t!(Code: r"\"        => Invalid(r"\"));
        t!(Code: "🌓"        => Invalid("🌓"));
        t!(Code: r"\:"       => Invalid(r"\"), Colon);
        t!(Code: "meal⌚"    => Ident("meal"), Invalid("⌚"));
        t!(Code[" /"]: r"\a" => Invalid(r"\"), Ident("a"));
        t!(Code[" /"]: "#"   => Invalid("#"));

        // Test invalid number suffixes.
        t!(Code[" /"]: "1foo" => Invalid("1foo"));
        t!(Code: "1p%"        => Invalid("1p"), Invalid("%"));
        t!(Code: "1%%"        => Percent(1.0), Invalid("%"));
    }
}
