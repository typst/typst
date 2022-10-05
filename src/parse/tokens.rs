use std::sync::Arc;

use unicode_xid::UnicodeXID;
use unscanny::Scanner;

use super::resolve::{resolve_hex, resolve_raw, resolve_string};
use crate::diag::ErrorPos;
use crate::geom::{AngleUnit, LengthUnit};
use crate::syntax::ast::{RawNode, Unit};
use crate::syntax::NodeKind;
use crate::util::EcoString;

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
    #[inline]
    pub fn new(text: &'s str, mode: TokenMode) -> Self {
        Self::with_prefix("", text, mode)
    }

    /// Create a new token iterator with the given mode and a prefix to offset
    /// column calculations.
    #[inline]
    pub fn with_prefix(prefix: &str, text: &'s str, mode: TokenMode) -> Self {
        Self {
            s: Scanner::new(text),
            mode,
            terminated: true,
            column_offset: column(prefix, prefix.len(), 0),
        }
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
    pub fn cursor(&self) -> usize {
        self.s.cursor()
    }

    /// Jump to the given index in the string.
    #[inline]
    pub fn jump(&mut self, index: usize) {
        self.s.jump(index);
    }

    /// The underlying scanner.
    #[inline]
    pub fn scanner(&self) -> Scanner<'s> {
        self.s
    }

    /// Whether the last token was terminated.
    #[inline]
    pub fn terminated(&self) -> bool {
        self.terminated
    }

    /// The column index of a given index in the source string.
    #[inline]
    pub fn column(&self, index: usize) -> usize {
        column(self.s.string(), index, self.column_offset)
    }
}

impl<'s> Iterator for Tokens<'s> {
    type Item = NodeKind;

    /// Parse the next token in the source code.
    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        let start = self.s.cursor();
        let c = self.s.eat()?;
        Some(match c {
            // Trivia.
            '/' if self.s.eat_if('/') => self.line_comment(),
            '/' if self.s.eat_if('*') => self.block_comment(),
            '*' if self.s.eat_if('/') => {
                NodeKind::Error(ErrorPos::Full, "unexpected end of block comment".into())
            }
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

impl<'s> Tokens<'s> {
    fn line_comment(&mut self) -> NodeKind {
        self.s.eat_until(is_newline);
        if self.s.peek().is_none() {
            self.terminated = false;
        }
        NodeKind::LineComment
    }

    fn block_comment(&mut self) -> NodeKind {
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

        NodeKind::BlockComment
    }

    fn whitespace(&mut self, c: char) -> NodeKind {
        if c == ' ' && !self.s.at(char::is_whitespace) {
            return NodeKind::Space { newlines: 0 };
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

        NodeKind::Space { newlines }
    }

    #[inline]
    fn markup(&mut self, start: usize, c: char) -> NodeKind {
        match c {
            // Blocks.
            '{' => NodeKind::LeftBrace,
            '}' => NodeKind::RightBrace,
            '[' => NodeKind::LeftBracket,
            ']' => NodeKind::RightBracket,

            // Escape sequences.
            '\\' => self.backslash(),

            // Single-char things.
            '~' => NodeKind::Tilde,
            '.' if self.s.eat_if("..") => NodeKind::Dot3,
            '\'' => NodeKind::Quote { double: false },
            '"' => NodeKind::Quote { double: true },
            '*' if !self.in_word() => NodeKind::Star,
            '_' if !self.in_word() => NodeKind::Underscore,
            '$' => NodeKind::Dollar,
            '=' => NodeKind::Eq,
            '+' => NodeKind::Plus,
            '/' => NodeKind::Slash,
            ':' => NodeKind::Colon,

            // Multi-char things.
            '#' => self.hash(start),
            '-' => self.hyph(),
            'h' if self.s.eat_if("ttp://") || self.s.eat_if("ttps://") => {
                self.link(start)
            }
            '`' => self.raw(),
            c if c.is_ascii_digit() => self.numbering(start),
            '<' => self.label(),
            '@' => self.reference(start),

            // Plain text.
            _ => self.text(start),
        }
    }

    #[inline]
    fn text(&mut self, start: usize) -> NodeKind {
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

        NodeKind::Text(self.s.from(start).into())
    }

    fn backslash(&mut self) -> NodeKind {
        match self.s.peek() {
            Some('u') if self.s.eat_if("u{") => {
                let sequence = self.s.eat_while(char::is_ascii_alphanumeric);
                if self.s.eat_if('}') {
                    if let Some(c) = resolve_hex(sequence) {
                        NodeKind::Escape(c)
                    } else {
                        NodeKind::Error(
                            ErrorPos::Full,
                            "invalid unicode escape sequence".into(),
                        )
                    }
                } else {
                    self.terminated = false;
                    NodeKind::Error(ErrorPos::End, "expected closing brace".into())
                }
            }

            // Linebreaks.
            Some(c) if c.is_whitespace() => NodeKind::Backslash,
            None => NodeKind::Backslash,

            // Escapes.
            Some(c) => {
                self.s.expect(c);
                NodeKind::Escape(c)
            }
        }
    }

    fn hash(&mut self, start: usize) -> NodeKind {
        if self.s.at(is_id_start) {
            let read = self.s.eat_while(is_id_continue);
            match keyword(read) {
                Some(keyword) => keyword,
                None => NodeKind::Ident(read.into()),
            }
        } else {
            self.text(start)
        }
    }

    fn hyph(&mut self) -> NodeKind {
        if self.s.eat_if('-') {
            if self.s.eat_if('-') {
                NodeKind::Hyph3
            } else {
                NodeKind::Hyph2
            }
        } else if self.s.eat_if('?') {
            NodeKind::HyphQuest
        } else {
            NodeKind::Minus
        }
    }

    fn in_word(&self) -> bool {
        let alphanumeric = |c: Option<char>| c.map_or(false, |c| c.is_alphanumeric());
        let prev = self.s.scout(-2);
        let next = self.s.peek();
        alphanumeric(prev) && alphanumeric(next)
    }

    fn link(&mut self, start: usize) -> NodeKind {
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
        NodeKind::Link(self.s.from(start).into())
    }

    fn raw(&mut self) -> NodeKind {
        let column = self.column(self.s.cursor() - 1);

        let mut backticks = 1;
        while self.s.eat_if('`') {
            backticks += 1;
        }

        // Special case for empty inline block.
        if backticks == 2 {
            return NodeKind::Raw(Arc::new(RawNode {
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
            NodeKind::Raw(Arc::new(resolve_raw(
                column,
                backticks,
                self.s.get(start .. end),
            )))
        } else {
            self.terminated = false;
            let remaining = backticks - found;
            let noun = if remaining == 1 { "backtick" } else { "backticks" };
            NodeKind::Error(
                ErrorPos::End,
                if found == 0 {
                    format_eco!("expected {} {}", remaining, noun)
                } else {
                    format_eco!("expected {} more {}", remaining, noun)
                },
            )
        }
    }

    fn numbering(&mut self, start: usize) -> NodeKind {
        self.s.eat_while(char::is_ascii_digit);
        let read = self.s.from(start);
        if self.s.eat_if('.') {
            if let Ok(number) = read.parse() {
                return NodeKind::EnumNumbering(number);
            }
        }

        self.text(start)
    }

    fn label(&mut self) -> NodeKind {
        let label = self.s.eat_while(is_id_continue);
        if self.s.eat_if('>') {
            if !label.is_empty() {
                NodeKind::Label(label.into())
            } else {
                NodeKind::Error(ErrorPos::Full, "label cannot be empty".into())
            }
        } else {
            self.terminated = false;
            NodeKind::Error(ErrorPos::End, "expected closing angle bracket".into())
        }
    }

    fn reference(&mut self, start: usize) -> NodeKind {
        let label = self.s.eat_while(is_id_continue);
        if !label.is_empty() {
            NodeKind::Ref(label.into())
        } else {
            self.text(start)
        }
    }

    fn math(&mut self, start: usize, c: char) -> NodeKind {
        match c {
            // Escape sequences.
            '\\' => self.backslash(),

            // Single-char things.
            '_' => NodeKind::Underscore,
            '^' => NodeKind::Hat,
            '/' => NodeKind::Slash,
            '&' => NodeKind::Amp,
            '$' => NodeKind::Dollar,

            // Brackets.
            '{' => NodeKind::LeftBrace,
            '}' => NodeKind::RightBrace,
            '[' => NodeKind::LeftBracket,
            ']' => NodeKind::RightBracket,
            '(' => NodeKind::LeftParen,
            ')' => NodeKind::RightParen,

            // Identifiers.
            c if is_math_id_start(c) && self.s.at(is_math_id_continue) => {
                self.s.eat_while(is_math_id_continue);
                NodeKind::Ident(self.s.from(start).into())
            }

            // Numbers.
            c if c.is_numeric() => {
                self.s.eat_while(char::is_numeric);
                NodeKind::Atom(self.s.from(start).into())
            }

            // Other math atoms.
            c => NodeKind::Atom(c.into()),
        }
    }

    fn code(&mut self, start: usize, c: char) -> NodeKind {
        match c {
            // Blocks.
            '{' => NodeKind::LeftBrace,
            '}' => NodeKind::RightBrace,
            '[' => NodeKind::LeftBracket,
            ']' => NodeKind::RightBracket,

            // Parentheses.
            '(' => NodeKind::LeftParen,
            ')' => NodeKind::RightParen,

            // Two-char operators.
            '=' if self.s.eat_if('=') => NodeKind::EqEq,
            '!' if self.s.eat_if('=') => NodeKind::ExclEq,
            '<' if self.s.eat_if('=') => NodeKind::LtEq,
            '>' if self.s.eat_if('=') => NodeKind::GtEq,
            '+' if self.s.eat_if('=') => NodeKind::PlusEq,
            '-' if self.s.eat_if('=') => NodeKind::HyphEq,
            '*' if self.s.eat_if('=') => NodeKind::StarEq,
            '/' if self.s.eat_if('=') => NodeKind::SlashEq,
            '.' if self.s.eat_if('.') => NodeKind::Dots,
            '=' if self.s.eat_if('>') => NodeKind::Arrow,

            // Single-char operators.
            ',' => NodeKind::Comma,
            ';' => NodeKind::Semicolon,
            ':' => NodeKind::Colon,
            '+' => NodeKind::Plus,
            '-' => NodeKind::Minus,
            '*' => NodeKind::Star,
            '/' => NodeKind::Slash,
            '=' => NodeKind::Eq,
            '<' => NodeKind::Lt,
            '>' => NodeKind::Gt,
            '.' if !self.s.at(char::is_ascii_digit) => NodeKind::Dot,

            // Identifiers.
            c if is_id_start(c) => self.ident(start),

            // Numbers.
            c if c.is_ascii_digit() || (c == '.' && self.s.at(char::is_ascii_digit)) => {
                self.number(start, c)
            }

            // Strings.
            '"' => self.string(),

            // Invalid token.
            _ => NodeKind::Error(ErrorPos::Full, "not valid here".into()),
        }
    }

    fn ident(&mut self, start: usize) -> NodeKind {
        self.s.eat_while(is_id_continue);
        match self.s.from(start) {
            "none" => NodeKind::None,
            "auto" => NodeKind::Auto,
            "true" => NodeKind::Bool(true),
            "false" => NodeKind::Bool(false),
            id => keyword(id).unwrap_or_else(|| NodeKind::Ident(id.into())),
        }
    }

    fn number(&mut self, start: usize, c: char) -> NodeKind {
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

        let number = self.s.get(start .. suffix_start);
        let suffix = self.s.from(suffix_start);

        // Find out whether it is a simple number.
        if suffix.is_empty() {
            if let Ok(i) = number.parse::<i64>() {
                return NodeKind::Int(i);
            }
        }

        if let Ok(f) = number.parse::<f64>() {
            match suffix {
                "" => NodeKind::Float(f),
                "pt" => NodeKind::Numeric(f, Unit::Length(LengthUnit::Pt)),
                "mm" => NodeKind::Numeric(f, Unit::Length(LengthUnit::Mm)),
                "cm" => NodeKind::Numeric(f, Unit::Length(LengthUnit::Cm)),
                "in" => NodeKind::Numeric(f, Unit::Length(LengthUnit::In)),
                "deg" => NodeKind::Numeric(f, Unit::Angle(AngleUnit::Deg)),
                "rad" => NodeKind::Numeric(f, Unit::Angle(AngleUnit::Rad)),
                "em" => NodeKind::Numeric(f, Unit::Em),
                "fr" => NodeKind::Numeric(f, Unit::Fr),
                "%" => NodeKind::Numeric(f, Unit::Percent),
                _ => NodeKind::Error(ErrorPos::Full, "invalid number suffix".into()),
            }
        } else {
            NodeKind::Error(ErrorPos::Full, "invalid number".into())
        }
    }

    fn string(&mut self) -> NodeKind {
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
            NodeKind::Str(string)
        } else {
            self.terminated = false;
            NodeKind::Error(ErrorPos::End, "expected quote".into())
        }
    }
}

fn keyword(ident: &str) -> Option<NodeKind> {
    Some(match ident {
        "not" => NodeKind::Not,
        "and" => NodeKind::And,
        "or" => NodeKind::Or,
        "let" => NodeKind::Let,
        "set" => NodeKind::Set,
        "show" => NodeKind::Show,
        "wrap" => NodeKind::Wrap,
        "if" => NodeKind::If,
        "else" => NodeKind::Else,
        "for" => NodeKind::For,
        "in" => NodeKind::In,
        "as" => NodeKind::As,
        "while" => NodeKind::While,
        "break" => NodeKind::Break,
        "continue" => NodeKind::Continue,
        "return" => NodeKind::Return,
        "import" => NodeKind::Import,
        "include" => NodeKind::Include,
        "from" => NodeKind::From,
        _ => return None,
    })
}

/// The column index of a given index in the source string, given a column
/// offset for the first line.
#[inline]
fn column(string: &str, index: usize, offset: usize) -> usize {
    let mut apply_offset = false;
    let res = string[.. index]
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

    if apply_offset { res + offset } else { res }
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

#[cfg(test)]
#[allow(non_snake_case)]
mod tests {
    use super::*;
    use crate::parse::tests::check;

    use ErrorPos::*;
    use NodeKind::*;
    use Option::None;
    use TokenMode::{Code, Markup};

    fn Space(newlines: usize) -> NodeKind {
        NodeKind::Space { newlines }
    }

    fn Raw(text: &str, lang: Option<&str>, block: bool) -> NodeKind {
        NodeKind::Raw(Arc::new(RawNode {
            text: text.into(),
            lang: lang.map(Into::into),
            block,
        }))
    }

    fn Str(string: &str) -> NodeKind {
        NodeKind::Str(string.into())
    }

    fn Text(string: &str) -> NodeKind {
        NodeKind::Text(string.into())
    }

    fn Ident(ident: &str) -> NodeKind {
        NodeKind::Ident(ident.into())
    }

    fn Error(pos: ErrorPos, message: &str) -> NodeKind {
        NodeKind::Error(pos, message.into())
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

    macro_rules! t {
        (Both $($tts:tt)*) => {
            t!(Markup $($tts)*);
            t!(Code $($tts)*);
        };
        ($mode:ident $([$blocks:literal])?: $text:expr => $($token:expr),*) => {{
            // Test without suffix.
            t!(@$mode: $text => $($token),*);

            // Suffixes described by four-tuples of:
            //
            // - block the suffix is part of
            // - mode in which the suffix is applicable
            // - the suffix string
            // - the resulting suffix NodeKind
            let suffixes: &[(char, Option<TokenMode>, &str, NodeKind)] = &[
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
                ('/', None, "//", LineComment),
                ('/', None, "/**/", BlockComment),
                ('/', Some(Markup), "*", Star),
                ('/', Some(Markup), r"\\", Escape('\\')),
                ('/', Some(Markup), "#let", Let),
                ('/', Some(Code), "(", LeftParen),
                ('/', Some(Code), ":", Colon),
                ('/', Some(Code), "+=", PlusEq),
            ];

            // Test with each applicable suffix.
            for &(block, mode, suffix, ref token) in suffixes {
                let text = $text;
                #[allow(unused_variables)]
                let blocks = BLOCKS;
                $(let blocks = $blocks;)?
                assert!(!blocks.contains(|c| !BLOCKS.contains(c)));
                if (mode.is_none() || mode == Some($mode)) && blocks.contains(block) {
                    t!(@$mode: format!("{}{}", text, suffix) => $($token,)* token);
                }
            }
        }};
        (@$mode:ident: $text:expr => $($token:expr),*) => {{
            let text = $text;
            let found = Tokens::new(&text, $mode).collect::<Vec<_>>();
            let expected = vec![$($token.clone()),*];
            check(&text, found, expected);
        }};
    }

    #[test]
    fn test_tokenize_brackets() {
        // Test in markup.
        t!(Markup: "{"       => LeftBrace);
        t!(Markup: "}"       => RightBrace);
        t!(Markup: "["       => LeftBracket);
        t!(Markup: "]"       => RightBracket);
        t!(Markup[" /"]: "(" => Text("("));
        t!(Markup[" /"]: ")" => Text(")"));

        // Test in code.
        t!(Code: "{" => LeftBrace);
        t!(Code: "}" => RightBrace);
        t!(Code: "[" => LeftBracket);
        t!(Code: "]" => RightBracket);
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
        t!(Both["a1/"]: "\r\n\r"       => Space(2));
        t!(Both["a1/"]: "  \n\t \n  "  => Space(2));
        t!(Both["a1/"]: "\n\r"         => Space(2));
        t!(Both["a1/"]: " \r\r\n \x0D" => Space(3));
    }

    #[test]
    fn test_tokenize_text() {
        // Test basic text.
        t!(Markup[" /"]: "hello"      => Text("hello"));
        t!(Markup[" /"]: "reha-world" => Text("reha-world"));

        // Test code symbols in text.
        t!(Markup[" /"]: "a():\"b" => Text("a()"), Colon, Quote { double: true }, Text("b"));
        t!(Markup[" /"]: ";,|/+"  => Text(";,|/+"));
        t!(Markup[" /"]: "=-a"     => Eq, Minus, Text("a"));
        t!(Markup[" "]: "#123"     => Text("#123"));

        // Test text ends.
        t!(Markup[""]: "hello " => Text("hello"), Space(0));
        t!(Markup[""]: "hello~" => Text("hello"), Tilde);
    }

    #[test]
    fn test_tokenize_escape_sequences() {
        // Test escapable symbols.
        t!(Markup: r"\\" => Escape('\\'));
        t!(Markup: r"\/" => Escape('/'));
        t!(Markup: r"\[" => Escape('['));
        t!(Markup: r"\]" => Escape(']'));
        t!(Markup: r"\{" => Escape('{'));
        t!(Markup: r"\}" => Escape('}'));
        t!(Markup: r"\*" => Escape('*'));
        t!(Markup: r"\_" => Escape('_'));
        t!(Markup: r"\=" => Escape('='));
        t!(Markup: r"\~" => Escape('~'));
        t!(Markup: r"\'" => Escape('\''));
        t!(Markup: r#"\""# => Escape('"'));
        t!(Markup: r"\`" => Escape('`'));
        t!(Markup: r"\$" => Escape('$'));
        t!(Markup: r"\#" => Escape('#'));
        t!(Markup: r"\a"   => Escape('a'));
        t!(Markup: r"\u"   => Escape('u'));
        t!(Markup: r"\1"   => Escape('1'));

        // Test basic unicode escapes.
        t!(Markup: r"\u{}"     => Error(Full, "invalid unicode escape sequence"));
        t!(Markup: r"\u{2603}" => Escape('☃'));
        t!(Markup: r"\u{P}"    => Error(Full, "invalid unicode escape sequence"));

        // Test unclosed unicode escapes.
        t!(Markup[" /"]: r"\u{"     => Error(End, "expected closing brace"));
        t!(Markup[" /"]: r"\u{1"    => Error(End, "expected closing brace"));
        t!(Markup[" /"]: r"\u{26A4" => Error(End, "expected closing brace"));
        t!(Markup[" /"]: r"\u{1Q3P" => Error(End, "expected closing brace"));
        t!(Markup: r"\u{1🏕}"       => Error(End, "expected closing brace"), Text("🏕"), RightBrace);
    }

    #[test]
    fn test_tokenize_markup_symbols() {
        // Test markup tokens.
        t!(Markup[" a1"]: "*"   => Star);
        t!(Markup: "_"          => Underscore);
        t!(Markup[""]: "==="    => Eq, Eq, Eq);
        t!(Markup["a1/"]: "= "  => Eq, Space(0));
        t!(Markup[" "]: r"\"    => Backslash);
        t!(Markup: "~"          => Tilde);
        t!(Markup["a1/"]: "-?"  => HyphQuest);
        t!(Markup["a "]: r"a--" => Text("a"), Hyph2);
        t!(Markup["a1/"]: "- "  => Minus, Space(0));
        t!(Markup[" "]: "+"     => Plus);
        t!(Markup[" "]: "1."    => EnumNumbering(1));
        t!(Markup[" "]: "1.a"   => EnumNumbering(1), Text("a"));
        t!(Markup[" /"]: "a1."  => Text("a1."));
    }

    #[test]
    fn test_tokenize_code_symbols() {
        // Test all symbols.
        t!(Code: ","        => Comma);
        t!(Code: ";"        => Semicolon);
        t!(Code: ":"        => Colon);
        t!(Code: "+"        => Plus);
        t!(Code: "-"        => Minus);
        t!(Code[" a1"]: "*" => Star);
        t!(Code[" a1"]: "/" => Slash);
        t!(Code[" a/"]: "." => Dot);
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
        t!(Code[" a/"]: "..." => Dots, Dot);

        // Test hyphen as symbol vs part of identifier.
        t!(Code[" /"]: "-1"   => Minus, Int(1));
        t!(Code[" /"]: "-a"   => Minus, Ident("a"));
        t!(Code[" /"]: "--1"  => Minus, Minus, Int(1));
        t!(Code[" /"]: "--_a" => Minus, Minus, Ident("_a"));
        t!(Code[" /"]: "a-b"  => Ident("a-b"));

        // Test invalid.
        t!(Code: r"\" => Error(Full, "not valid here"));
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

        for (s, t) in list.clone() {
            t!(Markup[" "]: format!("#{}", s) => t);
            t!(Markup[" "]: format!("#{0}#{0}", s) => t, t);
            t!(Markup[" /"]: format!("# {}", s) => Text(&format!("# {s}")));
        }

        for (s, t) in list {
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
        // Test basic raw block.
        t!(Markup: "``"     => Raw("", None, false));
        t!(Markup: "`raw`"  => Raw("raw", None, false));
        t!(Markup[""]: "`]" => Error(End, "expected 1 backtick"));

        // Test special symbols in raw block.
        t!(Markup: "`[brackets]`" => Raw("[brackets]", None, false));
        t!(Markup[""]: r"`\`` "   => Raw(r"\", None, false), Error(End, "expected 1 backtick"));

        // Test separated closing backticks.
        t!(Markup: "```not `y`e`t```" => Raw("`y`e`t", Some("not"), false));

        // Test more backticks.
        t!(Markup: "``nope``"             => Raw("", None, false), Text("nope"), Raw("", None, false));
        t!(Markup: "````🚀````"           => Raw("", None, false));
        t!(Markup[""]: "`````👩‍🚀````noend" => Error(End, "expected 5 backticks"));
        t!(Markup[""]: "````raw``````"    => Raw("", Some("raw"), false), Raw("", None, false));
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

        let suffixes: &[(&str, fn(f64) -> NodeKind)] = &[
            ("mm", |x| Numeric(x, Unit::Length(LengthUnit::Mm))),
            ("pt", |x| Numeric(x, Unit::Length(LengthUnit::Pt))),
            ("cm", |x| Numeric(x, Unit::Length(LengthUnit::Cm))),
            ("in", |x| Numeric(x, Unit::Length(LengthUnit::In))),
            ("rad", |x| Numeric(x, Unit::Angle(AngleUnit::Rad))),
            ("deg", |x| Numeric(x, Unit::Angle(AngleUnit::Deg))),
            ("em", |x| Numeric(x, Unit::Em)),
            ("fr", |x| Numeric(x, Unit::Fr)),
            ("%", |x| Numeric(x, Unit::Percent)),
        ];

        // Numeric types.
        for &(suffix, build) in suffixes {
            for (s, v) in nums.clone() {
                t!(Code[" /"]: format!("{}{}", s, suffix) => build(v));
            }
        }

        // Multiple dots close the number.
        t!(Code[" /"]: "1..2"   => Int(1), Dots, Int(2));
        t!(Code[" /"]: "1..2.3" => Int(1), Dots, Float(2.3));
        t!(Code[" /"]: "1.2..3" => Float(1.2), Dots, Int(3));

        // Test invalid.
        t!(Code[" /"]: "1foo" => Error(Full, "invalid number suffix"));
    }

    #[test]
    fn test_tokenize_strings() {
        // Test basic strings.
        t!(Code: "\"hi\""        => Str("hi"));
        t!(Code: "\"hi\nthere\"" => Str("hi\nthere"));
        t!(Code: "\"🌎\""        => Str("🌎"));

        // Test unterminated.
        t!(Code[""]: "\"hi" => Error(End, "expected quote"));

        // Test escaped quote.
        t!(Code: r#""a\"bc""# => Str("a\"bc"));
        t!(Code[""]: r#""\""# => Error(End, "expected quote"));
    }

    #[test]
    fn test_tokenize_line_comments() {
        // Test line comment with no trailing newline.
        t!(Both[""]: "//" => LineComment);

        // Test line comment ends at newline.
        t!(Both["a1/"]: "//bc\n"   => LineComment, Space(1));
        t!(Both["a1/"]: "// bc \n" => LineComment, Space(1));
        t!(Both["a1/"]: "//bc\r\n" => LineComment, Space(1));

        // Test nested line comments.
        t!(Both["a1/"]: "//a//b\n" => LineComment, Space(1));
    }

    #[test]
    fn test_tokenize_block_comments() {
        // Test basic block comments.
        t!(Both[""]: "/*" => BlockComment);
        t!(Both: "/**/"   => BlockComment);
        t!(Both: "/*🏞*/" => BlockComment);
        t!(Both: "/*\n*/" => BlockComment);

        // Test depth 1 and 2 nested block comments.
        t!(Both: "/* /* */ */"  => BlockComment);
        t!(Both: "/*/*/**/*/*/" => BlockComment);

        // Test two nested, one unclosed block comments.
        t!(Both[""]: "/*/*/**/*/" => BlockComment);

        // Test all combinations of up to two following slashes and stars.
        t!(Both[""]: "/*"   => BlockComment);
        t!(Both[""]: "/*/"  => BlockComment);
        t!(Both[""]: "/**"  => BlockComment);
        t!(Both[""]: "/*//" => BlockComment);
        t!(Both[""]: "/*/*" => BlockComment);
        t!(Both[""]: "/**/" => BlockComment);
        t!(Both[""]: "/***" => BlockComment);

        // Test unexpected terminator.
        t!(Both: "/*Hi*/*/" => BlockComment,
           Error(Full, "unexpected end of block comment"));
    }
}
