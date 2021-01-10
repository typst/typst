use std::fmt::{self, Debug, Formatter};

use super::{Scanner, TokenMode, Tokens};
use crate::diag::Diag;
use crate::diag::{Deco, Feedback};
use crate::syntax::{Pos, Span, Spanned, Token, WithSpan};

/// A convenient token-based parser.
pub struct Parser<'s> {
    /// An iterator over the source tokens.
    tokens: Tokens<'s>,
    /// The next token.
    next: Option<Token<'s>>,
    /// The peeked token.
    /// (Same as `next` except if we are at the end of group, then `None`).
    peeked: Option<Token<'s>>,
    /// The start position of the peeked token.
    next_start: Pos,
    /// The end position of the last (non-whitespace if in code mode) token.
    last_end: Pos,
    /// The stack of modes we were in.
    modes: Vec<TokenMode>,
    /// The stack of open groups.
    groups: Vec<Group>,
    /// Accumulated feedback.
    feedback: Feedback,
}

impl<'s> Parser<'s> {
    /// Create a new parser for the source string.
    pub fn new(src: &'s str) -> Self {
        let mut tokens = Tokens::new(src, TokenMode::Markup);
        let next = tokens.next();
        Self {
            tokens,
            next,
            peeked: next,
            next_start: Pos::ZERO,
            last_end: Pos::ZERO,
            modes: vec![],
            groups: vec![],
            feedback: Feedback::new(),
        }
    }

    /// Finish parsing and return the accumulated feedback.
    pub fn finish(self) -> Feedback {
        self.feedback
    }

    /// Add a diagnostic to the feedback.
    pub fn diag(&mut self, diag: Spanned<Diag>) {
        self.feedback.diags.push(diag);
    }

    /// Eat the next token and add a diagnostic that it is not the expected
    /// `thing`.
    pub fn diag_expected(&mut self, what: &str) {
        let before = self.next_start;
        if let Some(found) = self.eat() {
            let after = self.last_end;
            self.diag(error!(
                before .. after,
                "expected {}, found {}",
                what,
                found.name(),
            ));
        } else {
            self.diag_expected_at(what, self.next_start);
        }
    }

    /// Add a diagnostic that the `thing` was expected at the given position.
    pub fn diag_expected_at(&mut self, what: &str, pos: Pos) {
        self.diag(error!(pos, "expected {}", what));
    }

    /// Eat the next token and add a diagnostic that it is unexpected.
    pub fn diag_unexpected(&mut self) {
        let before = self.next_start;
        if let Some(found) = self.eat() {
            let after = self.last_end;
            self.diag(error!(before .. after, "unexpected {}", found.name()));
        }
    }

    /// Add a decoration to the feedback.
    pub fn deco(&mut self, deco: Spanned<Deco>) {
        self.feedback.decos.push(deco);
    }

    /// Update the token mode and push the previous mode onto a stack.
    pub fn push_mode(&mut self, mode: TokenMode) {
        self.modes.push(self.tokens.mode());
        self.tokens.set_mode(mode);
    }

    /// Pop the topmost token mode from the stack.
    ///
    /// # Panics
    /// This panics if there is no mode on the stack.
    pub fn pop_mode(&mut self) {
        self.tokens.set_mode(self.modes.pop().expect("no pushed mode"));
    }

    /// Continues parsing in a group.
    ///
    /// When the end delimiter of the group is reached, all subsequent calls to
    /// `eat()` and `peek()` return `None`. Parsing can only continue with
    /// a matching call to `end_group`.
    ///
    /// # Panics
    /// This panics if the next token does not start the given group.
    pub fn start_group(&mut self, group: Group) {
        match group {
            Group::Paren => self.eat_assert(Token::LeftParen),
            Group::Bracket => self.eat_assert(Token::LeftBracket),
            Group::Brace => self.eat_assert(Token::LeftBrace),
            Group::Subheader => {}
        }

        self.groups.push(group);
        self.repeek();
    }

    /// Ends the parsing of a group and returns the span of the whole group.
    ///
    /// # Panics
    /// This panics if no group was started.
    pub fn end_group(&mut self) {
        // Check that we are indeed at the end of the group.
        debug_assert_eq!(self.peek(), None, "unfinished group");

        let group = self.groups.pop().expect("no started group");
        self.repeek();

        let end = match group {
            Group::Paren => Some(Token::RightParen),
            Group::Bracket => Some(Token::RightBracket),
            Group::Brace => Some(Token::RightBrace),
            Group::Subheader => None,
        };

        if let Some(token) = end {
            if self.next == Some(token) {
                self.bump();
            } else {
                self.diag(error!(self.next_start, "expected {}", token.name()));
            }
        }
    }

    /// Execute `f` and return the result alongside the span of everything `f`
    /// ate. Excludes leading and trailing whitespace in code mode.
    pub fn span<T, F>(&mut self, f: F) -> Spanned<T>
    where
        F: FnOnce(&mut Self) -> T,
    {
        let start = self.next_start;
        let output = f(self);
        let end = self.last_end;
        output.with_span(start .. end)
    }

    /// A version of [`span`](Self::span) that works better with options.
    pub fn span_if<T, F>(&mut self, f: F) -> Option<Spanned<T>>
    where
        F: FnOnce(&mut Self) -> Option<T>,
    {
        self.span(|p| f(p)).transpose()
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

    /// Consume the next token, debug-asserting that it is the given one.
    pub fn eat_assert(&mut self, t: Token) {
        let next = self.eat();
        debug_assert_eq!(next, Some(t));
    }

    /// Peek at the next token without consuming it.
    pub fn peek(&self) -> Option<Token<'s>> {
        self.peeked
    }

    /// Peek at the span of the next token.
    ///
    /// Has length zero if `peek()` returns `None`.
    pub fn peek_span(&self) -> Span {
        Span::new(
            self.next_start,
            if self.eof() { self.next_start } else { self.tokens.pos() },
        )
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

    /// Whether the end of the source string or group is reached.
    pub fn eof(&self) -> bool {
        self.peek().is_none()
    }

    /// The position at which the next token starts.
    pub fn next_start(&self) -> Pos {
        self.next_start
    }

    /// The position at which the last token ended.
    ///
    /// Refers to the end of the last _non-whitespace_ token in code mode.
    pub fn last_end(&self) -> Pos {
        self.last_end
    }

    /// Slice a part out of the source string.
    pub fn get(&self, span: impl Into<Span>) -> &'s str {
        self.tokens.scanner().get(span.into().to_range())
    }

    /// The underlying scanner.
    pub fn scanner(&self) -> Scanner<'s> {
        let mut scanner = self.tokens.scanner().clone();
        scanner.jump(self.next_start.to_usize());
        scanner
    }

    fn bump(&mut self) {
        self.last_end = self.tokens.pos();
        self.next_start = self.tokens.pos();
        self.next = self.tokens.next();

        match self.tokens.mode() {
            TokenMode::Markup => {}
            TokenMode::Code => {
                while matches!(
                    self.next,
                    Some(Token::Space(_)) |
                    Some(Token::LineComment(_)) |
                    Some(Token::BlockComment(_))
                ) {
                    self.next_start = self.tokens.pos();
                    self.next = self.tokens.next();
                }
            }
        }

        self.repeek();
    }

    fn repeek(&mut self) {
        self.peeked = self.next;
        if self.groups.contains(&match self.next {
            Some(Token::RightParen) => Group::Paren,
            Some(Token::RightBracket) => Group::Bracket,
            Some(Token::RightBrace) => Group::Brace,
            Some(Token::Pipe) => Group::Subheader,
            _ => return,
        }) {
            self.peeked = None;
        }
    }
}

impl Debug for Parser<'_> {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        let s = self.scanner();
        write!(f, "Parser({}|{})", s.eaten(), s.rest())
    }
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
    /// A group ended by a chained subheader or a closing bracket:
    /// `... >>`, `...]`.
    Subheader,
}
