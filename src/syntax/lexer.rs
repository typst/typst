use ecow::{eco_format, EcoString};
use unicode_segmentation::UnicodeSegmentation;
use unicode_xid::UnicodeXID;
use unscanny::Scanner;

use super::{ErrorPos, SyntaxKind};

/// Splits up a string of source code into tokens.
#[derive(Clone)]
pub(super) struct Lexer<'s> {
    /// The underlying scanner.
    s: Scanner<'s>,
    /// The mode the lexer is in. This determines which kinds of tokens it
    /// produces.
    mode: LexMode,
    /// Whether the last token contained a newline.
    newline: bool,
    /// An error for the last token.
    error: Option<(EcoString, ErrorPos)>,
}

/// What kind of tokens to emit.
#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub(super) enum LexMode {
    /// Text and markup.
    Markup,
    /// Math atoms, operators, etc.
    Math,
    /// Keywords, literals and operators.
    Code,
}

impl<'s> Lexer<'s> {
    /// Create a new lexer with the given mode and a prefix to offset column
    /// calculations.
    pub fn new(text: &'s str, mode: LexMode) -> Self {
        Self {
            s: Scanner::new(text),
            mode,
            newline: false,
            error: None,
        }
    }

    /// Get the current lexing mode.
    pub fn mode(&self) -> LexMode {
        self.mode
    }

    /// Change the lexing mode.
    pub fn set_mode(&mut self, mode: LexMode) {
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

    /// Whether the last token contained a newline.
    pub fn newline(&self) -> bool {
        self.newline
    }

    /// Take out the last error, if any.
    pub fn take_error(&mut self) -> Option<(EcoString, ErrorPos)> {
        self.error.take()
    }
}

impl Lexer<'_> {
    /// Construct a full-positioned syntax error.
    fn error(&mut self, message: impl Into<EcoString>) -> SyntaxKind {
        self.error = Some((message.into(), ErrorPos::Full));
        SyntaxKind::Error
    }

    /// Construct a positioned syntax error.
    fn error_at_end(&mut self, message: impl Into<EcoString>) -> SyntaxKind {
        self.error = Some((message.into(), ErrorPos::End));
        SyntaxKind::Error
    }
}

/// Shared.
impl Lexer<'_> {
    pub fn next(&mut self) -> SyntaxKind {
        self.newline = false;
        self.error = None;
        let start = self.s.cursor();
        match self.s.eat() {
            Some(c) if c.is_whitespace() => self.whitespace(start, c),
            Some('/') if self.s.eat_if('/') => self.line_comment(),
            Some('/') if self.s.eat_if('*') => self.block_comment(),
            Some('*') if self.s.eat_if('/') => {
                self.error("unexpected end of block comment")
            }

            Some(c) => match self.mode {
                LexMode::Markup => self.markup(start, c),
                LexMode::Math => self.math(start, c),
                LexMode::Code => self.code(start, c),
            },

            None => SyntaxKind::Eof,
        }
    }

    fn whitespace(&mut self, start: usize, c: char) -> SyntaxKind {
        let more = self.s.eat_while(char::is_whitespace);
        let newlines = match c {
            ' ' if more.is_empty() => 0,
            _ => count_newlines(self.s.from(start)),
        };

        self.newline = newlines > 0;
        if self.mode == LexMode::Markup && newlines >= 2 {
            SyntaxKind::Parbreak
        } else {
            SyntaxKind::Space
        }
    }

    fn line_comment(&mut self) -> SyntaxKind {
        self.s.eat_until(is_newline);
        SyntaxKind::LineComment
    }

    fn block_comment(&mut self) -> SyntaxKind {
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
                ('/', '/') => {
                    self.line_comment();
                    '_'
                }
                _ => c,
            }
        }

        SyntaxKind::BlockComment
    }
}

/// Markup.
impl Lexer<'_> {
    fn markup(&mut self, start: usize, c: char) -> SyntaxKind {
        match c {
            '\\' => self.backslash(),
            '`' => self.raw(),
            'h' if self.s.eat_if("ttp://") => self.link(),
            'h' if self.s.eat_if("ttps://") => self.link(),
            '<' if self.s.at(is_id_continue) => self.label(),
            '@' => self.ref_marker(),

            '.' if self.s.eat_if("..") => SyntaxKind::Shorthand,
            '-' if self.s.eat_if("--") => SyntaxKind::Shorthand,
            '-' if self.s.eat_if('-') => SyntaxKind::Shorthand,
            '-' if self.s.eat_if('?') => SyntaxKind::Shorthand,
            '*' if !self.in_word() => SyntaxKind::Star,
            '_' if !self.in_word() => SyntaxKind::Underscore,

            '#' => SyntaxKind::Hashtag,
            '[' => SyntaxKind::LeftBracket,
            ']' => SyntaxKind::RightBracket,
            '\'' => SyntaxKind::SmartQuote,
            '"' => SyntaxKind::SmartQuote,
            '$' => SyntaxKind::Dollar,
            '~' => SyntaxKind::Shorthand,
            ':' => SyntaxKind::Colon,
            '=' => {
                self.s.eat_while('=');
                if self.space_or_end() {
                    SyntaxKind::HeadingMarker
                } else {
                    self.text()
                }
            }
            '-' if self.space_or_end() => SyntaxKind::ListMarker,
            '+' if self.space_or_end() => SyntaxKind::EnumMarker,
            '/' if self.space_or_end() => SyntaxKind::TermMarker,
            '0'..='9' => self.numbering(start),

            _ => self.text(),
        }
    }

    fn backslash(&mut self) -> SyntaxKind {
        if self.s.eat_if("u{") {
            let hex = self.s.eat_while(char::is_ascii_alphanumeric);
            if !self.s.eat_if('}') {
                return self.error_at_end("expected closing brace");
            }

            if u32::from_str_radix(hex, 16)
                .ok()
                .and_then(std::char::from_u32)
                .is_none()
            {
                return self.error("invalid unicode escape sequence");
            }

            return SyntaxKind::Escape;
        }

        if self.s.done() || self.s.at(char::is_whitespace) {
            SyntaxKind::Linebreak
        } else {
            self.s.eat();
            SyntaxKind::Escape
        }
    }

    fn raw(&mut self) -> SyntaxKind {
        let mut backticks = 1;
        while self.s.eat_if('`') {
            backticks += 1;
        }

        if backticks == 2 {
            return SyntaxKind::Raw;
        }

        let mut found = 0;
        while found < backticks {
            match self.s.eat() {
                Some('`') => found += 1,
                Some(_) => found = 0,
                None => break,
            }
        }

        if found != backticks {
            let remaining = backticks - found;
            let noun = if remaining == 1 { "backtick" } else { "backticks" };
            return self.error_at_end(if found == 0 {
                eco_format!("expected {} {}", remaining, noun)
            } else {
                eco_format!("expected {} more {}", remaining, noun)
            });
        }

        SyntaxKind::Raw
    }

    fn link(&mut self) -> SyntaxKind {
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

        SyntaxKind::Link
    }

    fn numbering(&mut self, start: usize) -> SyntaxKind {
        self.s.eat_while(char::is_ascii_digit);

        let read = self.s.from(start);
        if self.s.eat_if('.') && self.space_or_end() {
            if read.parse::<usize>().is_ok() {
                return SyntaxKind::EnumMarker;
            }
        }

        self.text()
    }

    fn ref_marker(&mut self) -> SyntaxKind {
        self.s.eat_while(is_id_continue);
        SyntaxKind::RefMarker
    }

    fn label(&mut self) -> SyntaxKind {
        let label = self.s.eat_while(is_id_continue);
        if label.is_empty() {
            return self.error("label cannot be empty");
        }

        if !self.s.eat_if('>') {
            return self.error_at_end("expected closing angle bracket");
        }

        SyntaxKind::Label
    }

    fn text(&mut self) -> SyntaxKind {
        macro_rules! table {
            ($(|$c:literal)*) => {
                static TABLE: [bool; 128] = {
                    let mut t = [false; 128];
                    $(t[$c as usize] = true;)*
                    t
                };
            };
        }

        table! {
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
                Some(' ') if s.at(char::is_alphanumeric) => {}
                Some('/') if !s.at(['/', '*']) => {}
                Some('-') if !s.at(['-', '?']) => {}
                Some('.') if !s.at("..") => {}
                Some('h') if !s.at("ttp://") && !s.at("ttps://") => {}
                Some('@') if !s.at(is_id_start) => {}
                _ => break,
            }

            self.s = s;
        }

        SyntaxKind::Text
    }

    fn in_word(&self) -> bool {
        let alphanum = |c: Option<char>| c.map_or(false, |c| c.is_alphanumeric());
        let prev = self.s.scout(-2);
        let next = self.s.peek();
        alphanum(prev) && alphanum(next)
    }

    fn space_or_end(&self) -> bool {
        self.s.done() || self.s.at(char::is_whitespace)
    }
}

/// Math.
impl Lexer<'_> {
    fn math(&mut self, start: usize, c: char) -> SyntaxKind {
        match c {
            '\\' => self.backslash(),
            '"' => self.string(),

            '-' if self.s.eat_if(">>") => SyntaxKind::Shorthand,
            '-' if self.s.eat_if('>') => SyntaxKind::Shorthand,
            '-' if self.s.eat_if("->") => SyntaxKind::Shorthand,
            ':' if self.s.eat_if('=') => SyntaxKind::Shorthand,
            ':' if self.s.eat_if(":=") => SyntaxKind::Shorthand,
            '!' if self.s.eat_if('=') => SyntaxKind::Shorthand,
            '.' if self.s.eat_if("..") => SyntaxKind::Shorthand,
            '[' if self.s.eat_if('|') => SyntaxKind::Shorthand,
            '<' if self.s.eat_if("==>") => SyntaxKind::Shorthand,
            '<' if self.s.eat_if("-->") => SyntaxKind::Shorthand,
            '<' if self.s.eat_if("--") => SyntaxKind::Shorthand,
            '<' if self.s.eat_if("-<") => SyntaxKind::Shorthand,
            '<' if self.s.eat_if("->") => SyntaxKind::Shorthand,
            '<' if self.s.eat_if("<-") => SyntaxKind::Shorthand,
            '<' if self.s.eat_if("<<") => SyntaxKind::Shorthand,
            '<' if self.s.eat_if("=>") => SyntaxKind::Shorthand,
            '<' if self.s.eat_if("==") => SyntaxKind::Shorthand,
            '<' if self.s.eat_if("~~") => SyntaxKind::Shorthand,
            '<' if self.s.eat_if('=') => SyntaxKind::Shorthand,
            '<' if self.s.eat_if('<') => SyntaxKind::Shorthand,
            '<' if self.s.eat_if('-') => SyntaxKind::Shorthand,
            '<' if self.s.eat_if('~') => SyntaxKind::Shorthand,
            '>' if self.s.eat_if("->") => SyntaxKind::Shorthand,
            '>' if self.s.eat_if(">>") => SyntaxKind::Shorthand,
            '=' if self.s.eat_if("=>") => SyntaxKind::Shorthand,
            '=' if self.s.eat_if('>') => SyntaxKind::Shorthand,
            '=' if self.s.eat_if(':') => SyntaxKind::Shorthand,
            '>' if self.s.eat_if('=') => SyntaxKind::Shorthand,
            '>' if self.s.eat_if('>') => SyntaxKind::Shorthand,
            '|' if self.s.eat_if("->") => SyntaxKind::Shorthand,
            '|' if self.s.eat_if("=>") => SyntaxKind::Shorthand,
            '|' if self.s.eat_if(']') => SyntaxKind::Shorthand,
            '|' if self.s.eat_if('|') => SyntaxKind::Shorthand,
            '~' if self.s.eat_if("~>") => SyntaxKind::Shorthand,
            '~' if self.s.eat_if('>') => SyntaxKind::Shorthand,
            '*' | '\'' | '-' => SyntaxKind::Shorthand,

            '#' => SyntaxKind::Hashtag,
            '_' => SyntaxKind::Underscore,
            '$' => SyntaxKind::Dollar,
            '/' => SyntaxKind::Slash,
            '^' => SyntaxKind::Hat,
            '&' => SyntaxKind::MathAlignPoint,

            // Identifiers.
            c if is_math_id_start(c) && self.s.at(is_math_id_continue) => {
                self.s.eat_while(is_math_id_continue);
                SyntaxKind::MathIdent
            }

            // Other math atoms.
            _ => self.math_text(start, c),
        }
    }

    fn math_text(&mut self, start: usize, c: char) -> SyntaxKind {
        // Keep numbers and grapheme clusters together.
        if c.is_numeric() {
            self.s.eat_while(char::is_numeric);
            let mut s = self.s;
            if s.eat_if('.') && !s.eat_while(char::is_numeric).is_empty() {
                self.s = s;
            }
        } else {
            let len = self
                .s
                .get(start..self.s.string().len())
                .graphemes(true)
                .next()
                .map_or(0, str::len);
            self.s.jump(start + len);
        }
        SyntaxKind::Text
    }
}

/// Code.
impl Lexer<'_> {
    fn code(&mut self, start: usize, c: char) -> SyntaxKind {
        match c {
            '`' => self.raw(),
            '<' if self.s.at(is_id_continue) => self.label(),
            '0'..='9' => self.number(start, c),
            '.' if self.s.at(char::is_ascii_digit) => self.number(start, c),
            '"' => self.string(),

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

            '{' => SyntaxKind::LeftBrace,
            '}' => SyntaxKind::RightBrace,
            '[' => SyntaxKind::LeftBracket,
            ']' => SyntaxKind::RightBracket,
            '(' => SyntaxKind::LeftParen,
            ')' => SyntaxKind::RightParen,
            '$' => SyntaxKind::Dollar,
            ',' => SyntaxKind::Comma,
            ';' => SyntaxKind::Semicolon,
            ':' => SyntaxKind::Colon,
            '.' => SyntaxKind::Dot,
            '+' => SyntaxKind::Plus,
            '-' => SyntaxKind::Minus,
            '*' => SyntaxKind::Star,
            '/' => SyntaxKind::Slash,
            '=' => SyntaxKind::Eq,
            '<' => SyntaxKind::Lt,
            '>' => SyntaxKind::Gt,

            c if is_id_start(c) => self.ident(start),

            _ => self.error("this character is not valid in code"),
        }
    }

    fn ident(&mut self, start: usize) -> SyntaxKind {
        self.s.eat_while(is_id_continue);
        let ident = self.s.from(start);

        let prev = self.s.get(0..start);
        if !prev.ends_with(['.', '@']) || prev.ends_with("..") {
            if let Some(keyword) = keyword(ident) {
                return keyword;
            }
        }

        SyntaxKind::Ident
    }

    fn number(&mut self, start: usize, c: char) -> SyntaxKind {
        // Read the first part (integer or fractional depending on `first`).
        self.s.eat_while(char::is_ascii_digit);

        // Read the fractional part if not already done.
        // Make sure not to confuse a range for the decimal separator.
        if c != '.'
            && !self.s.at("..")
            && !self.s.scout(1).map_or(false, is_id_start)
            && self.s.eat_if('.')
        {
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

        if suffix.is_empty() {
            return if number.parse::<i64>().is_ok() {
                SyntaxKind::Int
            } else if number.parse::<f64>().is_ok() {
                SyntaxKind::Float
            } else {
                self.error("invalid number")
            };
        }

        if !matches!(
            suffix,
            "pt" | "mm" | "cm" | "in" | "deg" | "rad" | "em" | "fr" | "%"
        ) {
            return self.error("invalid number suffix");
        }

        SyntaxKind::Numeric
    }

    fn string(&mut self) -> SyntaxKind {
        let mut escaped = false;
        self.s.eat_until(|c| {
            let stop = c == '"' && !escaped;
            escaped = c == '\\' && !escaped;
            stop
        });

        if !self.s.eat_if('"') {
            return self.error_at_end("expected quote");
        }

        SyntaxKind::Str
    }
}

/// Try to parse an identifier into a keyword.
fn keyword(ident: &str) -> Option<SyntaxKind> {
    Some(match ident {
        "none" => SyntaxKind::None,
        "auto" => SyntaxKind::Auto,
        "true" => SyntaxKind::Bool,
        "false" => SyntaxKind::Bool,
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

/// Split text at newlines.
pub(super) fn split_newlines(text: &str) -> Vec<&str> {
    let mut s = Scanner::new(text);
    let mut lines = Vec::new();
    let mut start = 0;
    let mut end = 0;

    while let Some(c) = s.eat() {
        if is_newline(c) {
            if c == '\r' {
                s.eat_if('\n');
            }

            lines.push(&text[start..end]);
            start = s.cursor();
        }
        end = s.cursor();
    }

    lines.push(&text[start..]);
    lines
}

/// Count the number of newlines in text.
fn count_newlines(text: &str) -> usize {
    let mut newlines = 0;
    let mut s = Scanner::new(text);
    while let Some(c) = s.eat() {
        if is_newline(c) {
            if c == '\r' {
                s.eat_if('\n');
            }
            newlines += 1;
        }
    }
    newlines
}

/// Whether a string is a valid Typst identifier.
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
pub(crate) fn is_id_start(c: char) -> bool {
    c.is_xid_start() || c == '_'
}

/// Whether a character can continue an identifier.
#[inline]
pub(crate) fn is_id_continue(c: char) -> bool {
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
