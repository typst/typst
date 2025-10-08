use ecow::{EcoString, eco_format};
use typst_utils::default_math_class;
use unicode_ident::{is_xid_continue, is_xid_start};
use unicode_math_class::MathClass;
use unicode_script::{Script, UnicodeScript};
use unicode_segmentation::UnicodeSegmentation;
use unscanny::Scanner;

use crate::{SyntaxError, SyntaxKind, SyntaxMode, SyntaxNode};

/// An iterator over a source code string which returns tokens.
#[derive(Clone)]
pub(super) struct Lexer<'s> {
    /// The scanner: contains the underlying string and location as a "cursor".
    s: Scanner<'s>,
    /// The mode the lexer is in. This determines which kinds of tokens it
    /// produces.
    mode: SyntaxMode,
    /// Whether the last token contained a newline.
    newline: bool,
    /// An error for the last token.
    error: Option<SyntaxError>,
}

impl<'s> Lexer<'s> {
    /// Create a new lexer with the given mode and a prefix to offset column
    /// calculations.
    pub fn new(text: &'s str, mode: SyntaxMode) -> Self {
        Self {
            s: Scanner::new(text),
            mode,
            newline: false,
            error: None,
        }
    }

    /// Get the current lexing mode.
    pub fn mode(&self) -> SyntaxMode {
        self.mode
    }

    /// Change the lexing mode.
    pub fn set_mode(&mut self, mode: SyntaxMode) {
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

    /// The number of characters until the most recent newline from an index.
    pub fn column(&self, index: usize) -> usize {
        let mut s = self.s; // Make a new temporary scanner (cheap).
        s.jump(index);
        s.before().chars().rev().take_while(|&c| !is_newline(c)).count()
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

/// Shared methods with all [`SyntaxMode`].
impl Lexer<'_> {
    /// Return the next token in our text. Returns both the [`SyntaxNode`]
    /// and the raw [`SyntaxKind`] to make it more ergonomic to check the kind
    pub fn next(&mut self) -> (SyntaxKind, SyntaxNode) {
        debug_assert!(self.error.is_none());
        let start = self.s.cursor();

        self.newline = false;
        let kind = match self.s.eat() {
            Some(c) if is_space(c, self.mode) => self.whitespace(start, c),
            Some('#') if start == 0 && self.s.eat_if('!') => self.shebang(),
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
            Some('`') if self.mode != SyntaxMode::Math => return self.raw(),
            Some(c) => match self.mode {
                SyntaxMode::Markup => self.markup(start, c),
                SyntaxMode::Math => match self.math(start, c) {
                    (kind, None) => kind,
                    (kind, Some(node)) => return (kind, node),
                },
                SyntaxMode::Code => self.code(start, c),
            },

            None => SyntaxKind::End,
        };

        let text = self.s.from(start);
        let node = match self.error.take() {
            Some(error) => SyntaxNode::error(error, text),
            None => SyntaxNode::leaf(kind, text),
        };
        (kind, node)
    }

    /// Eat whitespace characters greedily.
    fn whitespace(&mut self, start: usize, c: char) -> SyntaxKind {
        let more = self.s.eat_while(|c| is_space(c, self.mode));
        let newlines = match c {
            // Optimize eating a single space.
            ' ' if more.is_empty() => 0,
            _ => count_newlines(self.s.from(start)),
        };

        self.newline = newlines > 0;
        if self.mode == SyntaxMode::Markup && newlines >= 2 {
            SyntaxKind::Parbreak
        } else {
            SyntaxKind::Space
        }
    }

    fn shebang(&mut self) -> SyntaxKind {
        self.s.eat_until(is_newline);
        SyntaxKind::Shebang
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
            'h' if self.s.eat_if("ttp://") => self.link(),
            'h' if self.s.eat_if("ttps://") => self.link(),
            '<' if self.s.at(is_id_continue) => self.label(),
            '@' if self.s.at(is_id_continue) => self.ref_marker(),

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
                if self.space_or_end() { SyntaxKind::HeadingMarker } else { self.text() }
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

    /// We parse entire raw segments in the lexer as a convenience to avoid
    /// going to and from the parser for each raw section. See comments in
    /// [`Self::blocky_raw`] and [`Self::inline_raw`] for specific details.
    fn raw(&mut self) -> (SyntaxKind, SyntaxNode) {
        let start = self.s.cursor() - 1;

        // Determine number of opening backticks.
        let mut backticks = 1;
        while self.s.eat_if('`') {
            backticks += 1;
        }

        // Special case for ``.
        if backticks == 2 {
            let nodes = vec![
                SyntaxNode::leaf(SyntaxKind::RawDelim, "`"),
                SyntaxNode::leaf(SyntaxKind::RawDelim, "`"),
            ];
            return (SyntaxKind::Raw, SyntaxNode::inner(SyntaxKind::Raw, nodes));
        }

        // Find end of raw text.
        let mut found = 0;
        while found < backticks {
            match self.s.eat() {
                Some('`') => found += 1,
                Some(_) => found = 0,
                None => {
                    let msg = SyntaxError::new("unclosed raw text");
                    let error = SyntaxNode::error(msg, self.s.from(start));
                    return (SyntaxKind::Error, error);
                }
            }
        }
        let end = self.s.cursor();

        let mut nodes = Vec::with_capacity(3); // Will have at least 3.

        // A closure for pushing a node onto our raw vector. Assumes the caller
        // will move the scanner to the next location at each step.
        let mut prev_start = start;
        let mut push_raw = |kind, s: &Scanner| {
            nodes.push(SyntaxNode::leaf(kind, s.from(prev_start)));
            prev_start = s.cursor();
        };

        // Opening delimiter.
        self.s.jump(start + backticks);
        push_raw(SyntaxKind::RawDelim, &self.s);

        if backticks >= 3 {
            self.blocky_raw(end - backticks, &mut push_raw);
        } else {
            self.inline_raw(end - backticks, &mut push_raw);
        }

        // Closing delimiter.
        self.s.jump(end);
        push_raw(SyntaxKind::RawDelim, &self.s);

        (SyntaxKind::Raw, SyntaxNode::inner(SyntaxKind::Raw, nodes))
    }

    /// Raw blocks parse a language tag, have smart behavior for trimming
    /// whitespace in the start/end lines, and trim common leading whitespace
    /// from all other lines as the "dedent". The exact behavior is described
    /// below.
    ///
    /// ### The initial line:
    /// - A valid Typst identifier immediately following the opening delimiter
    ///   is parsed as the language tag.
    /// - We check the rest of the line and if all characters are whitespace,
    ///   trim it. Otherwise we trim a single leading space if present.
    ///   - If more trimmed characters follow on future lines, they will be
    ///     merged into the same trimmed element.
    /// - If we didn't trim the entire line, the rest is kept as text.
    ///
    /// ### Inner lines:
    /// - We determine the "dedent" by iterating over the lines. The dedent is
    ///   the minimum number of leading whitespace characters (not bytes) before
    ///   each line that has any non-whitespace characters.
    ///   - The opening delimiter's line does not contribute to the dedent, but
    ///     the closing delimiter's line does (even if that line is entirely
    ///     whitespace up to the delimiter).
    /// - We then trim the newline and dedent characters of each line, and add a
    ///   (potentially empty) text element of all remaining characters.
    ///
    /// ### The final line:
    /// - If the last line is entirely whitespace, it is trimmed.
    /// - Otherwise its text is kept like an inner line. However, if the last
    ///   non-whitespace character of the final line is a backtick, then one
    ///   ascii space (if present) is trimmed from the end.
    fn blocky_raw<F>(&mut self, inner_end: usize, mut push_raw: F)
    where
        F: FnMut(SyntaxKind, &Scanner),
    {
        // Language tag.
        if self.s.eat_if(is_id_start) {
            self.s.eat_while(is_id_continue);
            push_raw(SyntaxKind::RawLang, &self.s);
        }

        // The rest of the function operates on the lines between the backticks.
        let mut lines = split_newlines(self.s.to(inner_end));

        // Determine dedent level.
        let dedent = lines
            .iter()
            .skip(1)
            .filter(|line| !line.chars().all(char::is_whitespace))
            // The line with the closing ``` is always taken into account
            .chain(lines.last())
            .map(|line| line.chars().take_while(|c| c.is_whitespace()).count())
            .min()
            .unwrap_or(0);

        // Trim whitespace from the last line. Will be added as a `RawTrimmed`
        // kind by the check for `self.s.cursor() != inner_end` below.
        if lines.last().is_some_and(|last| last.chars().all(char::is_whitespace)) {
            lines.pop();
        } else if let Some(last) = lines.last_mut() {
            // If last line ends in a backtick, try to trim a single space. This
            // check must happen before we add the first line since the last and
            // first lines might be the same.
            if last.trim_end().ends_with('`') {
                *last = last.strip_suffix(' ').unwrap_or(last);
            }
        }

        let mut lines = lines.into_iter();

        // Handle the first line: trim if all whitespace, or trim a single space
        // at the start. Note that the first line does not affect the dedent
        // value.
        if let Some(first_line) = lines.next() {
            if first_line.chars().all(char::is_whitespace) {
                self.s.advance(first_line.len());
                // This is the only spot we advance the scanner, but don't
                // immediately call `push_raw`. But the rest of the function
                // ensures we will always add this text to a `RawTrimmed` later.
                debug_assert!(self.s.cursor() != inner_end);
                // A proof by cases follows:
                // # First case: The loop runs
                // If the loop runs, there must be a newline following, so
                // `cursor != inner_end`. And if the loop runs, the first thing
                // it does is add a trimmed element.
                // # Second case: The final if-statement runs.
                // To _not_ reach the loop from here, we must have only one or
                // two lines:
                // 1. If one line, we cannot be here, because the first and last
                //    lines are the same, so this line will have been removed by
                //    the check for the last line being all whitespace.
                // 2. If two lines, the loop will run unless the last is fully
                //    whitespace, but if it is, it will have been popped, then
                //    the final if-statement will run because the text removed
                //    by the last line must include at least a newline, so
                //    `cursor != inner_end` here.
            } else {
                let line_end = self.s.cursor() + first_line.len();
                if self.s.eat_if(' ') {
                    // Trim a single space after the lang tag on the first line.
                    push_raw(SyntaxKind::RawTrimmed, &self.s);
                }
                // We know here that the rest of the line is non-empty.
                self.s.jump(line_end);
                push_raw(SyntaxKind::Text, &self.s);
            }
        }

        // Add lines.
        for line in lines {
            let offset: usize = line.chars().take(dedent).map(char::len_utf8).sum();
            self.s.eat_newline();
            self.s.advance(offset);
            push_raw(SyntaxKind::RawTrimmed, &self.s);
            self.s.advance(line.len() - offset);
            push_raw(SyntaxKind::Text, &self.s);
        }

        // Add final trimmed.
        if self.s.cursor() < inner_end {
            self.s.jump(inner_end);
            push_raw(SyntaxKind::RawTrimmed, &self.s);
        }
    }

    /// Inline raw text is split on lines with non-newlines as `Text` kinds and
    /// newlines as `RawTrimmed`. Inline raw text does not dedent the text, all
    /// non-newline whitespace is kept.
    fn inline_raw<F>(&mut self, inner_end: usize, mut push_raw: F)
    where
        F: FnMut(SyntaxKind, &Scanner),
    {
        while self.s.cursor() < inner_end {
            if self.s.at(is_newline) {
                push_raw(SyntaxKind::Text, &self.s);
                self.s.eat_newline();
                push_raw(SyntaxKind::RawTrimmed, &self.s);
                continue;
            }
            self.s.eat();
        }
        push_raw(SyntaxKind::Text, &self.s);
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
        if self.s.eat_if('.') && self.space_or_end() && read.parse::<u64>().is_ok() {
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
    fn math(&mut self, start: usize, c: char) -> (SyntaxKind, Option<SyntaxNode>) {
        let kind = match c {
            '\\' => self.backslash(),
            '"' => self.string(),

            '-' if self.s.eat_if(">>") => SyntaxKind::MathShorthand,
            '-' if self.s.eat_if('>') => SyntaxKind::MathShorthand,
            '-' if self.s.eat_if("->") => SyntaxKind::MathShorthand,
            ':' if self.s.eat_if('=') => SyntaxKind::MathShorthand,
            ':' if self.s.eat_if(":=") => SyntaxKind::MathShorthand,
            '!' if self.s.eat_if('=') => SyntaxKind::MathShorthand,
            '.' if self.s.eat_if("..") => SyntaxKind::MathShorthand,
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
            '|' if self.s.eat_if('|') => SyntaxKind::MathShorthand,
            '~' if self.s.eat_if("~>") => SyntaxKind::MathShorthand,
            '~' if self.s.eat_if('>') => SyntaxKind::MathShorthand,
            '*' | '-' | '~' => SyntaxKind::MathShorthand,

            '.' => SyntaxKind::Dot,
            ',' => SyntaxKind::Comma,
            ';' => SyntaxKind::Semicolon,

            '#' => SyntaxKind::Hash,
            '_' => SyntaxKind::Underscore,
            '$' => SyntaxKind::Dollar,
            '/' => SyntaxKind::Slash,
            '^' => SyntaxKind::Hat,
            '&' => SyntaxKind::MathAlignPoint,
            '√' | '∛' | '∜' => SyntaxKind::Root,
            '!' => SyntaxKind::Bang,

            '\'' => {
                self.s.eat_while('\'');
                SyntaxKind::MathPrimes
            }

            // We lex delimiters as `{Left,Right}{Brace,Paren}` and convert back
            // to `MathText` or `MathShorthand` in the parser.
            '(' => SyntaxKind::LeftParen,
            ')' => SyntaxKind::RightParen,
            '[' if self.s.eat_if('|') => SyntaxKind::LeftBrace,
            '|' if self.s.eat_if(']') => SyntaxKind::RightBrace,
            c if default_math_class(c) == Some(MathClass::Opening) => {
                SyntaxKind::LeftBrace
            }
            c if default_math_class(c) == Some(MathClass::Closing) => {
                SyntaxKind::RightBrace
            }

            // Identifiers.
            c if is_math_id_start(c) && self.s.at(is_math_id_continue) => {
                self.s.eat_while(is_math_id_continue);
                let (kind, node) = self.math_ident_or_field(start);
                return (kind, Some(node));
            }

            // Other math atoms.
            _ => self.math_text(start, c),
        };
        (kind, None)
    }

    /// Parse a single `MathIdent` or an entire `FieldAccess`.
    fn math_ident_or_field(&mut self, start: usize) -> (SyntaxKind, SyntaxNode) {
        let mut kind = SyntaxKind::MathIdent;
        let mut node = SyntaxNode::leaf(kind, self.s.from(start));
        while let Some(ident) = self.maybe_dot_ident() {
            kind = SyntaxKind::FieldAccess;
            let field_children = vec![
                node,
                SyntaxNode::leaf(SyntaxKind::Dot, '.'),
                SyntaxNode::leaf(SyntaxKind::Ident, ident),
            ];
            node = SyntaxNode::inner(kind, field_children);
        }
        (kind, node)
    }

    /// If at a dot and a math identifier, eat and return the identifier.
    fn maybe_dot_ident(&mut self) -> Option<&str> {
        if self.s.scout(1).is_some_and(is_math_id_start) && self.s.eat_if('.') {
            let ident_start = self.s.cursor();
            self.s.eat();
            self.s.eat_while(is_math_id_continue);
            Some(self.s.from(ident_start))
        } else {
            None
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
            SyntaxKind::MathText
        } else {
            let len = self
                .s
                .get(start..self.s.string().len())
                .graphemes(true)
                .next()
                .map_or(0, str::len);
            self.s.jump(start + len);
            if len > c.len_utf8() {
                // Grapheme clusters are treated as normal text and stay grouped
                // This may need to change in the future.
                SyntaxKind::Text
            } else {
                SyntaxKind::MathText
            }
        }
    }

    /// Handle named arguments in math function call.
    pub fn maybe_math_named_arg(&mut self, start: usize) -> Option<SyntaxNode> {
        let cursor = self.s.cursor();
        self.s.jump(start);
        if self.s.eat_if(is_id_start) {
            self.s.eat_while(is_id_continue);
            // Check that a colon directly follows the identifier, and not the
            // `:=` or `::=` math shorthands.
            if self.s.at(':') && !self.s.at(":=") && !self.s.at("::=") {
                // Check that the identifier is not just `_`.
                let node = if self.s.from(start) != "_" {
                    SyntaxNode::leaf(SyntaxKind::Ident, self.s.from(start))
                } else {
                    let msg = SyntaxError::new("expected identifier, found underscore");
                    SyntaxNode::error(msg, self.s.from(start))
                };
                return Some(node);
            }
        }
        self.s.jump(cursor);
        None
    }

    /// Handle spread arguments in math function call.
    pub fn maybe_math_spread_arg(&mut self, start: usize) -> Option<SyntaxNode> {
        let cursor = self.s.cursor();
        self.s.jump(start);
        if self.s.eat_if("..") {
            // Check that neither a space nor a dot follows the spread syntax.
            // A dot would clash with the `...` math shorthand.
            if !self.space_or_end() && !self.s.at('.') {
                let node = SyntaxNode::leaf(SyntaxKind::Dots, self.s.from(start));
                return Some(node);
            }
        }
        self.s.jump(cursor);
        None
    }
}

/// Code.
impl Lexer<'_> {
    fn code(&mut self, start: usize, c: char) -> SyntaxKind {
        match c {
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
        if (!prev.ends_with(['.', '@']) || prev.ends_with(".."))
            && let Some(keyword) = keyword(ident)
        {
            return keyword;
        }

        if ident == "_" { SyntaxKind::Underscore } else { SyntaxKind::Ident }
    }

    fn number(&mut self, start: usize, first_c: char) -> SyntaxKind {
        // Handle alternative integer bases.
        let base = match first_c {
            '0' if self.s.eat_if('b') => 2,
            '0' if self.s.eat_if('o') => 8,
            '0' if self.s.eat_if('x') => 16,
            _ => 10,
        };

        // Read the initial digits.
        if base == 16 {
            self.s.eat_while(char::is_ascii_alphanumeric);
        } else {
            self.s.eat_while(char::is_ascii_digit);
        }

        // Read floating point digits and exponents.
        let mut is_float = false;
        if base == 10 {
            // Read digits following a dot. Make sure not to confuse a spread
            // operator or a method call for the decimal separator.
            if first_c == '.' {
                is_float = true; // We already ate the trailing digits above.
            } else if !self.s.at("..")
                && !self.s.scout(1).is_some_and(is_id_start)
                && self.s.eat_if('.')
            {
                is_float = true;
                self.s.eat_while(char::is_ascii_digit);
            }

            // Read the exponent.
            if !self.s.at("em") && self.s.eat_if(['e', 'E']) {
                is_float = true;
                self.s.eat_if(['+', '-']);
                self.s.eat_while(char::is_ascii_digit);
            }
        }

        let number = self.s.from(start);
        let suffix = self.s.eat_while(|c: char| c.is_ascii_alphanumeric() || c == '%');

        let mut suffix_result = match suffix {
            "" => Ok(None),
            "pt" | "mm" | "cm" | "in" | "deg" | "rad" | "em" | "fr" | "%" => Ok(Some(())),
            _ => Err(eco_format!("invalid number suffix: {suffix}")),
        };

        let number_result = if is_float && number.parse::<f64>().is_err() {
            // The only invalid case should be when a float lacks digits after
            // the exponent: e.g. `1.2e`, `2.3E-`, or `1EM`.
            Err(eco_format!("invalid floating point number: {number}"))
        } else if base == 10 {
            Ok(())
        } else {
            let name = match base {
                2 => "binary",
                8 => "octal",
                16 => "hexadecimal",
                _ => unreachable!(),
            };
            // The index `[2..]` skips the leading `0b`/`0o`/`0x`.
            match i64::from_str_radix(&number[2..], base) {
                Ok(_) if suffix.is_empty() => Ok(()),
                Ok(value) => {
                    if suffix_result.is_ok() {
                        suffix_result = Err(eco_format!(
                            "try using a decimal number: {value}{suffix}"
                        ));
                    }
                    Err(eco_format!("{name} numbers cannot have a suffix"))
                }
                Err(_) => Err(eco_format!("invalid {name} number: {number}")),
            }
        };

        // Return our number or write an error with helpful hints.
        match (number_result, suffix_result) {
            // Valid numbers :D
            (Ok(()), Ok(None)) if is_float => SyntaxKind::Float,
            (Ok(()), Ok(None)) => SyntaxKind::Int,
            (Ok(()), Ok(Some(()))) => SyntaxKind::Numeric,
            // Invalid numbers :(
            (Err(number_err), Err(suffix_err)) => {
                let err = self.error(number_err);
                self.hint(suffix_err);
                err
            }
            (Ok(()), Err(msg)) | (Err(msg), Ok(_)) => self.error(msg),
        }
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
fn is_space(character: char, mode: SyntaxMode) -> bool {
    match mode {
        SyntaxMode::Markup => matches!(character, ' ' | '\t') || is_newline(character),
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
