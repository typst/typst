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
                // Functions, blocks and terminators.
                '[' => Token::LeftBracket,
                ']' => Token::RightBracket,
                '{' => Token::LeftBrace,
                '}' => Token::RightBrace,

                // Whitespace.
                c if c.is_whitespace() => self.whitespace(c),

                // Comments.
                '/' if self.s.eat_if('/') => self.line_comment(),
                '/' if self.s.eat_if('*') => self.block_comment(),
                '*' if self.s.eat_if('/') => Token::Invalid(self.s.eaten_from(start)),

                _ => break,
            });
        }

        Some(match self.mode {
            TokenMode::Markup => match c {
                // Markup.
                '*' => Token::Star,
                '_' => Token::Underscore,
                '~' => Token::Tilde,
                '#' => self.hash(start),
                '`' => self.raw(),
                '$' => self.math(),
                '\\' => self.backslash(),

                // Plain text.
                _ => self.text(start),
            },

            TokenMode::Code => match c {
                // Parens.
                '(' => Token::LeftParen,
                ')' => Token::RightParen,

                // Length two.
                '=' if self.s.eat_if('=') => Token::EqEq,
                '!' if self.s.eat_if('=') => Token::BangEq,
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
                '|' => Token::Pipe,
                '+' => Token::Plus,
                '-' => Token::Hyph,
                '*' => Token::Star,
                '/' => Token::Slash,
                '=' => Token::Eq,
                '<' => Token::Lt,
                '>' => Token::Gt,
                '?' => Token::Question,

                // Identifiers.
                c if is_id_start(c) => self.ident(start),

                // Numbers.
                c if c.is_ascii_digit()
                    || (c == '.' && self.s.check(|n| n.is_ascii_digit())) =>
                {
                    self.number(start, c)
                }

                // Hex values and strings.
                '#' => self.hex(),
                '"' => self.string(),

                _ => Token::Invalid(self.s.eaten_from(start)),
            },
        })
    }
}

impl<'s> Tokens<'s> {
    fn whitespace(&mut self, first: char) -> Token<'s> {
        // Fast path for just a single space
        if first == ' ' && !self.s.check(|c| c.is_whitespace()) {
            Token::Space(0)
        } else {
            self.s.uneat();

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

    fn hash(&mut self, start: usize) -> Token<'s> {
        if self.s.check(is_id_start) {
            self.s.eat();
            self.s.eat_while(is_id_continue);
            match self.s.eaten_from(start) {
                "#let" => Token::Let,
                "#if" => Token::If,
                "#else" => Token::Else,
                "#for" => Token::For,
                "#in" => Token::In,
                "#while" => Token::While,
                "#break" => Token::Break,
                "#continue" => Token::Continue,
                "#return" => Token::Return,
                s => Token::Invalid(s),
            }
        } else {
            Token::Hash
        }
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

    fn math(&mut self) -> Token<'s> {
        let mut dollars = 1;
        if self.s.eat_if('$') {
            dollars = 2;
        }

        let start = self.s.index();

        let mut found = 0;
        let mut escaped = false;
        while found < dollars {
            match self.s.eat() {
                Some('$') if !escaped => found += 1,
                Some(c) => {
                    found = 0;
                    escaped = c == '\\' && !escaped;
                }
                None => break,
            }
        }

        let terminated = found == dollars;
        let end = self.s.index() - if terminated { found } else { 0 };

        Token::Math(TokenMath {
            formula: self.s.get(start .. end),
            inline: dollars == 1,
            terminated,
        })
    }

    fn backslash(&mut self) -> Token<'s> {
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

    fn ident(&mut self, start: usize) -> Token<'s> {
        self.s.eat_while(is_id_continue);
        match self.s.eaten_from(start) {
            "not" => Token::Not,
            "and" => Token::And,
            "or" => Token::Or,
            "let" => Token::Let,
            "if" => Token::If,
            "else" => Token::Else,
            "for" => Token::For,
            "in" => Token::In,
            "while" => Token::While,
            "break" => Token::Break,
            "continue" => Token::Continue,
            "return" => Token::Return,
            "none" => Token::None,
            "true" => Token::Bool(true),
            "false" => Token::Bool(false),
            id => Token::Ident(id),
        }
    }

    fn number(&mut self, start: usize, first: char) -> Token<'s> {
        // Read the first part (integer or fractional depending on `first`).
        self.s.eat_while(|c| c.is_ascii_digit());

        // Read the fractional part if not already done and present.
        if first != '.' && self.s.eat_if('.') {
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
}

impl Debug for Tokens<'_> {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        write!(f, "Tokens({}|{})", self.s.eaten(), self.s.rest())
    }
}

#[cfg(test)]
#[allow(non_snake_case)]
mod tests {
    use super::*;
    use crate::parse::tests::check;

    use Option::None;
    use Token::{Ident, *};
    use TokenMode::{Code, Markup};

    fn Raw(text: &str, backticks: usize, terminated: bool) -> Token {
        Token::Raw(TokenRaw { text, backticks, terminated })
    }

    fn Math(formula: &str, inline: bool, terminated: bool) -> Token {
        Token::Math(TokenMath { formula, inline, terminated })
    }

    fn UnicodeEscape(sequence: &str, terminated: bool) -> Token {
        Token::UnicodeEscape(TokenUnicodeEscape { sequence, terminated })
    }

    fn Str(string: &str, terminated: bool) -> Token {
        Token::Str(TokenStr { string, terminated })
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
        ('a', Some(Code), "if", If),
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
        ('/', Some(Markup), "_", Underscore),
        ('/', Some(Markup), r"\\", Text(r"\")),
        ('/', Some(Markup), "#let", Let),
        ('/', Some(Code), "(", LeftParen),
        ('/', Some(Code), ":", Colon),
        ('/', Some(Code), "+=", PlusEq),
        ('/', Some(Code), "#123", Hex("123")),
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
    fn test_tokenize_markup_symbols() {
        // Test markup tokens.
        t!(Markup[" a1"]: "*"  => Star);
        t!(Markup: "_"         => Underscore);
        t!(Markup["a1/"]: "# " => Hash, Space(0));
        t!(Markup: "~"         => Tilde);
        t!(Markup[" "]: r"\"   => Backslash);
    }

    #[test]
    fn test_tokenize_code_symbols() {
        // Test all symbols.
        t!(Code: ","        => Comma);
        t!(Code: ";"        => Semicolon);
        t!(Code: ":"        => Colon);
        t!(Code: "|"        => Pipe);
        t!(Code: "+"        => Plus);
        t!(Code: "-"        => Hyph);
        t!(Code[" a1"]: "*" => Star);
        t!(Code[" a1"]: "/" => Slash);
        t!(Code: "="        => Eq);
        t!(Code: "=="       => EqEq);
        t!(Code: "!="       => BangEq);
        t!(Code: "<"        => Lt);
        t!(Code: "<="       => LtEq);
        t!(Code: ">"        => Gt);
        t!(Code: ">="       => GtEq);
        t!(Code: "+="       => PlusEq);
        t!(Code: "-="       => HyphEq);
        t!(Code: "*="       => StarEq);
        t!(Code: "/="       => SlashEq);
        t!(Code: "?"        => Question);
        t!(Code: ".."       => Dots);
        t!(Code: "=>"       => Arrow);

        // Test combinations.
        t!(Code: "|=>"        => Pipe, Arrow);
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
        let both = [
            ("let", Let),
            ("if", If),
            ("else", Else),
            ("for", For),
            ("in", In),
            ("while", While),
            ("break", Break),
            ("continue", Continue),
            ("return", Return),
        ];

        for &(s, t) in &both {
            t!(Code[" "]: s => t);
            t!(Markup[" "]: format!("#{}", s) => t);
            t!(Markup[" "]: format!("#{0}#{0}", s) => t, t);
            t!(Markup[" /"]: format!("# {}", s) => Hash, Space(0), Text(s));
        }

        let code = [
            ("not", Not),
            ("and", And),
            ("or", Or),
            ("none", Token::None),
            ("false", Bool(false)),
            ("true", Bool(true)),
        ];

        for &(s, t) in &code {
            t!(Code[" "]: s => t);
            t!(Markup[" /"]: s => Text(s));
        }

        // Test invalid case.
        t!(Code[" /"]: "None" => Ident("None"));
        t!(Code[" /"]: "True"   => Ident("True"));

        // Test word that contains keyword.
        t!(Markup[" "]: "#letter" => Invalid("#letter"));
        t!(Code[" /"]: "falser" => Ident("falser"));
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
        t!(Markup[" /"]: "hello-world" => Text("hello-world"));

        // Test code symbols in text.
        t!(Markup[" /"]: "a():\"b" => Text("a():\"b"));
        t!(Markup[" /"]: ";:,=|/+-" => Text(";:,=|/+-"));

        // Test text ends.
        t!(Markup[""]: "hello " => Text("hello"), Space(0));
        t!(Markup[""]: "hello~" => Text("hello"), Tilde);
    }

    #[test]
    fn test_tokenize_raw_blocks() {
        // Test basic raw block.
        t!(Markup: "`raw`"  => Raw("raw", 1, true));
        t!(Markup[""]: "`]" => Raw("]", 1, false));

        // Test special symbols in raw block.
        t!(Markup: "`[func]`"   => Raw("[func]", 1, true));
        t!(Markup[""]: r"`\`` " => Raw(r"\", 1, true), Raw(" ", 1, false));

        // Test more backticks.
        t!(Markup: "````🚀````"           => Raw("🚀", 4, true));
        t!(Markup[""]: "````👩‍🚀``noend"    => Raw("👩‍🚀``noend", 4, false));
        t!(Markup[""]: "````raw``````new" => Raw("raw", 4, true), Raw("new", 2, false));

        // Test separated closing backticks.
        t!(Markup: "```not `y`e`t```" => Raw("not `y`e`t", 3, true));
    }

    #[test]
    fn test_tokenize_math_formulas() {
        // Test basic formula.
        t!(Markup: "$x$"         => Math("x", true, true));
        t!(Markup: "$$x + y$$"   => Math("x + y", false, true));

        // Test unterminated.
        t!(Markup[""]: "$$x"     => Math("x", false, false));
        t!(Markup[""]: "$$x$\n$" => Math("x$\n$", false, false));

        // Test escape sequences.
        t!(Markup: r"$$\\\$$$"    => Math(r"\\\$", false, true));
        t!(Markup[""]: r"$$ $\\$" => Math(r" $\\$", false, false));
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
        t!(Markup: r"\#" => Text("#"));
        t!(Markup: r"\~" => Text("~"));
        t!(Markup: r"\`" => Text("`"));

        // Test unescapable symbols.
        t!(Markup[" /"]: r"\a"   => Text(r"\"), Text("a"));
        t!(Markup[" /"]: r"\u"   => Text(r"\"), Text("u"));
        t!(Markup[" /"]: r"\1"   => Text(r"\"), Text("1"));
        t!(Markup[" /"]: r"\:"   => Text(r"\"), Text(":"));
        t!(Markup[" /"]: r"\="   => Text(r"\"), Text("="));
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
        let nums = ints.iter().map(|&(k, v)| (k, v as f64)).chain(floats.iter().copied());

        let suffixes = [
            ("%", Percent as fn(f64) -> Token<'static>),
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
    }

    #[test]
    fn test_tokenize_hex() {
        // Test basic hex expressions.
        t!(Code[" /"]: "#6ae6dd" => Hex("6ae6dd"));
        t!(Code[" /"]: "#8A083c" => Hex("8A083c"));

        // Test with non-hex letters.
        t!(Code[" /"]: "#PQ" => Hex("PQ"));
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
        t!(Code: r"\"          => Invalid(r"\"));
        t!(Code: "🌓"          => Invalid("🌓"));
        t!(Code: r"\:"         => Invalid(r"\"), Colon);
        t!(Code: "meal⌚"      => Ident("meal"), Invalid("⌚"));
        t!(Code[" /"]: r"\a"   => Invalid(r"\"), Ident("a"));

        // Test invalid number suffixes.
        t!(Code[" /"]: "1foo" => Invalid("1foo"));
        t!(Code: "1p%"        => Invalid("1p"), Invalid("%"));
        t!(Code: "1%%"        => Percent(1.0), Invalid("%"));

        // Test invalid keyword.
        t!(Markup[" /"]: "#-" => Hash, Text("-"));
        t!(Markup[" "]: "#do" => Invalid("#do"))
    }
}
