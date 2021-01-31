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
    pub fn expected(&mut self, what: &str) {
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
            self.expected_at(what, self.next_start);
        }
    }

    /// Add a diagnostic that `what` was expected at the given position.
    pub fn expected_at(&mut self, what: &str, pos: Pos) {
        self.diag(error!(pos, "expected {}", what));
    }

    /// Eat the next token and add a diagnostic that it is unexpected.
    pub fn unexpected(&mut self) {
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

    /// Continue parsing in a group.
    ///
    /// When the end delimiter of the group is reached, all subsequent calls to
    /// `eat()` and `peek()` return `None`. Parsing can only continue with
    /// a matching call to `end_group`.
    ///
    /// # Panics
    /// This panics if the next token does not start the given group.
    pub fn start_group(&mut self, group: Group, mode: TokenMode) {
        self.modes.push(self.tokens.mode());
        self.tokens.set_mode(mode);

        self.groups.push(group);
        self.repeek();

        match group {
            Group::Paren => self.assert(&[Token::LeftParen]),
            Group::Bracket => self.assert(&[Token::HashBracket, Token::LeftBracket]),
            Group::Brace => self.assert(&[Token::LeftBrace]),
            Group::Subheader => {}
            Group::Stmt => {}
            Group::Expr => {}
        }
    }

    /// End the parsing of a group.
    ///
    /// # Panics
    /// This panics if no group was started.
    pub fn end_group(&mut self) {
        let prev_mode = self.tokens.mode();
        self.tokens.set_mode(self.modes.pop().expect("no pushed mode"));

        let group = self.groups.pop().expect("no started group");
        self.repeek();

        // Eat the end delimiter if there is one.
        if let Some((end, required)) = match group {
            Group::Paren => Some((Token::RightParen, true)),
            Group::Bracket => Some((Token::RightBracket, true)),
            Group::Brace => Some((Token::RightBrace, true)),
            Group::Subheader => None,
            Group::Stmt => Some((Token::Semicolon, false)),
            Group::Expr => None,
        } {
            if self.next == Some(end) {
                // Bump the delimeter and return. No need to rescan in this case.
                self.bump();
                return;
            } else if required {
                self.diag(error!(self.next_start, "expected {}", end.name()));
            }
        }

        // Rescan the peeked token if the mode changed.
        if self.tokens.mode() != prev_mode {
            self.tokens.jump(self.last_end);
            self.bump();
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
        self.span(f).transpose()
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

    /// Consume the next token if it is the given one and produce an error if
    /// not.
    pub fn expect(&mut self, t: Token) -> bool {
        let eaten = self.eat_if(t);
        if !eaten {
            self.expected_at(t.name(), self.last_end);
        }
        eaten
    }

    /// Consume the next token, debug-asserting that it is one of the given ones.
    pub fn assert(&mut self, ts: &[Token]) {
        let next = self.eat();
        debug_assert!(next.map_or(false, |n| ts.contains(&n)));
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

    /// Jump to a position in the source string.
    pub fn jump(&mut self, pos: Pos) {
        self.tokens.jump(pos);
        self.bump();
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

    /// Move to the next token, skipping whitespace and comments in code mode.
    fn bump(&mut self) {
        self.last_end = self.tokens.pos();
        self.next_start = self.tokens.pos();
        self.next = self.tokens.next();

        match self.tokens.mode() {
            TokenMode::Markup => {}
            TokenMode::Code => loop {
                match self.next {
                    Some(Token::Space(n)) if n < 1 || !self.in_line_group() => {}
                    Some(Token::LineComment(_)) => {}
                    Some(Token::BlockComment(_)) => {}
                    _ => break,
                }

                self.next_start = self.tokens.pos();
                self.next = self.tokens.next();
            },
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

        match token {
            Token::RightParen if self.groups.contains(&Group::Paren) => {}
            Token::RightBracket if self.groups.contains(&Group::Bracket) => {}
            Token::RightBrace if self.groups.contains(&Group::Brace) => {}
            Token::Semicolon if self.groups.contains(&Group::Stmt) => {}
            Token::Space(n) if n >= 1 && self.in_line_group() => {}
            Token::Pipe if self.groups.contains(&Group::Subheader) => {}
            _ => return,
        }

        self.peeked = None;
    }

    /// Whether the active group ends at a newline.
    fn in_line_group(&self) -> bool {
        matches!(self.groups.last(), Some(&Group::Stmt) | Some(&Group::Expr))
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
    /// A group ended by a semicolon or a line break: `;`, `\n`.
    Stmt,
    /// A group for a single expression. Not ended by something specific.
    Expr,
}
