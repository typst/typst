use ecow::{eco_format, EcoString};
use unicode_ident::{is_xid_continue, is_xid_start};
use unicode_script::{Script, UnicodeScript};
use unicode_segmentation::UnicodeSegmentation;
use unscanny::Scanner;

use crate::{SyntaxError, SyntaxKind};

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
    /// The state held by raw line lexing.
    raw: Vec<(SyntaxKind, usize)>,
    /// An error for the last token.
    error: Option<SyntaxError>,
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
    /// The contents of a raw block.
    Raw,
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
            raw: Vec::new(),
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
    pub fn take_error(&mut self) -> Option<SyntaxError> {
        self.error.take()
    }
}

impl Lexer<'_> {
    /// Construct a full-positioned syntax error.
    fn error(&mut self, message: impl Into<EcoString>) -> SyntaxKind {
        self.error = Some(SyntaxError::new(message));
        SyntaxKind::Error
    }

    /// If the current node is an error, adds a hint.
    fn hint(&mut self, message: impl Into<EcoString>) {
        if let Some(error) = &mut self.error {
            error.hints.push(message.into());
        }
    }
}

/// Shared methods with all [`LexMode`].
impl Lexer<'_> {
    /// Proceed to the next token and return its [`SyntaxKind`]. Note the
    /// token could be a [trivia](SyntaxKind::is_trivia).
    pub fn next(&mut self) -> SyntaxKind {
        if self.mode == LexMode::Raw {
            let Some((kind, end)) = self.raw.pop() else {
                return SyntaxKind::End;
            };
            self.s.jump(end);
            return kind;
        }

        self.newline = false;
        self.error = None;
        let start = self.s.cursor();
        match self.s.eat() {
            Some(c) if is_space(c, self.mode) => self.whitespace(start, c),
            Some('/') if self.s.eat_if('/') => self.line_comment(),
            Some('/') if self.s.eat_if('*') => self.block_comment(),
            Some('*') if self.s.eat_if('/') => {
                let kind = self.error("unexpected end of block comment");
                self.hint(
                    "consider escaping the `*` with a backslash or \
                     opening the block comment with `/*`",
                );
                kind
            }

            Some(c) => match self.mode {
                LexMode::Markup => self.markup(start, c),
                LexMode::Math => self.math(start, c),
                LexMode::Code => self.code(start, c),
                LexMode::Raw => unreachable!(),
            },

            None => SyntaxKind::End,
        }
    }

    /// Eat whitespace characters greedily.
    fn whitespace(&mut self, start: usize, c: char) -> SyntaxKind {
        let more = self.s.eat_while(|c| is_space(c, self.mode));
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
            '-' if self.s.at(char::is_numeric) => SyntaxKind::Shorthand,
            '*' if !self.in_word() => SyntaxKind::Star,
            '_' if !self.in_word() => SyntaxKind::Underscore,

            '#' => SyntaxKind::Hash,
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
                return self.error("unclosed Unicode escape sequence");
            }

            if u32::from_str_radix(hex, 16)
                .ok()
                .and_then(std::char::from_u32)
                .is_none()
            {
                return self.error(eco_format!("invalid Unicode codepoint: {}", hex));
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
        let start = self.s.cursor() - 1;
        self.raw.clear();

        // Determine number of opening backticks.
        let mut backticks = 1;
        while self.s.eat_if('`') {
            backticks += 1;
        }

        // Special case for ``.
        if backticks == 2 {
            self.push_raw(SyntaxKind::RawDelim);
            self.s.jump(start + 1);
            return SyntaxKind::RawDelim;
        }

        // Find end of raw text.
        let mut found = 0;
        while found < backticks {
            match self.s.eat() {
                Some('`') => found += 1,
                Some(_) => found = 0,
                None => break,
            }
        }

        if found != backticks {
            return self.error("unclosed raw text");
        }

        let end = self.s.cursor();
        if backticks >= 3 {
            self.blocky_raw(start, end, backticks);
        } else {
            self.inline_raw(start, end, backticks);
        }

        // Closing delimiter.
        self.push_raw(SyntaxKind::RawDelim);

        // The saved tokens will be removed in reverse.
        self.raw.reverse();

        // Opening delimiter.
        self.s.jump(start + backticks);
        SyntaxKind::RawDelim
    }

    fn blocky_raw(&mut self, start: usize, end: usize, backticks: usize) {
        // Language tag.
        self.s.jump(start + backticks);
        if self.s.eat_if(is_id_start) {
            self.s.eat_while(is_id_continue);
            self.push_raw(SyntaxKind::RawLang);
        }

        // Determine inner content between backticks.
        self.s.eat_if(' ');
        let inner = self.s.to(end - backticks);

        // Determine dedent level.
        let mut lines = split_newlines(inner);
        let dedent = lines
            .iter()
            .skip(1)
            .filter(|line| !line.chars().all(char::is_whitespace))
            // The line with the closing ``` is always taken into account
            .chain(lines.last())
            .map(|line| line.chars().take_while(|c| c.is_whitespace()).count())
            .min()
            .unwrap_or(0);

        // Trim single space in last line if text ends with a backtick. The last
        // line is the one directly before the closing backticks and if it is
        // just whitespace, it will be completely trimmed below.
        if inner.trim_end().ends_with('`') {
            if let Some(last) = lines.last_mut() {
                *last = last.strip_suffix(' ').unwrap_or(last);
            }
        }

        let is_whitespace = |line: &&str| line.chars().all(char::is_whitespace);
        let starts_whitespace = lines.first().is_some_and(is_whitespace);
        let ends_whitespace = lines.last().is_some_and(is_whitespace);

        let mut lines = lines.into_iter();
        let mut skipped = false;

        // Trim whitespace + newline at start.
        if starts_whitespace {
            self.s.advance(lines.next().unwrap().len());
            skipped = true;
        }
        // Trim whitespace + newline at end.
        if ends_whitespace {
            lines.next_back();
        }

        // Add lines.
        for (i, line) in lines.enumerate() {
            let dedent = if i == 0 && !skipped { 0 } else { dedent };
            let offset: usize = line.chars().take(dedent).map(char::len_utf8).sum();
            self.s.eat_newline();
            self.s.advance(offset);
            self.push_raw(SyntaxKind::RawTrimmed);
            self.s.advance(line.len() - offset);
            self.push_raw(SyntaxKind::Text);
        }

        // Add final trimmed.
        if self.s.cursor() < end - backticks {
            self.s.jump(end - backticks);
            self.push_raw(SyntaxKind::RawTrimmed);
        }
        self.s.jump(end);
    }

    fn inline_raw(&mut self, start: usize, end: usize, backticks: usize) {
        self.s.jump(start + backticks);

        while self.s.cursor() < end - backticks {
            if self.s.at(is_newline) {
                self.push_raw(SyntaxKind::Text);
                self.s.eat_newline();
                self.push_raw(SyntaxKind::RawTrimmed);
                continue;
            }
            self.s.eat();
        }
        self.push_raw(SyntaxKind::Text);

        self.s.jump(end);
    }

    /// Push the current cursor that marks the end of a raw segment of
    /// the given `kind`.
    fn push_raw(&mut self, kind: SyntaxKind) {
        let end = self.s.cursor();
        self.raw.push((kind, end));
    }

    fn link(&mut self) -> SyntaxKind {
        let (link, balanced) = link_prefix(self.s.after());
        self.s.advance(link.len());

        if !balanced {
            return self.error(
                "automatic links cannot contain unbalanced brackets, \
                 use the `link` function instead",
            );
        }

        SyntaxKind::Link
    }

    fn numbering(&mut self, start: usize) -> SyntaxKind {
        self.s.eat_while(char::is_ascii_digit);

        let read = self.s.from(start);
        if self.s.eat_if('.') && self.space_or_end() && read.parse::<usize>().is_ok() {
            return SyntaxKind::EnumMarker;
        }

        self.text()
    }

    fn ref_marker(&mut self) -> SyntaxKind {
        self.s.eat_while(is_valid_in_label_literal);

        // Don't include the trailing characters likely to be part of text.
        while matches!(self.s.scout(-1), Some('.' | ':')) {
            self.s.uneat();
        }

        SyntaxKind::RefMarker
    }

    fn label(&mut self) -> SyntaxKind {
        let label = self.s.eat_while(is_valid_in_label_literal);
        if label.is_empty() {
            return self.error("label cannot be empty");
        }

        if !self.s.eat_if('>') {
            return self.error("unclosed label");
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
            | '[' | ']' | '~' | '-' | '.' | '\'' | '"' | '*' | '_'
            | ':' | 'h' | '`' | '$' | '<' | '>' | '@' | '#'
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
                Some('@') if !s.at(is_valid_in_label_literal) => {}
                _ => break,
            }

            self.s = s;
        }

        SyntaxKind::Text
    }

    fn in_word(&self) -> bool {
        let wordy = |c: Option<char>| {
            c.is_some_and(|c| {
                c.is_alphanumeric()
                    && !matches!(
                        c.script(),
                        Script::Han
                            | Script::Hiragana
                            | Script::Katakana
                            | Script::Hangul
                    )
            })
        };
        let prev = self.s.scout(-2);
        let next = self.s.peek();
        wordy(prev) && wordy(next)
    }

    fn space_or_end(&self) -> bool {
        self.s.done()
            || self.s.at(char::is_whitespace)
            || self.s.at("//")
            || self.s.at("/*")
    }
}

/// Math.
impl Lexer<'_> {
    fn math(&mut self, start: usize, c: char) -> SyntaxKind {
        match c {
            '\\' => self.backslash(),
            '"' => self.string(),

            '-' if self.s.eat_if(">>") => SyntaxKind::MathShorthand,
            '-' if self.s.eat_if('>') => SyntaxKind::MathShorthand,
            '-' if self.s.eat_if("->") => SyntaxKind::MathShorthand,
            ':' if self.s.eat_if('=') => SyntaxKind::MathShorthand,
            ':' if self.s.eat_if(":=") => SyntaxKind::MathShorthand,
            '!' if self.s.eat_if('=') => SyntaxKind::MathShorthand,
            '.' if self.s.eat_if("..") => SyntaxKind::MathShorthand,
            '[' if self.s.eat_if('|') => SyntaxKind::MathShorthand,
            '<' if self.s.eat_if("==>") => SyntaxKind::MathShorthand,
            '<' if self.s.eat_if("-->") => SyntaxKind::MathShorthand,
            '<' if self.s.eat_if("--") => SyntaxKind::MathShorthand,
            '<' if self.s.eat_if("-<") => SyntaxKind::MathShorthand,
            '<' if self.s.eat_if("->") => SyntaxKind::MathShorthand,
            '<' if self.s.eat_if("<-") => SyntaxKind::MathShorthand,
            '<' if self.s.eat_if("<<") => SyntaxKind::MathShorthand,
            '<' if self.s.eat_if("=>") => SyntaxKind::MathShorthand,
            '<' if self.s.eat_if("==") => SyntaxKind::MathShorthand,
            '<' if self.s.eat_if("~~") => SyntaxKind::MathShorthand,
            '<' if self.s.eat_if('=') => SyntaxKind::MathShorthand,
            '<' if self.s.eat_if('<') => SyntaxKind::MathShorthand,
            '<' if self.s.eat_if('-') => SyntaxKind::MathShorthand,
            '<' if self.s.eat_if('~') => SyntaxKind::MathShorthand,
            '>' if self.s.eat_if("->") => SyntaxKind::MathShorthand,
            '>' if self.s.eat_if(">>") => SyntaxKind::MathShorthand,
            '=' if self.s.eat_if("=>") => SyntaxKind::MathShorthand,
            '=' if self.s.eat_if('>') => SyntaxKind::MathShorthand,
            '=' if self.s.eat_if(':') => SyntaxKind::MathShorthand,
            '>' if self.s.eat_if('=') => SyntaxKind::MathShorthand,
            '>' if self.s.eat_if('>') => SyntaxKind::MathShorthand,
            '|' if self.s.eat_if("->") => SyntaxKind::MathShorthand,
            '|' if self.s.eat_if("=>") => SyntaxKind::MathShorthand,
            '|' if self.s.eat_if(']') => SyntaxKind::MathShorthand,
            '|' if self.s.eat_if('|') => SyntaxKind::MathShorthand,
            '~' if self.s.eat_if("~>") => SyntaxKind::MathShorthand,
            '~' if self.s.eat_if('>') => SyntaxKind::MathShorthand,
            '*' | '-' | '~' => SyntaxKind::MathShorthand,

            '#' => SyntaxKind::Hash,
            '_' => SyntaxKind::Underscore,
            '$' => SyntaxKind::Dollar,
            '/' => SyntaxKind::Slash,
            '^' => SyntaxKind::Hat,
            '\'' => SyntaxKind::Prime,
            '&' => SyntaxKind::MathAlignPoint,
            '√' | '∛' | '∜' => SyntaxKind::Root,

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
            '-' | '\u{2212}' if self.s.eat_if('=') => SyntaxKind::HyphEq,
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
            '-' | '\u{2212}' => SyntaxKind::Minus,
            '*' => SyntaxKind::Star,
            '/' => SyntaxKind::Slash,
            '=' => SyntaxKind::Eq,
            '<' => SyntaxKind::Lt,
            '>' => SyntaxKind::Gt,

            c if is_id_start(c) => self.ident(start),

            c => self.error(eco_format!("the character `{c}` is not valid in code")),
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

        if ident == "_" {
            SyntaxKind::Underscore
        } else {
            SyntaxKind::Ident
        }
    }

    fn number(&mut self, mut start: usize, c: char) -> SyntaxKind {
        // Handle alternative integer bases.
        let mut base = 10;
        if c == '0' {
            if self.s.eat_if('b') {
                base = 2;
            } else if self.s.eat_if('o') {
                base = 8;
            } else if self.s.eat_if('x') {
                base = 16;
            }
            if base != 10 {
                start = self.s.cursor();
            }
        }

        // Read the first part (integer or fractional depending on `first`).
        self.s.eat_while(if base == 16 {
            char::is_ascii_alphanumeric
        } else {
            char::is_ascii_digit
        });

        // Read the fractional part if not already done.
        // Make sure not to confuse a range for the decimal separator.
        if c != '.'
            && !self.s.at("..")
            && !self.s.scout(1).is_some_and(is_id_start)
            && self.s.eat_if('.')
            && base == 10
        {
            self.s.eat_while(char::is_ascii_digit);
        }

        // Read the exponent.
        if !self.s.at("em") && self.s.eat_if(['e', 'E']) && base == 10 {
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

        let kind = if i64::from_str_radix(number, base).is_ok() {
            SyntaxKind::Int
        } else if base == 10 && number.parse::<f64>().is_ok() {
            SyntaxKind::Float
        } else {
            return self.error(match base {
                2 => eco_format!("invalid binary number: 0b{}", number),
                8 => eco_format!("invalid octal number: 0o{}", number),
                16 => eco_format!("invalid hexadecimal number: 0x{}", number),
                _ => eco_format!("invalid number: {}", number),
            });
        };

        if suffix.is_empty() {
            return kind;
        }

        if !matches!(
            suffix,
            "pt" | "mm" | "cm" | "in" | "deg" | "rad" | "em" | "fr" | "%"
        ) {
            return self.error(eco_format!("invalid number suffix: {}", suffix));
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
            return self.error("unclosed string");
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
        "context" => SyntaxKind::Context,
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

trait ScannerExt {
    fn advance(&mut self, by: usize);
    fn eat_newline(&mut self) -> bool;
}

impl ScannerExt for Scanner<'_> {
    fn advance(&mut self, by: usize) {
        self.jump(self.cursor() + by);
    }

    fn eat_newline(&mut self) -> bool {
        let ate = self.eat_if(is_newline);
        if ate && self.before().ends_with('\r') {
            self.eat_if('\n');
        }
        ate
    }
}

/// Whether a character will become a [`SyntaxKind::Space`] token.
#[inline]
fn is_space(character: char, mode: LexMode) -> bool {
    match mode {
        LexMode::Markup => matches!(character, ' ' | '\t') || is_newline(character),
        _ => character.is_whitespace(),
    }
}

/// Whether a character is interpreted as a newline by Typst.
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

/// Extracts a prefix of the text that is a link and also returns whether the
/// parentheses and brackets in the link were balanced.
pub fn link_prefix(text: &str) -> (&str, bool) {
    let mut s = unscanny::Scanner::new(text);
    let mut brackets = Vec::new();

    #[rustfmt::skip]
    s.eat_while(|c: char| {
        match c {
            | '0' ..= '9'
            | 'a' ..= 'z'
            | 'A' ..= 'Z'
            | '!' | '#' | '$' | '%' | '&' | '*' | '+'
            | ',' | '-' | '.' | '/' | ':' | ';' | '='
            | '?' | '@' | '_' | '~' | '\'' => true,
            '[' => {
                brackets.push(b'[');
                true
            }
            '(' => {
                brackets.push(b'(');
                true
            }
            ']' => brackets.pop() == Some(b'['),
            ')' => brackets.pop() == Some(b'('),
            _ => false,
        }
    });

    // Don't include the trailing characters likely to be part of text.
    while matches!(s.scout(-1), Some('!' | ',' | '.' | ':' | ';' | '?' | '\'')) {
        s.uneat();
    }

    (s.before(), brackets.is_empty())
}

/// Split text at newlines. These newline characters are not kept.
pub fn split_newlines(text: &str) -> Vec<&str> {
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
        .is_some_and(|c| is_id_start(c) && chars.all(is_id_continue))
}

/// Whether a character can start an identifier.
#[inline]
pub fn is_id_start(c: char) -> bool {
    is_xid_start(c) || c == '_'
}

/// Whether a character can continue an identifier.
#[inline]
pub fn is_id_continue(c: char) -> bool {
    is_xid_continue(c) || c == '_' || c == '-'
}

/// Whether a character can start an identifier in math.
#[inline]
fn is_math_id_start(c: char) -> bool {
    is_xid_start(c)
}

/// Whether a character can continue an identifier in math.
#[inline]
fn is_math_id_continue(c: char) -> bool {
    is_xid_continue(c) && c != '_'
}

/// Whether a character can be part of a label literal's name.
#[inline]
fn is_valid_in_label_literal(c: char) -> bool {
    is_id_continue(c) || matches!(c, ':' | '.')
}

/// Returns true if this string is valid in a label literal.
pub fn is_valid_label_literal_id(id: &str) -> bool {
    !id.is_empty() && id.chars().all(is_valid_in_label_literal)
}
