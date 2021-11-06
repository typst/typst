use std::fmt::{self, Display, Formatter};
use std::mem;

use super::{TokenMode, Tokens};
use crate::syntax::{ErrorPos, Green, GreenData, GreenNode, NodeKind};
use crate::util::EcoString;

/// A convenient token-based parser.
pub struct Parser<'s> {
    /// An iterator over the source tokens.
    tokens: Tokens<'s>,
    /// Whether we are at the end of the file or of a group.
    eof: bool,
    /// The current token.
    current: Option<NodeKind>,
    /// The end byte index of the last non-trivia token.
    prev_end: usize,
    /// The start byte index of the peeked token.
    current_start: usize,
    /// The stack of open groups.
    groups: Vec<GroupEntry>,
    /// The children of the currently built node.
    children: Vec<Green>,
    /// Whether the last group was terminated.
    last_group_terminated: bool,
}

impl<'s> Parser<'s> {
    /// Create a new parser for the source string.
    pub fn new(src: &'s str) -> Self {
        let mut tokens = Tokens::new(src, TokenMode::Markup);
        let current = tokens.next();
        Self {
            tokens,
            eof: current.is_none(),
            current,
            prev_end: 0,
            current_start: 0,
            groups: vec![],
            children: vec![],
            last_group_terminated: true,
        }
    }

    /// End the parsing process and return the last child.
    pub fn finish(self) -> Vec<Green> {
        self.children
    }

    /// End the parsing process and return multiple children.
    pub fn eject(self) -> Option<Vec<Green>> {
        if self.eof() && self.group_success() {
            Some(self.children)
        } else {
            None
        }
    }

    /// Create a new marker.
    pub fn marker(&mut self) -> Marker {
        Marker(self.children.len())
    }

    /// Create a markup right before the trailing trivia.
    pub fn trivia_start(&self) -> Marker {
        let count = self
            .children
            .iter()
            .rev()
            .take_while(|node| self.is_trivia(node.kind()))
            .count();
        Marker(self.children.len() - count)
    }

    /// Perform a subparse that wraps its result in a node with the given kind.
    pub fn perform<F, T>(&mut self, kind: NodeKind, f: F) -> T
    where
        F: FnOnce(&mut Self) -> T,
    {
        let prev = mem::take(&mut self.children);
        let output = f(self);
        let until = self.trivia_start();
        let mut children = mem::replace(&mut self.children, prev);

        if self.tokens.mode() == TokenMode::Code {
            // Trailing trivia should not be wrapped into the new node.
            let idx = self.children.len();
            self.children.push(Green::default());
            self.children.extend(children.drain(until.0 ..));
            self.children[idx] = GreenNode::with_children(kind, children).into();
        } else {
            self.children.push(GreenNode::with_children(kind, children).into());
        }

        output
    }

    /// Whether the end of the source string or group is reached.
    pub fn eof(&self) -> bool {
        self.eof
    }

    /// Consume the current token and also trailing trivia.
    pub fn eat(&mut self) {
        self.prev_end = self.tokens.index();
        self.bump();

        if self.tokens.mode() == TokenMode::Code {
            // Skip whitespace and comments.
            while self.current.as_ref().map_or(false, |x| self.is_trivia(x)) {
                self.bump();
            }
        }

        self.repeek();
    }

    /// Eat if the current token it is the given one.
    pub fn eat_if(&mut self, t: &NodeKind) -> bool {
        let at = self.at(t);
        if at {
            self.eat();
        }
        at
    }

    /// Eat if the current token is the given one and produce an error if not.
    pub fn eat_expect(&mut self, t: &NodeKind) -> ParseResult {
        let eaten = self.eat_if(t);
        if !eaten {
            self.expected_at(t.as_str());
        }
        if eaten { Ok(()) } else { Err(ParseError) }
    }

    /// Eat, debug-asserting that the token is the given one.
    #[track_caller]
    pub fn eat_assert(&mut self, t: &NodeKind) {
        debug_assert_eq!(self.peek(), Some(t));
        self.eat();
    }

    /// Eat tokens while the condition is true.
    pub fn eat_while<F>(&mut self, mut f: F)
    where
        F: FnMut(&NodeKind) -> bool,
    {
        while self.peek().map_or(false, |t| f(t)) {
            self.eat();
        }
    }

    /// Eat the current token, but change its type.
    pub fn convert(&mut self, kind: NodeKind) {
        let marker = self.marker();
        self.eat();
        marker.convert(self, kind);
    }

    /// Whether the current token is of the given type.
    pub fn at(&self, kind: &NodeKind) -> bool {
        self.peek() == Some(kind)
    }

    /// Peek at the current token without consuming it.
    pub fn peek(&self) -> Option<&NodeKind> {
        if self.eof { None } else { self.current.as_ref() }
    }

    /// Peek at the current token, if it follows immediately after the last one
    /// without any trivia in between.
    pub fn peek_direct(&self) -> Option<&NodeKind> {
        if self.prev_end() == self.current_start() {
            self.peek()
        } else {
            None
        }
    }

    /// Peek at the source of the current token.
    pub fn peek_src(&self) -> &'s str {
        self.tokens.scanner().get(self.current_start() .. self.current_end())
    }

    /// The byte index at which the last non-trivia token ended.
    pub fn prev_end(&self) -> usize {
        self.prev_end
    }

    /// The byte index at which the current token starts.
    pub fn current_start(&self) -> usize {
        self.current_start
    }

    /// The byte index at which the current token ends.
    pub fn current_end(&self) -> usize {
        self.tokens.index()
    }

    /// Determine the column index for the given byte index.
    pub fn column(&self, index: usize) -> usize {
        self.tokens.scanner().column(index)
    }

    /// Set the tokenizer's mode.
    pub fn set_mode(&mut self, mode: TokenMode) {
        self.tokens.set_mode(mode);
    }

    /// Continue parsing in a group.
    ///
    /// When the end delimiter of the group is reached, all subsequent calls to
    /// `peek()` return `None`. Parsing can only continue with a matching call
    /// to `end_group`.
    ///
    /// This panics if the current token does not start the given group.
    #[track_caller]
    pub fn start_group(&mut self, kind: Group) {
        self.groups.push(GroupEntry { kind, prev_mode: self.tokens.mode() });
        self.tokens.set_mode(match kind {
            Group::Bracket => TokenMode::Markup,
            _ => TokenMode::Code,
        });

        self.repeek();
        match kind {
            Group::Paren => self.eat_assert(&NodeKind::LeftParen),
            Group::Bracket => self.eat_assert(&NodeKind::LeftBracket),
            Group::Brace => self.eat_assert(&NodeKind::LeftBrace),
            Group::Stmt => {}
            Group::Expr => {}
            Group::Imports => {}
        }
    }

    /// End the parsing of a group.
    ///
    /// This panics if no group was started.
    #[track_caller]
    pub fn end_group(&mut self) {
        let group_mode = self.tokens.mode();
        let group = self.groups.pop().expect("no started group");
        self.tokens.set_mode(group.prev_mode);
        self.repeek();
        self.last_group_terminated = true;

        let mut rescan = self.tokens.mode() != group_mode;

        // Eat the end delimiter if there is one.
        if let Some((end, required)) = match group.kind {
            Group::Paren => Some((NodeKind::RightParen, true)),
            Group::Bracket => Some((NodeKind::RightBracket, true)),
            Group::Brace => Some((NodeKind::RightBrace, true)),
            Group::Stmt => Some((NodeKind::Semicolon, false)),
            Group::Expr => None,
            Group::Imports => None,
        } {
            if self.current.as_ref() == Some(&end) {
                // Bump the delimeter and return. No need to rescan in this case.
                self.eat();
                rescan = false;
            } else if required {
                self.push_error(format_eco!("expected {}", end));
                self.last_group_terminated = false;
            }
        }

        // Rescan the peeked token if the mode changed.
        if rescan {
            if group_mode == TokenMode::Code {
                self.children.truncate(self.trivia_start().0);
            }

            self.tokens.jump(self.prev_end());
            self.prev_end = self.tokens.index();
            self.current_start = self.tokens.index();
            self.current = self.tokens.next();
            self.repeek();
        }
    }

    /// Check if the group processing was successfully terminated.
    pub fn group_success(&self) -> bool {
        self.last_group_terminated && self.groups.is_empty()
    }

    /// Low-level bump that consumes exactly one token without special trivia
    /// handling.
    fn bump(&mut self) {
        let kind = self.current.take().unwrap();
        let len = self.tokens.index() - self.current_start;
        self.children.push(GreenData::new(kind, len).into());
        self.current_start = self.tokens.index();
        self.current = self.tokens.next();
    }

    /// Take another look at the current token to recheck whether it ends a
    /// group.
    fn repeek(&mut self) {
        self.eof = match &self.current {
            Some(NodeKind::RightParen) => self.inside(Group::Paren),
            Some(NodeKind::RightBracket) => self.inside(Group::Bracket),
            Some(NodeKind::RightBrace) => self.inside(Group::Brace),
            Some(NodeKind::Semicolon) => self.inside(Group::Stmt),
            Some(NodeKind::From) => self.inside(Group::Imports),
            Some(NodeKind::Space(n)) => *n >= 1 && self.stop_at_newline(),
            Some(_) => false,
            None => true,
        };
    }

    /// Returns whether the given type can be skipped over.
    fn is_trivia(&self, token: &NodeKind) -> bool {
        Self::is_trivia_ext(token, self.stop_at_newline())
    }

    /// Returns whether the given type can be skipped over given the current
    /// newline mode.
    fn is_trivia_ext(token: &NodeKind, stop_at_newline: bool) -> bool {
        match token {
            NodeKind::Space(n) => *n == 0 || !stop_at_newline,
            NodeKind::LineComment => true,
            NodeKind::BlockComment => true,
            _ => false,
        }
    }

    /// Whether the active group must end at a newline.
    fn stop_at_newline(&self) -> bool {
        matches!(
            self.groups.last().map(|group| group.kind),
            Some(Group::Stmt | Group::Expr | Group::Imports)
        )
    }

    /// Whether we are inside the given group.
    fn inside(&self, kind: Group) -> bool {
        self.groups.iter().any(|g| g.kind == kind)
    }
}

/// Error handling.
impl Parser<'_> {
    /// Push an error into the children list.
    pub fn push_error(&mut self, msg: impl Into<EcoString>) {
        let error = NodeKind::Error(ErrorPos::Full, msg.into());
        self.children.push(GreenData::new(error, 0).into());
    }

    /// Eat the current token and add an error that it is unexpected.
    pub fn unexpected(&mut self) {
        match self.peek() {
            Some(found) => {
                let msg = format_eco!("unexpected {}", found);
                let error = NodeKind::Error(ErrorPos::Full, msg);
                self.perform(error, Self::eat);
            }
            None => self.push_error("unexpected end of file"),
        }
    }

    /// Eat the current token and add an error that it is not the expected `thing`.
    pub fn expected(&mut self, thing: &str) {
        match self.peek() {
            Some(found) => {
                let msg = format_eco!("expected {}, found {}", thing, found);
                let error = NodeKind::Error(ErrorPos::Full, msg);
                self.perform(error, Self::eat);
            }
            None => self.expected_at(thing),
        }
    }

    /// Add an error that the `thing` was expected at the end of the last
    /// non-trivia token.
    pub fn expected_at(&mut self, thing: &str) {
        self.trivia_start().expected(self, thing);
    }
}

/// A marker that indicates where a node may start.
#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub struct Marker(usize);

impl Marker {
    /// Perform a subparse that wraps all children after the marker in a node
    /// with the given kind.
    pub fn perform<T, F>(self, p: &mut Parser, kind: NodeKind, f: F) -> T
    where
        F: FnOnce(&mut Parser) -> T,
    {
        let success = f(p);
        self.end(p, kind);
        success
    }

    /// Wrap all children after the marker (excluding trailing trivia) in a node
    /// with the given `kind`.
    pub fn end(self, p: &mut Parser, kind: NodeKind) {
        let until = p.trivia_start();
        let children = p.children.drain(self.0 .. until.0).collect();
        p.children
            .insert(self.0, GreenNode::with_children(kind, children).into());
    }

    /// Wrap all children that do not fulfill the predicate in error nodes.
    pub fn filter_children<F>(self, p: &mut Parser, f: F)
    where
        F: Fn(&Green) -> Result<(), &'static str>,
    {
        for child in &mut p.children[self.0 ..] {
            if (p.tokens.mode() == TokenMode::Markup
                || !Parser::is_trivia_ext(child.kind(), false))
                && !child.kind().is_error()
            {
                if let Err(msg) = f(child) {
                    let error = NodeKind::Error(ErrorPos::Full, msg.into());
                    let inner = mem::take(child);
                    *child = GreenNode::with_child(error, inner).into();
                }
            }
        }
    }

    /// Insert an error message that `what` was expected at the marker position.
    pub fn expected(self, p: &mut Parser, what: &str) {
        let msg = format_eco!("expected {}", what);
        let error = NodeKind::Error(ErrorPos::Full, msg);
        p.children.insert(self.0, GreenData::new(error, 0).into());
    }

    /// Peek at the child directly after the marker.
    pub fn peek<'a>(self, p: &'a Parser) -> Option<&'a Green> {
        p.children.get(self.0)
    }

    /// Convert the child directly after marker.
    pub fn convert(self, p: &mut Parser, kind: NodeKind) {
        if let Some(child) = p.children.get_mut(self.0) {
            child.convert(kind);
        }
    }
}

/// A logical group of tokens, e.g. `[...]`.
struct GroupEntry {
    /// The kind of group this is. This decides which tokens will end the group.
    /// For example, a [`Group::Paren`] will be ended by
    /// [`Token::RightParen`].
    pub kind: Group,
    /// The mode the parser was in _before_ the group started (to which we go
    /// back once the group ends).
    pub prev_mode: TokenMode,
}

/// A group, confined by optional start and end delimiters.
#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum Group {
    /// A bracketed group: `[...]`.
    Bracket,
    /// A curly-braced group: `{...}`.
    Brace,
    /// A parenthesized group: `(...)`.
    Paren,
    /// A group ended by a semicolon or a line break: `;`, `\n`.
    Stmt,
    /// A group for a single expression, ended by a line break.
    Expr,
    /// A group for import items, ended by a semicolon, line break or `from`.
    Imports,
}

/// Allows parser methods to use the try operator. Never returned top-level
/// because the parser recovers from all errors.
pub type ParseResult<T = ()> = Result<T, ParseError>;

/// The error type for parsing.
#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub struct ParseError;

impl Display for ParseError {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        f.pad("failed to parse")
    }
}

impl std::error::Error for ParseError {}
