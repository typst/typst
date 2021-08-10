use std::fmt::{self, Debug, Formatter};

use super::{TokenMode, Tokens};
use crate::diag::Error;
use crate::source::SourceFile;
use crate::syntax::{Pos, Span, Token};

/// A convenient token-based parser.
pub struct Parser<'s> {
    /// The id of the parsed file.
    source: &'s SourceFile,
    /// Parsing errors.
    errors: Vec<Error>,
    /// An iterator over the source tokens.
    tokens: Tokens<'s>,
    /// The stack of open groups.
    groups: Vec<GroupEntry>,
    /// The next token.
    next: Option<Token<'s>>,
    /// The peeked token.
    /// (Same as `next` except if we are at the end of group, then `None`).
    peeked: Option<Token<'s>>,
    /// The end position of the last (non-whitespace if in code mode) token.
    prev_end: Pos,
    /// The start position of the peeked token.
    next_start: Pos,
}

/// A logical group of tokens, e.g. `[...]`.
#[derive(Debug, Copy, Clone)]
struct GroupEntry {
    /// The start position of the group. Used by `Parser::end_group` to return
    /// The group's full span.
    pub start: Pos,
    /// The kind of group this is. This decides which tokens will end the group.
    /// For example, a [`Group::Paren`] will be ended by
    /// [`Token::RightParen`].
    pub kind: Group,
    /// The mode the parser was in _before_ the group started.
    pub outer_mode: TokenMode,
}

/// A group, confined by optional start and end delimiters.
#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum Group {
    /// A parenthesized group: `(...)`.
    Paren,
    /// A bracketed group: `[...]`.
    Bracket,
    /// A curly-braced group: `{...}`.
    Brace,
    /// A group ended by a semicolon or a line break: `;`, `\n`.
    Stmt,
    /// A group for a single expression, ended by a line break.
    Expr,
    /// A group for import items, ended by a semicolon, line break or `from`.
    Imports,
}

impl<'s> Parser<'s> {
    /// Create a new parser for the source string.
    pub fn new(source: &'s SourceFile) -> Self {
        let mut tokens = Tokens::new(source.src(), TokenMode::Markup);
        let next = tokens.next();
        Self {
            source,
            errors: vec![],
            tokens,
            groups: vec![],
            next,
            peeked: next,
            prev_end: Pos::ZERO,
            next_start: Pos::ZERO,
        }
    }

    /// Finish parsing and return all errors.
    pub fn finish(self) -> Vec<Error> {
        self.errors
    }

    /// Whether the end of the source string or group is reached.
    pub fn eof(&self) -> bool {
        self.peek().is_none()
    }

    /// Consume the next token.
    pub fn eat(&mut self) -> Option<Token<'s>> {
        let token = self.peek()?;
        self.bump();
        Some(token)
    }

    /// Eat the next token and return its source range.
    pub fn eat_span(&mut self) -> Span {
        let start = self.next_start();
        self.eat();
        Span::new(start, self.prev_end())
    }

    /// Consume the next token if it is the given one.
    pub fn eat_if(&mut self, t: Token) -> bool {
        if self.peek() == Some(t) {
            self.bump();
            true
        } else {
            false
        }
    }

    /// Consume the next token if the closure maps it a to `Some`-variant.
    pub fn eat_map<T, F>(&mut self, f: F) -> Option<T>
    where
        F: FnOnce(Token<'s>) -> Option<T>,
    {
        let token = self.peek()?;
        let mapped = f(token);
        if mapped.is_some() {
            self.bump();
        }
        mapped
    }

    /// Consume the next token if it is the given one and produce an error if
    /// not.
    pub fn eat_expect(&mut self, t: Token) -> bool {
        let eaten = self.eat_if(t);
        if !eaten {
            self.expected_at(self.prev_end(), t.name());
        }
        eaten
    }

    /// Consume the next token, debug-asserting that it is one of the given ones.
    pub fn eat_assert(&mut self, t: Token) {
        let next = self.eat();
        debug_assert_eq!(next, Some(t));
    }

    /// Consume tokens while the condition is true.
    pub fn eat_while<F>(&mut self, mut f: F)
    where
        F: FnMut(Token<'s>) -> bool,
    {
        while self.peek().map_or(false, |t| f(t)) {
            self.eat();
        }
    }

    /// Peek at the next token without consuming it.
    pub fn peek(&self) -> Option<Token<'s>> {
        self.peeked
    }

    /// Peek at the next token if it follows immediately after the last one
    /// without any whitespace in between.
    pub fn peek_direct(&self) -> Option<Token<'s>> {
        if self.next_start() == self.prev_end() {
            self.peeked
        } else {
            None
        }
    }

    /// Peek at the span of the next token.
    ///
    /// Has length zero if `peek()` returns `None`.
    pub fn peek_span(&self) -> Span {
        Span::new(self.next_start(), self.next_end())
    }

    /// Peek at the source of the next token.
    pub fn peek_src(&self) -> &'s str {
        self.get(self.peek_span())
    }

    /// Checks whether the next token fulfills a condition.
    ///
    /// Returns `false` if there is no next token.
    pub fn check<F>(&self, f: F) -> bool
    where
        F: FnOnce(Token<'s>) -> bool,
    {
        self.peek().map_or(false, f)
    }

    /// The byte position at which the last token ended.
    ///
    /// Refers to the end of the last _non-whitespace_ token in code mode.
    pub fn prev_end(&self) -> Pos {
        self.prev_end.into()
    }

    /// The byte position at which the next token starts.
    pub fn next_start(&self) -> Pos {
        self.next_start.into()
    }

    /// The byte position at which the next token will end.
    ///
    /// Is the same as [`next_start()`][Self::next_start] if `peek()` returns
    /// `None`.
    pub fn next_end(&self) -> Pos {
        self.tokens.index().into()
    }

    /// The span from `start` to [`self.prev_end()`](Self::prev_end).
    pub fn span_from(&self, start: Pos) -> Span {
        Span::new(start, self.prev_end())
    }

    /// Determine the column index for the given byte position.
    pub fn column(&self, pos: Pos) -> usize {
        self.source.pos_to_column(pos).unwrap()
    }

    /// Slice out part of the source string.
    pub fn get(&self, span: impl Into<Span>) -> &'s str {
        self.tokens.scanner().get(span.into().to_range())
    }

    /// Continue parsing in a group.
    ///
    /// When the end delimiter of the group is reached, all subsequent calls to
    /// `eat()` and `peek()` return `None`. Parsing can only continue with
    /// a matching call to `end_group`.
    ///
    /// This panics if the next token does not start the given group.
    pub fn start_group(&mut self, kind: Group, mode: TokenMode) {
        self.groups.push(GroupEntry {
            start: self.next_start(),
            kind,
            outer_mode: self.tokens.mode(),
        });

        self.tokens.set_mode(mode);
        self.repeek();

        match kind {
            Group::Paren => self.eat_assert(Token::LeftParen),
            Group::Bracket => self.eat_assert(Token::LeftBracket),
            Group::Brace => self.eat_assert(Token::LeftBrace),
            Group::Stmt => {}
            Group::Expr => {}
            Group::Imports => {}
        }
    }

    /// End the parsing of a group.
    ///
    /// This panics if no group was started.
    pub fn end_group(&mut self) -> Span {
        let prev_mode = self.tokens.mode();
        let group = self.groups.pop().expect("no started group");
        self.tokens.set_mode(group.outer_mode);
        self.repeek();

        let mut rescan = self.tokens.mode() != prev_mode;

        // Eat the end delimiter if there is one.
        if let Some((end, required)) = match group.kind {
            Group::Paren => Some((Token::RightParen, true)),
            Group::Bracket => Some((Token::RightBracket, true)),
            Group::Brace => Some((Token::RightBrace, true)),
            Group::Stmt => Some((Token::Semicolon, false)),
            Group::Expr => None,
            Group::Imports => None,
        } {
            if self.next == Some(end) {
                // Bump the delimeter and return. No need to rescan in this case.
                self.bump();
                rescan = false;
            } else if required {
                self.error(self.next_start(), format!("expected {}", end.name()));
            }
        }

        // Rescan the peeked token if the mode changed.
        if rescan {
            self.tokens.jump(self.prev_end().to_usize());
            self.bump();
        }

        Span::new(group.start, self.prev_end())
    }

    /// The tokenization mode outside of the current group.
    ///
    /// For example, this would be [`Markup`] if we are in a [`Code`] group that
    /// is embedded in a [`Markup`] group.
    ///
    /// [`Markup`]: TokenMode::Markup
    /// [`Code`]: TokenMode::Code
    pub fn outer_mode(&mut self) -> TokenMode {
        self.groups.last().map_or(TokenMode::Markup, |group| group.outer_mode)
    }

    /// Add an error with location and message.
    pub fn error(&mut self, span: impl Into<Span>, message: impl Into<String>) {
        self.errors.push(Error::new(self.source.id(), span, message));
    }

    /// Eat the next token and add an error that it is not the expected `thing`.
    pub fn expected(&mut self, what: &str) {
        let before = self.next_start();
        if let Some(found) = self.eat() {
            let after = self.prev_end();
            self.error(
                before .. after,
                format!("expected {}, found {}", what, found.name()),
            );
        } else {
            self.expected_at(self.next_start(), what);
        }
    }

    /// Add an error that `what` was expected at the given position.
    pub fn expected_at(&mut self, pos: Pos, what: &str) {
        self.error(pos, format!("expected {}", what));
    }

    /// Eat the next token and add an error that it is unexpected.
    pub fn unexpected(&mut self) {
        let before = self.next_start();
        if let Some(found) = self.eat() {
            let after = self.prev_end();
            self.error(before .. after, format!("unexpected {}", found.name()));
        }
    }

    /// Move to the next token.
    fn bump(&mut self) {
        self.prev_end = self.tokens.index().into();
        self.next_start = self.tokens.index().into();
        self.next = self.tokens.next();

        if self.tokens.mode() == TokenMode::Code {
            // Skip whitespace and comments.
            while match self.next {
                Some(Token::Space(n)) => n < 1 || !self.stop_at_newline(),
                Some(Token::LineComment(_)) => true,
                Some(Token::BlockComment(_)) => true,
                _ => false,
            } {
                self.next_start = self.tokens.index().into();
                self.next = self.tokens.next();
            }
        }

        self.repeek();
    }

    /// Take another look at the next token to recheck whether it ends a group.
    fn repeek(&mut self) {
        self.peeked = self.next;
        let token = match self.next {
            Some(token) => token,
            None => return,
        };

        if match token {
            Token::RightParen => self.inside(Group::Paren),
            Token::RightBracket => self.inside(Group::Bracket),
            Token::RightBrace => self.inside(Group::Brace),
            Token::Semicolon => self.inside(Group::Stmt),
            Token::From => self.inside(Group::Imports),
            Token::Space(n) => n >= 1 && self.stop_at_newline(),
            _ => false,
        } {
            self.peeked = None;
        }
    }

    /// Whether the active group ends at a newline.
    fn stop_at_newline(&self) -> bool {
        let active = self.groups.last().map(|group| group.kind);
        matches!(active, Some(Group::Stmt | Group::Expr | Group::Imports))
    }

    /// Whether we are inside the given group.
    fn inside(&self, kind: Group) -> bool {
        self.groups.iter().any(|g| g.kind == kind)
    }
}

impl Debug for Parser<'_> {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        let mut s = self.tokens.scanner();
        s.jump(self.next_start().to_usize());
        write!(f, "Parser({}|{})", s.eaten(), s.rest())
    }
}
