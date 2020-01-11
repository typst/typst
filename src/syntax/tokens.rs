use std::iter::Peekable;
use std::str::Chars;

use super::*;
use Token::*;
use State::*;


pub fn tokenize(src: &str) -> Tokens {
    Tokens::new(src)
}

/// A minimal semantic entity of source code.
#[derive(Debug, Clone, PartialEq)]
pub enum Token<'s> {
    /// One or more whitespace characters. The contained `usize` denotes the
    /// number of newlines that were contained in the whitespace.
    Whitespace(usize),

    /// A line comment with inner string contents `//<&'s str>\n`.
    LineComment(&'s str),
    /// A block comment with inner string contents `/*<&'s str>*/`. The comment
    /// can contain nested block comments.
    BlockComment(&'s str),
    /// An erroneous `*/` without an opening block comment.
    StarSlash,

    /// A left bracket: `[`.
    LeftBracket,
    /// A right bracket: `]`.
    RightBracket,

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

    /// An expression in a function header.
    Expr(Expression),

    /// A star in body-text.
    Star,
    /// An underscore in body-text.
    Underscore,
    /// A backtick in body-text.
    Backtick,

    /// Any other consecutive string.
    Text(&'s str),
}

/// An iterator over the tokens of a string of source code.
pub struct Tokens<'s> {
    src: &'s str,
    chars: Characters<'s>,
    state: State,
    stack: Vec<State>,
}

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
enum State {
    Header,
    StartBody,
    Body,
}

impl<'s> Tokens<'s> {
    pub fn new(src: &'s str) -> Tokens<'s> {
        Tokens {
            src,
            chars: Characters::new(src),
            state: State::Body,
            stack: vec![],
        }
    }
}

impl<'s> Iterator for Tokens<'s> {
    type Item = Spanned<Token<'s>>;

    /// Parse the next token in the source code.
    fn next(&mut self) -> Option<Spanned<Token<'s>>> {
        let start = self.chars.position();
        let first = self.chars.next()?;
        let second = self.chars.peek();

        let token = match first {
            // Comments.
            '/' if second == Some('/') => self.parse_line_comment(),
            '/' if second == Some('*') => self.parse_block_comment(),
            '*' if second == Some('/') => { self.eat(); StarSlash }

            // Whitespace.
            c if c.is_whitespace() => self.parse_whitespace(start),

            // Functions.
            '[' => { self.set_state(Header); LeftBracket }
            ']' => {
                if self.state == Header && second == Some('[') {
                    self.state = StartBody;
                } else {
                    self.pop_state();
                }

                RightBracket
            }

            // Syntactic elements in function headers.
            '(' if self.state == Header => LeftParen,
            ')' if self.state == Header => RightParen,
            '{' if self.state == Header => LeftBrace,
            '}' if self.state == Header => RightBrace,
            ':' if self.state == Header => Colon,
            ',' if self.state == Header => Comma,
            '=' if self.state == Header => Equals,

            // String values.
            '"' if self.state == Header => self.parse_string(),

            // Style toggles.
            '*' if self.state == Body => Star,
            '_' if self.state == Body => Underscore,
            '`' if self.state == Body => Backtick,

            // An escaped thing.
            '\\' => self.parse_escaped(),

            // Expressions or just strings.
            c => {
                let word = self.read_string_until(|n| {
                    match n {
                        c if c.is_whitespace() => true,
                        '\\' | '[' | ']' | '*' | '_' | '`' | ':' | '=' |
                        ',' | '"' | '/' => true,
                        _ => false,
                    }
                }, false, -(c.len_utf8() as isize), 0);

                if self.state == Header {
                    self.parse_expr(word)
                } else {
                    Text(word)
                }
            }
        };

        let end = self.chars.position();
        let span = Span { start, end };

        Some(Spanned { v: token, span })
    }
}

impl<'s> Tokens<'s> {
    fn parse_line_comment(&mut self) -> Token<'s> {
        LineComment(self.read_string_until(is_newline_char, false, 1, 0))
    }

    fn parse_block_comment(&mut self) -> Token<'s> {
        enum Last { Slash, Star, Other }
        use Last::*;

        self.eat();

        let mut depth = 0;
        let mut last = Last::Other;

        // Find the first `*/` that does not correspond to a nested `/*`.
        // Remove the last two bytes to obtain the raw inner text without `*/`.
        BlockComment(self.read_string_until(|n| {
            match n {
                '/' => match last {
                    Star if depth == 0 => return true,
                    Star => depth -= 1,
                    _ => last = Slash
                }
                '*' => match last {
                    Slash => depth += 1,
                    _ => last = Star,
                }
                _ => last = Other,
            }

            false
        }, true, 0, -2))
    }

    fn parse_whitespace(&mut self, start: Position) -> Token<'s> {
        self.read_string_until(|n| !n.is_whitespace(), false, 0, 0);
        let end = self.chars.position();

        Whitespace(end.line - start.line)
    }

    fn parse_string(&mut self) -> Token<'s> {
        let mut escaped = false;
        Expr(Expression::Str(self.read_string_until(|n| {
            if n == '"' && !escaped {
                return true;
            } else if n == '\\' {
                escaped = !escaped;
            } else {
                escaped = false;
            }

            false
        }, true, 0, -1).to_string()))
    }

    fn parse_escaped(&mut self) -> Token<'s> {
        fn is_escapable(c: char) -> bool {
            match c {
                '\\' | '[' | ']' | '*' | '_' | '`' | '/' => true,
                _ => false,
            }
        }

        let c = self.chars.peek().unwrap_or('n');
        if self.state == Body && is_escapable(c) {
            let index = self.chars.index();
            self.eat();
            Text(&self.src[index .. index + c.len_utf8()])
        } else {
            Text("\\")
        }
    }

    fn parse_expr(&mut self, word: &'s str) -> Token<'s> {
        if let Ok(b) = word.parse::<bool>() {
            Expr(Expression::Bool(b))
        } else if let Ok(num) = word.parse::<f64>() {
            Expr(Expression::Num(num))
        } else if let Ok(num) = parse_percentage(word) {
            Expr(Expression::Num(num / 100.0))
        } else if let Ok(size) = word.parse::<Size>() {
            Expr(Expression::Size(size))
        } else if let Some(ident) = Ident::new(word) {
            Expr(Expression::Ident(ident))
        } else {
            Text(word)
        }
    }

    fn read_string_until<F>(
        &mut self,
        mut f: F,
        eat_match: bool,
        offset_start: isize,
        offset_end: isize,
    ) -> &'s str where F: FnMut(char) -> bool {
        let start = ((self.chars.index() as isize) + offset_start) as usize;
        let mut matched = false;

        while let Some(c) = self.chars.peek() {
            if f(c) {
                matched = true;
                if eat_match {
                    self.chars.next();
                }
                break;
            }

            self.chars.next();
        }

        let mut end = self.chars.index();
        if matched {
            end = ((end as isize) + offset_end) as usize;
        }

        &self.src[start .. end]
    }

    fn set_state(&mut self, state: State) {
        self.stack.push(self.state);
        self.state = state;
    }

    fn pop_state(&mut self) {
        self.state = self.stack.pop().unwrap_or(Body);
    }

    fn eat(&mut self) {
        self.chars.next();
    }
}

fn parse_percentage(word: &str) -> Result<f64, ()> {
    if word.ends_with('%') {
        word[.. word.len() - 1].parse::<f64>().map_err(|_| ())
    } else {
        Err(())
    }
}

/// Whether this character denotes a newline.
fn is_newline_char(character: char) -> bool {
    match character {
        // Line Feed, Vertical Tab, Form Feed, Carriage Return.
        '\x0A' ..= '\x0D' => true,
        // Next Line, Line Separator, Paragraph Separator.
        '\u{0085}' | '\u{2028}' | '\u{2029}' => true,
        _ => false,
    }
}

struct Characters<'s> {
    iter: Peekable<Chars<'s>>,
    position: Position,
    index: usize,
}

impl<'s> Characters<'s> {
    fn new(src: &'s str) -> Characters<'s> {
        Characters {
            iter: src.chars().peekable(),
            position: Position::new(0, 0),
            index: 0,
        }
    }

    fn next(&mut self) -> Option<char> {
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

    fn index(&self) -> usize {
        self.index
    }

    fn position(&self) -> Position {
        self.position
    }
}
