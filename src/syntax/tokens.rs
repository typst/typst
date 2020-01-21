use std::iter::Peekable;
use std::str::Chars;
use unicode_xid::UnicodeXID;

use super::*;
use Token::*;
use TokenizationMode::*;


/// A minimal semantic entity of source code.
#[derive(Debug, Copy, Clone, PartialEq)]
pub enum Token<'s> {
    /// One or more whitespace characters. The contained `usize` denotes the
    /// number of newlines that were contained in the whitespace.
    Space(usize),

    /// A line comment with inner string contents `//<&'s str>\n`.
    LineComment(&'s str),
    /// A block comment with inner string contents `/*<&'s str>*/`. The comment
    /// can contain nested block comments.
    BlockComment(&'s str),

    /// A function invocation `[<header>][<body>]`.
    Function {
        header: &'s str,
        body: Option<(Position, &'s str)>,
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
    /// A comma in a function header: `:`.
    Comma,
    /// An equals sign in a function header: `=`.
    Equals,

    /// An identifier in a function header: `center`.
    ExprIdent(&'s str),
    /// A quoted string in a function header: `"..."`.
    ExprStr { string: &'s str, terminated: bool },
    /// A number in a function header: `3.14`.
    ExprNumber(f64),
    /// A size in a function header: `12pt`.
    ExprSize(Size),
    /// A boolean in a function header: `true | false`.
    ExprBool(bool),

    /// A star in body-text.
    Star,
    /// An underscore in body-text.
    Underscore,
    /// A backtick in body-text.
    Backtick,

    /// Any other consecutive string.
    Text(&'s str),

    /// Things that are not valid in the context they appeared in.
    Invalid(&'s str),
}

/// An iterator over the tokens of a string of source code.
pub struct Tokens<'s> {
    src: &'s str,
    mode: TokenizationMode,
    iter: Peekable<Chars<'s>>,
    position: Position,
    index: usize,
}

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum TokenizationMode {
    Header,
    Body,
}

impl<'s> Tokens<'s> {
    pub fn new(start: Position, src: &'s str, mode: TokenizationMode) -> Tokens<'s> {
        Tokens {
            src,
            mode,
            iter: src.chars().peekable(),
            position: start,
            index: 0,
        }
    }

    /// The index in the string at which the last token ends and next token will
    /// start.
    pub fn index(&self) -> usize {
        self.index
    }

    /// The line-colunn position in the source at which the last token ends and
    /// next token will start. This position is
    pub fn pos(&self) -> Position {
        self.position
    }
}

impl<'s> Iterator for Tokens<'s> {
    type Item = Spanned<Token<'s>>;

    /// Parse the next token in the source code.
    fn next(&mut self) -> Option<Spanned<Token<'s>>> {
        let start = self.pos();
        let first = self.eat()?;

        let token = match first {
            // Comments.
            '/' if self.peek() == Some('/') => self.parse_line_comment(),
            '/' if self.peek() == Some('*') => self.parse_block_comment(),
            '*' if self.peek() == Some('/') => { self.eat(); Invalid("*/") }

            // Whitespace.
            c if c.is_whitespace() => self.parse_whitespace(start),

            // Functions.
            '[' => self.parse_function(start),
            ']' => Invalid("]"),

            // Syntactic elements in function headers.
            '(' if self.mode == Header => LeftParen,
            ')' if self.mode == Header => RightParen,
            '{' if self.mode == Header => LeftBrace,
            '}' if self.mode == Header => RightBrace,
            ':' if self.mode == Header => Colon,
            ',' if self.mode == Header => Comma,
            '=' if self.mode == Header => Equals,

            // String values.
            '"' if self.mode == Header => self.parse_string(),

            // Style toggles.
            '*' if self.mode == Body => Star,
            '_' if self.mode == Body => Underscore,
            '`' if self.mode == Body => Backtick,

            // An escaped thing.
            '\\' => self.parse_escaped(),

            // Expressions or just strings.
            c => {
                let text = self.read_string_until(|n| {
                    match n {
                        c if c.is_whitespace() => true,
                        '\\' | '[' | ']' | '*' | '_' | '`' | ':' | '=' |
                        ',' | '"' | '/' => true,
                        _ => false,
                    }
                }, false, -(c.len_utf8() as isize), 0).0;

                if self.mode == Header {
                    self.parse_expr(text)
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
    fn parse_line_comment(&mut self) -> Token<'s> {
        LineComment(self.read_string_until(is_newline_char, false, 1, 0).0)
    }

    fn parse_block_comment(&mut self) -> Token<'s> {
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

    fn parse_whitespace(&mut self, start: Position) -> Token<'s> {
        self.read_string_until(|n| !n.is_whitespace(), false, 0, 0);
        let end = self.pos();

        Space(end.line - start.line)
    }

    fn parse_function(&mut self, start: Position) -> Token<'s> {
        let (header, terminated) = self.read_function_part();
        self.eat();

        if self.peek() != Some('[') {
            return Function { header, body: None, terminated };
        }

        self.eat();

        let offset = self.pos() - start;
        let (body, terminated) = self.read_function_part();
        self.eat();

        Function { header, body: Some((offset, body)), terminated }
    }

    fn read_function_part(&mut self) -> (&'s str, bool) {
        let mut escaped = false;
        let mut in_string = false;
        let mut depth = 0;

        self.read_string_until(|n| {
            match n {
                '"' if !escaped => in_string = !in_string,
                '[' if !escaped && !in_string => depth += 1,
                ']' if !escaped && !in_string => {
                    if depth == 0 {
                        return true;
                    } else {
                        depth -= 1;
                    }
                }
                '\\' => escaped = !escaped,
                _ => escaped = false,
            }

            false
        }, false, 0, 0)
    }

    fn parse_string(&mut self) -> Token<'s> {
        let mut escaped = false;
        let (string, terminated) = self.read_string_until(|n| {
            match n {
                '"' if !escaped => return true,
                '\\' => escaped = !escaped,
                _ => escaped = false,
            }

            false
        }, true, 0, -1);
        ExprStr { string, terminated }
    }

    fn parse_escaped(&mut self) -> Token<'s> {
        fn is_escapable(c: char) -> bool {
            match c {
                '\\' | '[' | ']' | '*' | '_' | '`' | '/' => true,
                _ => false,
            }
        }

        let c = self.peek().unwrap_or('n');
        if self.mode == Body && is_escapable(c) {
            let index = self.index();
            self.eat();
            Text(&self.src[index .. index + c.len_utf8()])
        } else {
            Text("\\")
        }
    }

    fn parse_expr(&mut self, text: &'s str) -> Token<'s> {
        if let Ok(b) = text.parse::<bool>() {
            ExprBool(b)
        } else if let Ok(num) = text.parse::<f64>() {
            ExprNumber(num)
        } else if let Some(num) = parse_percentage(text) {
            ExprNumber(num / 100.0)
        } else if let Ok(size) = text.parse::<Size>() {
            ExprSize(size)
        } else if is_identifier(text) {
            ExprIdent(text)
        } else {
            Invalid(text)
        }
    }

    fn read_string_until<F>(
        &mut self,
        mut f: F,
        eat_match: bool,
        offset_start: isize,
        offset_end: isize,
    ) -> (&'s str, bool) where F: FnMut(char) -> bool {
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

        (&self.src[start .. end], matched)
    }

    fn eat(&mut self) -> Option<char> {
        let c = self.iter.next()?;
        let len = c.len_utf8();

        self.index += len;

        if is_newline_char(c) && !(c == '\r' && self.peek() == Some('\n')) {
            self.position.line += 1;
            self.position.column = 0;
        } else {
            self.position.column += len;
        }

        Some(c)
    }

    fn peek(&mut self) -> Option<char> {
        self.iter.peek().copied()
    }
}

fn parse_percentage(text: &str) -> Option<f64> {
    if text.ends_with('%') {
        text[.. text.len() - 1].parse::<f64>().ok()
    } else {
        None
    }
}

/// Whether this character denotes a newline.
pub fn is_newline_char(character: char) -> bool {
    match character {
        // Line Feed, Vertical Tab, Form Feed, Carriage Return.
        '\x0A' ..= '\x0D' => true,
        // Next Line, Line Separator, Paragraph Separator.
        '\u{0085}' | '\u{2028}' | '\u{2029}' => true,
        _ => false,
    }
}

/// Whether this word is a valid identifier.
pub fn is_identifier(string: &str) -> bool {
    let mut chars = string.chars();

    match chars.next() {
        Some('-') => {}
        Some(c) if UnicodeXID::is_xid_start(c) => {}
        _ => return false,
    }

    while let Some(c) = chars.next() {
        match c {
            '.' | '-' => {}
            c if UnicodeXID::is_xid_continue(c) => {}
            _ => return false,
        }
    }

    true
}
