use std::fmt::{self, Debug, Formatter};

use super::{Scanner, TokenMode, Tokens};
use crate::diag::Diag;
use crate::diag::{Deco, Feedback};
use crate::syntax::{Pos, Span, SpanWith, Spanned, Token};

/// A convenient token-based parser.
pub struct Parser<'s> {
    tokens: Tokens<'s>,
    peeked: Option<Token<'s>>,
    modes: Vec<TokenMode>,
    groups: Vec<Group>,
    pos: Pos,
    f: Feedback,
}

impl<'s> Parser<'s> {
    /// Create a new parser for the source string.
    pub fn new(src: &'s str) -> Self {
        Self {
            tokens: Tokens::new(src, TokenMode::Body),
            peeked: None,
            modes: vec![],
            groups: vec![],
            pos: Pos::ZERO,
            f: Feedback::new(),
        }
    }

    /// Finish parsing and return the accumulated feedback.
    pub fn finish(self) -> Feedback {
        self.f
    }

    /// Add a diagnostic to the feedback.
    pub fn diag(&mut self, diag: Spanned<Diag>) {
        self.f.diags.push(diag);
    }

    /// Eat the next token and add a diagnostic that it is not the expected
    /// `thing`.
    pub fn diag_expected(&mut self, what: &str) {
        let before = self.pos();
        if let Some(found) = self.eat() {
            let after = self.pos();
            self.diag(error!(
                before .. after,
                "expected {}, found {}",
                what,
                found.name(),
            ));
        } else {
            self.diag_expected_at(what, self.pos());
        }
    }

    /// Add a diagnostic that the `thing` was expected at the given position.
    pub fn diag_expected_at(&mut self, what: &str, pos: Pos) {
        self.diag(error!(pos, "expected {}", what));
    }

    /// Eat the next token and add a diagnostic that it is unexpected.
    pub fn diag_unexpected(&mut self) {
        let before = self.pos();
        if let Some(found) = self.eat() {
            let after = self.pos();
            self.diag(match found {
                Token::Invalid(_) => error!(before .. after, "invalid token"),
                _ => error!(before .. after, "unexpected {}", found.name()),
            });
        }
    }

    /// Add a decoration to the feedback.
    pub fn deco(&mut self, deco: Spanned<Deco>) {
        self.f.decos.push(deco);
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
    }

    /// Ends the parsing of a group and returns the span of the whole group.
    ///
    /// # Panics
    /// This panics if no group was started.
    pub fn end_group(&mut self) {
        // Check that we are indeed at the end of the group.
        debug_assert_eq!(self.peek(), None, "unfinished group");

        let group = self.groups.pop().expect("no started group");
        let end = match group {
            Group::Paren => Some(Token::RightParen),
            Group::Bracket => Some(Token::RightBracket),
            Group::Brace => Some(Token::RightBrace),
            Group::Subheader => None,
        };

        if let Some(token) = end {
            // This `peek()` can't be used directly because it hides the end of
            // group token. To circumvent this, we drop down to `self.peeked`.
            self.peek();
            if self.peeked == Some(token) {
                self.bump();
            } else {
                self.diag(error!(self.pos(), "expected {}", token.name()));
            }
        }
    }

    /// Skip whitespace tokens.
    pub fn skip_white(&mut self) {
        self.eat_while(|t| {
            matches!(t, Token::Space(_) | Token::LineComment(_) | Token::BlockComment(_))
        });
    }

    /// Execute `f` and return the result alongside the span of everything `f`
    /// ate.
    pub fn span<T>(&mut self, f: impl FnOnce(&mut Self) -> T) -> Spanned<T> {
        let start = self.pos;
        f(self).span_with(start .. self.pos)
    }

    /// Consume the next token.
    pub fn eat(&mut self) -> Option<Token<'s>> {
        self.peek()?;
        self.bump()
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
    pub fn eat_map<T>(&mut self, f: impl FnOnce(Token<'s>) -> Option<T>) -> Option<T> {
        let token = self.peek()?;
        let out = f(token);
        if out.is_some() {
            self.bump();
        }
        out
    }

    /// Consume the next token, debug-asserting that it is the given one.
    pub fn eat_assert(&mut self, t: Token) {
        let next = self.eat();
        debug_assert_eq!(next, Some(t));
    }

    /// Consume tokens while the condition is true.
    ///
    /// Returns how many tokens were eaten.
    pub fn eat_while(&mut self, mut f: impl FnMut(Token<'s>) -> bool) -> usize {
        self.eat_until(|t| !f(t))
    }

    /// Consume tokens until the condition is true.
    ///
    /// Returns how many tokens were eaten.
    pub fn eat_until(&mut self, mut f: impl FnMut(Token<'s>) -> bool) -> usize {
        let mut count = 0;
        while let Some(t) = self.peek() {
            if f(t) {
                break;
            }
            self.bump();
            count += 1;
        }
        count
    }

    /// Peek at the next token without consuming it.
    pub fn peek(&mut self) -> Option<Token<'s>> {
        let token = match self.peeked {
            Some(token) => token,
            None => {
                let token = self.tokens.next()?;
                self.peeked = Some(token);
                token
            }
        };

        let group = match token {
            Token::RightParen => Group::Paren,
            Token::RightBracket => Group::Bracket,
            Token::RightBrace => Group::Brace,
            Token::Pipe => Group::Subheader,
            _ => return Some(token),
        };

        if self.groups.contains(&group) {
            None
        } else {
            Some(token)
        }
    }

    /// Checks whether the next token fulfills a condition.
    ///
    /// Returns `false` if there is no next token.
    pub fn check(&mut self, f: impl FnOnce(Token<'s>) -> bool) -> bool {
        self.peek().map_or(false, f)
    }

    /// Whether the end of the source string or group is reached.
    pub fn eof(&mut self) -> bool {
        self.peek().is_none()
    }

    /// The position in the string at which the last token ends and next token
    /// will start.
    pub fn pos(&self) -> Pos {
        self.pos
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

    /// The full source string up to the current index.
    pub fn eaten(&self) -> &'s str {
        self.tokens.scanner().get(.. self.pos.to_usize())
    }

    /// The source string from `start` to the current index.
    pub fn eaten_from(&self, start: Pos) -> &'s str {
        self.tokens.scanner().get(start.to_usize() .. self.pos.to_usize())
    }

    /// The remaining source string after the current index.
    pub fn rest(&self) -> &'s str {
        self.tokens.scanner().get(self.pos.to_usize() ..)
    }

    /// The underlying scanner.
    pub fn scanner(&self) -> Scanner<'s> {
        let mut scanner = self.tokens.scanner().clone();
        scanner.jump(self.pos.to_usize());
        scanner
    }

    /// Set the position to the tokenizer's position and take the peeked token.
    fn bump(&mut self) -> Option<Token<'s>> {
        self.pos = self.tokens.pos();
        let token = self.peeked;
        self.peeked = None;
        token
    }
}

impl Debug for Parser<'_> {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        write!(f, "Parser({}|{})", self.eaten(), self.rest())
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
