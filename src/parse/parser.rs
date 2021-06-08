use std::fmt::{self, Debug, Formatter};
use std::ops::Range;

use super::{search_column, TokenMode, Tokens};
use crate::diag::{Diag, DiagSet};
use crate::syntax::{Pos, Span, Token};

/// A convenient token-based parser.
pub struct Parser<'s> {
    /// Parsing diagnostics.
    pub diags: DiagSet,
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
    prev_end: usize,
    /// The start position of the peeked token.
    next_start: usize,
}

/// A logical group of tokens, e.g. `[...]`.
#[derive(Debug, Copy, Clone)]
struct GroupEntry {
    /// The start position of the group. Used by `Parser::end_group` to return
    /// The group's full span.
    pub start: usize,
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
}

impl<'s> Parser<'s> {
    /// Create a new parser for the source string.
    pub fn new(src: &'s str) -> Self {
        let mut tokens = Tokens::new(src, TokenMode::Markup);
        let next = tokens.next();
        Self {
            diags: DiagSet::new(),
            tokens,
            groups: vec![],
            next,
            peeked: next,
            prev_end: 0,
            next_start: 0,
        }
    }

    /// Add a diagnostic.
    pub fn diag(&mut self, diag: Diag) {
        self.diags.insert(diag);
    }

    /// Eat the next token and add a diagnostic that it is not the expected
    /// `thing`.
    pub fn expected(&mut self, what: &str) {
        let before = self.next_start();
        if let Some(found) = self.eat() {
            let after = self.prev_end();
            self.diag(error!(
                before .. after,
                "expected {}, found {}",
                what,
                found.name(),
            ));
        } else {
            self.expected_at(what, self.next_start());
        }
    }

    /// Add a diagnostic that `what` was expected at the given position.
    pub fn expected_at(&mut self, what: &str, pos: impl Into<Pos>) {
        self.diag(error!(pos.into(), "expected {}", what));
    }

    /// Eat the next token and add a diagnostic that it is unexpected.
    pub fn unexpected(&mut self) {
        let before = self.next_start();
        if let Some(found) = self.eat() {
            let after = self.prev_end();
            self.diag(error!(before .. after, "unexpected {}", found.name()));
        }
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
            Group::Paren => self.assert(Token::LeftParen),
            Group::Bracket => self.assert(Token::LeftBracket),
            Group::Brace => self.assert(Token::LeftBrace),
            Group::Stmt => {}
            Group::Expr => {}
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
        } {
            if self.next == Some(end) {
                // Bump the delimeter and return. No need to rescan in this case.
                self.bump();
                rescan = false;
            } else if required {
                self.diag(error!(self.next_start(), "expected {}", end.name()));
            }
        }

        // Rescan the peeked token if the mode changed.
        if rescan {
            self.jump(self.prev_end());
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

    /// Whether the end of the source string or group is reached.
    pub fn eof(&self) -> bool {
        self.peek().is_none()
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
        self.peek_range().into()
    }

    /// Peek at the source of the next token.
    pub fn peek_src(&self) -> &'s str {
        self.tokens.scanner().get(self.peek_range())
    }

    /// Peek at the source range (start and end index) of the next token.
    pub fn peek_range(&self) -> Range<usize> {
        self.next_start() .. self.next_end()
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

    /// Consume the next token.
    pub fn eat(&mut self) -> Option<Token<'s>> {
        let token = self.peek()?;
        self.bump();
        Some(token)
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

    /// Eat the next token and return its source range.
    pub fn eat_span(&mut self) -> Span {
        let start = self.next_start();
        self.eat();
        Span::new(start, self.prev_end())
    }

    /// Consume the next token if it is the given one and produce a diagnostic
    /// if not.
    pub fn expect(&mut self, t: Token) -> bool {
        let eaten = self.eat_if(t);
        if !eaten {
            self.expected_at(t.name(), self.prev_end());
        }
        eaten
    }

    /// Consume the next token, debug-asserting that it is one of the given ones.
    pub fn assert(&mut self, t: Token) {
        let next = self.eat();
        debug_assert_eq!(next, Some(t));
    }

    /// Skip whitespace and comment tokens.
    pub fn skip_white(&mut self) {
        while matches!(
            self.peek(),
            Some(Token::Space(_)) |
            Some(Token::LineComment(_)) |
            Some(Token::BlockComment(_))
        ) {
            self.eat();
        }
    }

    /// The index at which the last token ended.
    ///
    /// Refers to the end of the last _non-whitespace_ token in code mode.
    pub fn prev_end(&self) -> usize {
        self.prev_end
    }

    /// The index at which the next token starts.
    pub fn next_start(&self) -> usize {
        self.next_start
    }

    /// The index at which the next token will end.
    ///
    /// Is the same as [`next_start()`][Self::next_start] if `peek()` returns
    /// `None`.
    pub fn next_end(&self) -> usize {
        self.tokens.index()
    }

    /// Determine the column for the given index in the source.
    pub fn column(&self, index: usize) -> usize {
        search_column(self.tokens.scanner().get(.. index))
    }

    /// The span from `start` to [`self.prev_end()`](Self::prev_end).
    pub fn span(&self, start: impl Into<Pos>) -> Span {
        Span::new(start, self.prev_end())
    }

    /// Jump to an index in the string.
    ///
    /// You need to know the correct column.
    fn jump(&mut self, index: usize) {
        self.tokens.jump(index);
        self.bump();
    }

    /// Move to the next token.
    fn bump(&mut self) {
        self.prev_end = self.tokens.index();
        self.next_start = self.tokens.index();
        self.next = self.tokens.next();

        if self.tokens.mode() == TokenMode::Code {
            // Skip whitespace and comments.
            while match self.next {
                Some(Token::Space(n)) => n < 1 || !self.stop_at_newline(),
                Some(Token::LineComment(_)) => true,
                Some(Token::BlockComment(_)) => true,
                _ => false,
            } {
                self.next_start = self.tokens.index();
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
            Token::Space(n) => n >= 1 && self.stop_at_newline(),
            _ => false,
        } {
            self.peeked = None;
        }
    }

    /// Whether the active group ends at a newline.
    fn stop_at_newline(&self) -> bool {
        let active = self.groups.last().map(|group| group.kind);
        matches!(active, Some(Group::Stmt) | Some(Group::Expr))
    }

    /// Whether we are inside the given group.
    fn inside(&self, kind: Group) -> bool {
        self.groups.iter().any(|g| g.kind == kind)
    }
}

impl Debug for Parser<'_> {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        let mut s = self.tokens.scanner();
        s.jump(self.next_start());
        write!(f, "Parser({}|{})", s.eaten(), s.rest())
    }
}
