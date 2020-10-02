use std::fmt::{self, Debug, Formatter};

use super::{Scanner, TokenMode, Tokens};
use crate::diagnostic::Diagnostic;
use crate::syntax::{Decoration, Pos, Span, Spanned, Token};
use crate::Feedback;

/// A convenient token-based parser.
pub struct Parser<'s> {
    tokens: Tokens<'s>,
    peeked: Option<Spanned<Token<'s>>>,
    modes: Vec<TokenMode>,
    groups: Vec<(Pos, Group)>,
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
            f: Feedback::new(),
        }
    }

    /// Finish parsing and return the accumulated feedback.
    pub fn finish(self) -> Feedback {
        self.f
    }

    /// Add a diagnostic to the feedback.
    pub fn diag(&mut self, diag: Spanned<Diagnostic>) {
        self.f.diagnostics.push(diag);
    }

    /// Eat the next token and add a diagnostic that it was not the expected
    /// `thing`.
    pub fn diag_expected(&mut self, thing: &str) {
        if let Some(found) = self.eat() {
            self.diag(error!(
                found.span,
                "expected {}, found {}",
                thing,
                found.v.name(),
            ));
        } else {
            self.diag_expected_at(thing, self.pos());
        }
    }

    /// Add a diagnostic that the `thing` was expected at the given position.
    pub fn diag_expected_at(&mut self, thing: &str, pos: Pos) {
        self.diag(error!(pos, "expected {}", thing));
    }

    /// Add a diagnostic that the given `token` was unexpected.
    pub fn diag_unexpected(&mut self, token: Spanned<Token>) {
        self.diag(error!(token.span, "unexpected {}", token.v.name()));
    }

    /// Add a decoration to the feedback.
    pub fn deco(&mut self, deco: Spanned<Decoration>) {
        self.f.decorations.push(deco);
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
        let start = self.pos();
        match group {
            Group::Paren => self.eat_assert(Token::LeftParen),
            Group::Bracket => self.eat_assert(Token::LeftBracket),
            Group::Brace => self.eat_assert(Token::LeftBrace),
            Group::Subheader => {}
        }
        self.groups.push((start, group));
    }

    /// Ends the parsing of a group and returns the span of the whole group.
    ///
    /// # Panics
    /// This panics if no group was started.
    pub fn end_group(&mut self) -> Span {
        // Check that we are indeed at the end of the group.
        debug_assert_eq!(self.peek(), None, "unfinished group");

        let (start, group) = self.groups.pop().expect("unstarted group");
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
            if self.peeked.map(|s| s.v) == Some(token) {
                self.peeked = None;
            } else {
                self.diag(error!(self.pos(), "expected {}", token.name()));
            }
        }

        Span::new(start, self.pos())
    }

    /// Skip whitespace tokens.
    pub fn skip_white(&mut self) {
        self.eat_while(|t| {
            matches!(t, Token::Space(_) | Token::LineComment(_) | Token::BlockComment(_))
        });
    }

    /// Consume the next token.
    pub fn eat(&mut self) -> Option<Spanned<Token<'s>>> {
        self.peek()?;
        self.peeked.take()
    }

    /// Consume the next token if it is the given one.
    pub fn eat_if(&mut self, t: Token) -> Option<Spanned<Token<'s>>> {
        if self.peek()? == t { self.peeked.take() } else { None }
    }

    /// Consume the next token if the closure maps it a to `Some`-variant.
    pub fn eat_map<T>(
        &mut self,
        mut f: impl FnMut(Token<'s>) -> Option<T>,
    ) -> Option<Spanned<T>> {
        let token = self.peek()?;
        if let Some(t) = f(token) {
            self.peeked.take().map(|spanned| spanned.map(|_| t))
        } else {
            None
        }
    }

    /// Consume the next token, debug-asserting that it is the given one.
    pub fn eat_assert(&mut self, t: Token) {
        let next = self.eat();
        debug_assert_eq!(next.map(|s| s.v), Some(t));
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
            self.peeked = None;
            count += 1;
        }
        count
    }

    /// Peek at the next token without consuming it.
    pub fn peek(&mut self) -> Option<Token<'s>> {
        let token = match self.peeked {
            Some(token) => token.v,
            None => {
                let token = self.tokens.next()?;
                self.peeked = Some(token);
                token.v
            }
        };

        let group = match token {
            Token::RightParen => Group::Paren,
            Token::RightBracket => Group::Bracket,
            Token::RightBrace => Group::Brace,
            Token::Chain => Group::Subheader,
            _ => return Some(token),
        };

        if self.groups.iter().rev().any(|&(_, g)| g == group) {
            None
        } else {
            Some(token)
        }
    }

    /// Checks whether the next token fulfills a condition.
    ///
    /// Returns `false` if there is no next token.
    pub fn check(&mut self, f: impl FnMut(Token<'s>) -> bool) -> bool {
        self.peek().map(f).unwrap_or(false)
    }

    /// Whether the end of the source string or group is reached.
    pub fn eof(&mut self) -> bool {
        self.peek().is_none()
    }

    /// The position in the string at which the last token ends and next token
    /// will start.
    pub fn pos(&self) -> Pos {
        self.peeked.map(|s| s.span.start).unwrap_or_else(|| self.tokens.pos())
    }

    /// Jump to a position in the source string.
    pub fn jump(&mut self, pos: Pos) {
        self.tokens.jump(pos);
        self.peeked = None;
    }

    /// Returns the part of the source string that is spanned by the given span.
    pub fn get(&self, span: Span) -> &'s str {
        self.scanner().get(span.start.to_usize() .. span.end.to_usize())
    }

    /// The underlying scanner.
    pub fn scanner(&self) -> Scanner<'s> {
        self.tokens.scanner()
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
