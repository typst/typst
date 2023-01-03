use std::num::NonZeroUsize;
use std::sync::Arc;

use unicode_xid::UnicodeXID;
use unscanny::Scanner;

use super::resolve::{resolve_hex, resolve_raw, resolve_string};
use super::{ErrorPos, RawFields, SyntaxKind, Unit};
use crate::geom::{AbsUnit, AngleUnit};
use crate::util::{format_eco, EcoString};

/// An iterator over the tokens of a string of source code.
#[derive(Clone)]
pub struct Tokens<'s> {
    /// The underlying scanner.
    s: Scanner<'s>,
    /// The mode the scanner is in. This determines what tokens it recognizes.
    mode: TokenMode,
    /// Whether the last token has been terminated.
    terminated: bool,
    /// Offsets the indentation on the first line of the source.
    column_offset: usize,
}

/// What kind of tokens to emit.
#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum TokenMode {
    /// Text and markup.
    Markup,
    /// Math atoms, operators, etc.
    Math,
    /// Keywords, literals and operators.
    Code,
}

impl<'s> Tokens<'s> {
    /// Create a new token iterator with the given mode.
    pub fn new(text: &'s str, mode: TokenMode) -> Self {
        Self::with_prefix("", text, mode)
    }

    /// Create a new token iterator with the given mode and a prefix to offset
    /// column calculations.
    pub fn with_prefix(prefix: &str, text: &'s str, mode: TokenMode) -> Self {
        Self {
            s: Scanner::new(text),
            mode,
            terminated: true,
            column_offset: column(prefix, prefix.len(), 0),
        }
    }

    /// Get the current token mode.
    pub fn mode(&self) -> TokenMode {
        self.mode
    }

    /// Change the token mode.
    pub fn set_mode(&mut self, mode: TokenMode) {
        self.mode = mode;
    }

    /// The index in the string at which the last token ends and next token
    /// will start.
    pub fn cursor(&self) -> usize {
        self.s.cursor()
    }

    /// Jump to the given index in the string.
    pub fn jump(&mut self, index: usize) {
        self.s.jump(index);
    }

    /// The underlying scanner.
    pub fn scanner(&self) -> Scanner<'s> {
        self.s
    }

    /// Whether the last token was terminated.
    pub fn terminated(&self) -> bool {
        self.terminated
    }

    /// The column index of a given index in the source string.
    pub fn column(&self, index: usize) -> usize {
        column(self.s.string(), index, self.column_offset)
    }
}

impl Iterator for Tokens<'_> {
    type Item = SyntaxKind;

    /// Parse the next token in the source code.
    fn next(&mut self) -> Option<Self::Item> {
        let start = self.s.cursor();
        let c = self.s.eat()?;
        Some(match c {
            // Trivia.
            '/' if self.s.eat_if('/') => self.line_comment(),
            '/' if self.s.eat_if('*') => self.block_comment(),
            '*' if self.s.eat_if('/') => SyntaxKind::Error(
                ErrorPos::Full,
                "unexpected end of block comment".into(),
            ),
            c if c.is_whitespace() => self.whitespace(c),

            // Other things.
            _ => match self.mode {
                TokenMode::Markup => self.markup(start, c),
                TokenMode::Math => self.math(start, c),
                TokenMode::Code => self.code(start, c),
            },
        })
    }
}

/// Shared.
impl Tokens<'_> {
    fn line_comment(&mut self) -> SyntaxKind {
        self.s.eat_until(is_newline);
        if self.s.peek().is_none() {
            self.terminated = false;
        }
        SyntaxKind::LineComment
    }

    fn block_comment(&mut self) -> SyntaxKind {
        let mut state = '_';
        let mut depth = 1;
        self.terminated = false;

        // Find the first `*/` that does not correspond to a nested `/*`.
        while let Some(c) = self.s.eat() {
            state = match (state, c) {
                ('*', '/') => {
                    depth -= 1;
                    if depth == 0 {
                        self.terminated = true;
                        break;
                    }
                    '_'
                }
                ('/', '*') => {
                    depth += 1;
                    '_'
                }
                ('/', '/') => {
                    self.line_comment();
                    '_'
                }
                _ => c,
            }
        }

        SyntaxKind::BlockComment
    }

    fn whitespace(&mut self, c: char) -> SyntaxKind {
        if c == ' ' && !self.s.at(char::is_whitespace) {
            return SyntaxKind::Space { newlines: 0 };
        }

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

        SyntaxKind::Space { newlines }
    }
}

impl Tokens<'_> {
    fn markup(&mut self, start: usize, c: char) -> SyntaxKind {
        match c {
            // Blocks.
            '{' => SyntaxKind::LeftBrace,
            '}' => SyntaxKind::RightBrace,
            '[' => SyntaxKind::LeftBracket,
            ']' => SyntaxKind::RightBracket,

            // Multi-char things.
            '#' => self.hash(start),
            '.' if self.s.eat_if("..") => SyntaxKind::Shorthand('\u{2026}'),
            '-' => self.hyph(),
            ':' => self.colon(),
            'h' if self.s.eat_if("ttp://") || self.s.eat_if("ttps://") => {
                self.link(start)
            }
            '`' => self.raw(),
            c if c.is_ascii_digit() => self.numbering(start),
            '<' if self.s.at(is_id_continue) => self.label(),
            '@' if self.s.at(is_id_continue) => self.reference(),

            // Escape sequences.
            '\\' => self.backslash(),

            // Single-char things.
            '~' => SyntaxKind::Shorthand('\u{00A0}'),
            '\'' => SyntaxKind::SmartQuote { double: false },
            '"' => SyntaxKind::SmartQuote { double: true },
            '*' if !self.in_word() => SyntaxKind::Star,
            '_' if !self.in_word() => SyntaxKind::Underscore,
            '$' => SyntaxKind::Dollar,
            '=' => SyntaxKind::Eq,
            '+' => SyntaxKind::Plus,
            '/' => SyntaxKind::Slash,

            // Plain text.
            _ => self.text(start),
        }
    }

    fn text(&mut self, start: usize) -> SyntaxKind {
        macro_rules! table {
            ($(|$c:literal)*) => {{
                let mut t = [false; 128];
                $(t[$c as usize] = true;)*
                t
            }}
        }

        const TABLE: [bool; 128] = table! {
            | ' ' | '\t' | '\n' | '\x0b' | '\x0c' | '\r' | '\\' | '/'
            | '[' | ']' | '{' | '}' | '~' | '-' | '.' | '\'' | '"'
            | '*' | '_' | ':' | 'h' | '`' | '$' | '<' | '>' | '@' | '#'
        };

        loop {
            self.s.eat_until(|c: char| {
                TABLE.get(c as usize).copied().unwrap_or_else(|| c.is_whitespace())
            });

            // Continue with the same text node if the thing would become text
            // anyway.
            let mut s = self.s;
            match s.eat() {
                Some('/') if !s.at(['/', '*']) => {}
                Some(' ') if s.at(char::is_alphanumeric) => {}
                Some('-') if !s.at(['-', '?']) => {}
                Some('.') if !s.at("..") => {}
                Some('h') if !s.at("ttp://") && !s.at("ttps://") => {}
                Some('@' | '#') if !s.at(is_id_start) => {}
                _ => break,
            }

            self.s = s;
        }

        SyntaxKind::Text(self.s.from(start).into())
    }

    fn backslash(&mut self) -> SyntaxKind {
        match self.s.peek() {
            Some('u') if self.s.eat_if("u{") => {
                let sequence = self.s.eat_while(char::is_ascii_alphanumeric);
                if self.s.eat_if('}') {
                    if let Some(c) = resolve_hex(sequence) {
                        SyntaxKind::Escape(c)
                    } else {
                        SyntaxKind::Error(
                            ErrorPos::Full,
                            "invalid unicode escape sequence".into(),
                        )
                    }
                } else {
                    self.terminated = false;
                    SyntaxKind::Error(ErrorPos::End, "expected closing brace".into())
                }
            }

            // Linebreaks.
            Some(c) if c.is_whitespace() => SyntaxKind::Linebreak,
            None => SyntaxKind::Linebreak,

            // Escapes.
            Some(c) => {
                self.s.expect(c);
                SyntaxKind::Escape(c)
            }
        }
    }

    fn hash(&mut self, start: usize) -> SyntaxKind {
        if self.s.eat_if('{') {
            SyntaxKind::LeftBrace
        } else if self.s.eat_if('[') {
            SyntaxKind::LeftBracket
        } else if self.s.at(is_id_start) {
            let read = self.s.eat_while(is_id_continue);
            match keyword(read) {
                Some(keyword) => keyword,
                None => SyntaxKind::Ident(read.into()),
            }
        } else if self.mode == TokenMode::Markup {
            self.text(start)
        } else {
            SyntaxKind::Atom("#".into())
        }
    }

    fn hyph(&mut self) -> SyntaxKind {
        if self.s.eat_if('-') {
            if self.s.eat_if('-') {
                SyntaxKind::Shorthand('\u{2014}')
            } else {
                SyntaxKind::Shorthand('\u{2013}')
            }
        } else if self.s.eat_if('?') {
            SyntaxKind::Shorthand('\u{00AD}')
        } else {
            SyntaxKind::Minus
        }
    }

    fn colon(&mut self) -> SyntaxKind {
        let start = self.s.cursor();
        let mut end = start;
        while !self.s.eat_while(char::is_ascii_alphanumeric).is_empty() && self.s.at(':')
        {
            end = self.s.cursor();
            self.s.eat();
        }

        self.s.jump(end);

        if start < end {
            self.s.expect(':');
            SyntaxKind::Symbol(self.s.get(start..end).into())
        } else if self.mode == TokenMode::Markup {
            SyntaxKind::Colon
        } else {
            SyntaxKind::Atom(":".into())
        }
    }

    fn link(&mut self, start: usize) -> SyntaxKind {
        #[rustfmt::skip]
        self.s.eat_while(|c: char| matches!(c,
            | '0' ..= '9'
            | 'a' ..= 'z'
            | 'A' ..= 'Z'
            | '~'  | '/' | '%' | '?' | '#' | '&' | '+' | '='
            | '\'' | '.' | ',' | ';'
        ));
        if self.s.scout(-1) == Some('.') {
            self.s.uneat();
        }
        SyntaxKind::Link(self.s.from(start).into())
    }

    fn raw(&mut self) -> SyntaxKind {
        let column = self.column(self.s.cursor() - 1);

        let mut backticks = 1;
        while self.s.eat_if('`') {
            backticks += 1;
        }

        // Special case for empty inline block.
        if backticks == 2 {
            return SyntaxKind::Raw(Arc::new(RawFields {
                text: EcoString::new(),
                lang: None,
                block: false,
            }));
        }

        let start = self.s.cursor();
        let mut found = 0;
        while found < backticks {
            match self.s.eat() {
                Some('`') => found += 1,
                Some(_) => found = 0,
                None => break,
            }
        }

        if found == backticks {
            let end = self.s.cursor() - found as usize;
            SyntaxKind::Raw(Arc::new(resolve_raw(
                column,
                backticks,
                self.s.get(start..end),
            )))
        } else {
            self.terminated = false;
            let remaining = backticks - found;
            let noun = if remaining == 1 { "backtick" } else { "backticks" };
            SyntaxKind::Error(
                ErrorPos::End,
                if found == 0 {
                    format_eco!("expected {} {}", remaining, noun)
                } else {
                    format_eco!("expected {} more {}", remaining, noun)
                },
            )
        }
    }

    fn numbering(&mut self, start: usize) -> SyntaxKind {
        self.s.eat_while(char::is_ascii_digit);
        let read = self.s.from(start);
        if self.s.eat_if('.') {
            if let Ok(number) = read.parse::<usize>() {
                return match NonZeroUsize::new(number) {
                    Some(number) => SyntaxKind::EnumNumbering(number),
                    None => SyntaxKind::Error(ErrorPos::Full, "must be positive".into()),
                };
            }
        }

        self.text(start)
    }

    fn reference(&mut self) -> SyntaxKind {
        SyntaxKind::Ref(self.s.eat_while(is_id_continue).into())
    }

    fn in_word(&self) -> bool {
        let alphanumeric = |c: Option<char>| c.map_or(false, |c| c.is_alphanumeric());
        let prev = self.s.scout(-2);
        let next = self.s.peek();
        alphanumeric(prev) && alphanumeric(next)
    }
}

/// Math.
impl Tokens<'_> {
    fn math(&mut self, start: usize, c: char) -> SyntaxKind {
        match c {
            // Symbol shorthands.
            '|' if self.s.eat_if("->") => SyntaxKind::Shorthand('\u{21A6}'),
            '<' if self.s.eat_if("->") => SyntaxKind::Shorthand('\u{2194}'),
            '<' if self.s.eat_if("=>") => SyntaxKind::Shorthand('\u{21D4}'),
            '!' if self.s.eat_if('=') => SyntaxKind::Shorthand('\u{2260}'),
            '<' if self.s.eat_if('=') => SyntaxKind::Shorthand('\u{2264}'),
            '>' if self.s.eat_if('=') => SyntaxKind::Shorthand('\u{2265}'),
            '<' if self.s.eat_if('-') => SyntaxKind::Shorthand('\u{2190}'),
            '-' if self.s.eat_if('>') => SyntaxKind::Shorthand('\u{2192}'),
            '=' if self.s.eat_if('>') => SyntaxKind::Shorthand('\u{21D2}'),
            ':' if self.s.eat_if('=') => SyntaxKind::Shorthand('\u{2254}'),

            // Multi-char things.
            '#' => self.hash(start),

            // Escape sequences.
            '\\' => self.backslash(),

            // Single-char things.
            '_' => SyntaxKind::Underscore,
            '^' => SyntaxKind::Hat,
            '/' => SyntaxKind::Slash,
            '&' => SyntaxKind::Amp,
            '$' => SyntaxKind::Dollar,

            // Symbol notation.
            ':' => self.colon(),

            // Strings.
            '"' => self.string(),

            // Identifiers and symbol notation.
            c if is_math_id_start(c) && self.s.at(is_math_id_continue) => {
                self.s.eat_while(is_math_id_continue);

                let mut symbol = false;
                while self.s.eat_if(':')
                    && !self.s.eat_while(char::is_alphanumeric).is_empty()
                {
                    symbol = true;
                }

                if symbol {
                    SyntaxKind::Symbol(self.s.from(start).into())
                } else {
                    if self.s.scout(-1) == Some(':') {
                        self.s.uneat();
                    }

                    SyntaxKind::Ident(self.s.from(start).into())
                }
            }

            // Numbers.
            c if c.is_numeric() => {
                self.s.eat_while(char::is_numeric);
                SyntaxKind::Atom(self.s.from(start).into())
            }

            // Other math atoms.
            c => SyntaxKind::Atom(c.into()),
        }
    }
}

/// Code.
impl Tokens<'_> {
    fn code(&mut self, start: usize, c: char) -> SyntaxKind {
        match c {
            // Blocks.
            '{' => SyntaxKind::LeftBrace,
            '}' => SyntaxKind::RightBrace,
            '[' => SyntaxKind::LeftBracket,
            ']' => SyntaxKind::RightBracket,

            // Parentheses.
            '(' => SyntaxKind::LeftParen,
            ')' => SyntaxKind::RightParen,

            // Math.
            '$' => SyntaxKind::Dollar,

            // Labels and raw.
            '<' if self.s.at(is_id_continue) => self.label(),
            '`' => self.raw(),

            // Two-char operators.
            '=' if self.s.eat_if('=') => SyntaxKind::EqEq,
            '!' if self.s.eat_if('=') => SyntaxKind::ExclEq,
            '<' if self.s.eat_if('=') => SyntaxKind::LtEq,
            '>' if self.s.eat_if('=') => SyntaxKind::GtEq,
            '+' if self.s.eat_if('=') => SyntaxKind::PlusEq,
            '-' if self.s.eat_if('=') => SyntaxKind::HyphEq,
            '*' if self.s.eat_if('=') => SyntaxKind::StarEq,
            '/' if self.s.eat_if('=') => SyntaxKind::SlashEq,
            '.' if self.s.eat_if('.') => SyntaxKind::Dots,
            '=' if self.s.eat_if('>') => SyntaxKind::Arrow,

            // Single-char operators.
            ',' => SyntaxKind::Comma,
            ';' => SyntaxKind::Semicolon,
            ':' => SyntaxKind::Colon,
            '+' => SyntaxKind::Plus,
            '-' => SyntaxKind::Minus,
            '*' => SyntaxKind::Star,
            '/' => SyntaxKind::Slash,
            '=' => SyntaxKind::Eq,
            '<' => SyntaxKind::Lt,
            '>' => SyntaxKind::Gt,
            '.' if !self.s.at(char::is_ascii_digit) => SyntaxKind::Dot,

            // Identifiers.
            c if is_id_start(c) => self.ident(start),

            // Numbers.
            c if c.is_ascii_digit() || (c == '.' && self.s.at(char::is_ascii_digit)) => {
                self.number(start, c)
            }

            // Strings.
            '"' => self.string(),

            // Invalid token.
            _ => SyntaxKind::Error(ErrorPos::Full, "not valid here".into()),
        }
    }

    fn ident(&mut self, start: usize) -> SyntaxKind {
        self.s.eat_while(is_id_continue);
        match self.s.from(start) {
            "none" => SyntaxKind::None,
            "auto" => SyntaxKind::Auto,
            "true" => SyntaxKind::Bool(true),
            "false" => SyntaxKind::Bool(false),
            id => keyword(id).unwrap_or_else(|| SyntaxKind::Ident(id.into())),
        }
    }

    fn number(&mut self, start: usize, c: char) -> SyntaxKind {
        // Read the first part (integer or fractional depending on `first`).
        self.s.eat_while(char::is_ascii_digit);

        // Read the fractional part if not already done.
        // Make sure not to confuse a range for the decimal separator.
        if c != '.' && !self.s.at("..") && self.s.eat_if('.') {
            self.s.eat_while(char::is_ascii_digit);
        }

        // Read the exponent.
        if !self.s.at("em") && self.s.eat_if(['e', 'E']) {
            self.s.eat_if(['+', '-']);
            self.s.eat_while(char::is_ascii_digit);
        }

        // Read the suffix.
        let suffix_start = self.s.cursor();
        if !self.s.eat_if('%') {
            self.s.eat_while(char::is_ascii_alphanumeric);
        }

        let number = self.s.get(start..suffix_start);
        let suffix = self.s.from(suffix_start);

        // Find out whether it is a simple number.
        if suffix.is_empty() {
            if let Ok(i) = number.parse::<i64>() {
                return SyntaxKind::Int(i);
            }
        }

        let Ok(v) = number.parse::<f64>() else {
            return SyntaxKind::Error(ErrorPos::Full, "invalid number".into());
        };

        match suffix {
            "" => SyntaxKind::Float(v),
            "pt" => SyntaxKind::Numeric(v, Unit::Length(AbsUnit::Pt)),
            "mm" => SyntaxKind::Numeric(v, Unit::Length(AbsUnit::Mm)),
            "cm" => SyntaxKind::Numeric(v, Unit::Length(AbsUnit::Cm)),
            "in" => SyntaxKind::Numeric(v, Unit::Length(AbsUnit::In)),
            "deg" => SyntaxKind::Numeric(v, Unit::Angle(AngleUnit::Deg)),
            "rad" => SyntaxKind::Numeric(v, Unit::Angle(AngleUnit::Rad)),
            "em" => SyntaxKind::Numeric(v, Unit::Em),
            "fr" => SyntaxKind::Numeric(v, Unit::Fr),
            "%" => SyntaxKind::Numeric(v, Unit::Percent),
            _ => SyntaxKind::Error(ErrorPos::Full, "invalid number suffix".into()),
        }
    }

    fn string(&mut self) -> SyntaxKind {
        let mut escaped = false;
        let verbatim = self.s.eat_until(|c| {
            if c == '"' && !escaped {
                true
            } else {
                escaped = c == '\\' && !escaped;
                false
            }
        });

        let string = resolve_string(verbatim);
        if self.s.eat_if('"') {
            SyntaxKind::Str(string)
        } else {
            self.terminated = false;
            SyntaxKind::Error(ErrorPos::End, "expected quote".into())
        }
    }

    fn label(&mut self) -> SyntaxKind {
        let label = self.s.eat_while(is_id_continue);
        if self.s.eat_if('>') {
            if !label.is_empty() {
                SyntaxKind::Label(label.into())
            } else {
                SyntaxKind::Error(ErrorPos::Full, "label cannot be empty".into())
            }
        } else {
            self.terminated = false;
            SyntaxKind::Error(ErrorPos::End, "expected closing angle bracket".into())
        }
    }
}

/// Try to parse an identifier into a keyword.
fn keyword(ident: &str) -> Option<SyntaxKind> {
    Some(match ident {
        "not" => SyntaxKind::Not,
        "and" => SyntaxKind::And,
        "or" => SyntaxKind::Or,
        "let" => SyntaxKind::Let,
        "set" => SyntaxKind::Set,
        "show" => SyntaxKind::Show,
        "if" => SyntaxKind::If,
        "else" => SyntaxKind::Else,
        "for" => SyntaxKind::For,
        "in" => SyntaxKind::In,
        "while" => SyntaxKind::While,
        "break" => SyntaxKind::Break,
        "continue" => SyntaxKind::Continue,
        "return" => SyntaxKind::Return,
        "import" => SyntaxKind::Import,
        "include" => SyntaxKind::Include,
        "as" => SyntaxKind::As,
        _ => return None,
    })
}

/// The column index of a given index in the source string, given a column
/// offset for the first line.
fn column(string: &str, index: usize, offset: usize) -> usize {
    let mut apply_offset = false;
    let res = string[..index]
        .char_indices()
        .rev()
        .take_while(|&(_, c)| !is_newline(c))
        .inspect(|&(i, _)| {
            if i == 0 {
                apply_offset = true
            }
        })
        .count();

    // The loop is never executed if the slice is empty, but we are of
    // course still at the start of the first line.
    if index == 0 {
        apply_offset = true;
    }

    if apply_offset {
        res + offset
    } else {
        res
    }
}

/// Whether this character denotes a newline.
#[inline]
pub fn is_newline(character: char) -> bool {
    matches!(
        character,
        // Line Feed, Vertical Tab, Form Feed, Carriage Return.
        '\n' | '\x0B' | '\x0C' | '\r' |
        // Next Line, Line Separator, Paragraph Separator.
        '\u{0085}' | '\u{2028}' | '\u{2029}'
    )
}

/// Whether a string is a valid unicode identifier.
///
/// In addition to what is specified in the [Unicode Standard][uax31], we allow:
/// - `_` as a starting character,
/// - `_` and `-` as continuing characters.
///
/// [uax31]: http://www.unicode.org/reports/tr31/
#[inline]
pub fn is_ident(string: &str) -> bool {
    let mut chars = string.chars();
    chars
        .next()
        .map_or(false, |c| is_id_start(c) && chars.all(is_id_continue))
}

/// Whether a character can start an identifier.
#[inline]
fn is_id_start(c: char) -> bool {
    c.is_xid_start() || c == '_'
}

/// Whether a character can continue an identifier.
#[inline]
fn is_id_continue(c: char) -> bool {
    c.is_xid_continue() || c == '_' || c == '-'
}

/// Whether a character can start an identifier in math.
#[inline]
fn is_math_id_start(c: char) -> bool {
    c.is_xid_start()
}

/// Whether a character can continue an identifier in math.
#[inline]
fn is_math_id_continue(c: char) -> bool {
    c.is_xid_continue() && c != '_'
}
