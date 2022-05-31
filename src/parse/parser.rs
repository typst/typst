use std::fmt::{self, Display, Formatter};
use std::mem;
use std::ops::Range;

use super::{TokenMode, Tokens};
use crate::diag::ErrorPos;
use crate::syntax::{InnerNode, NodeData, NodeKind, SyntaxNode};
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
    children: Vec<SyntaxNode>,
    /// Whether the last group was not correctly terminated.
    unterminated_group: bool,
    /// Whether a group terminator was found, that did not close a group.
    stray_terminator: bool,
}

impl<'s> Parser<'s> {
    /// Create a new parser for the source string.
    pub fn new(src: &'s str, mode: TokenMode) -> Self {
        Self::with_prefix("", src, mode)
    }

    /// Create a new parser for the source string that is prefixed by some text
    /// that does not need to be parsed but taken into account for column
    /// calculation.
    pub fn with_prefix(prefix: &str, src: &'s str, mode: TokenMode) -> Self {
        let mut tokens = Tokens::with_prefix(prefix, src, mode);
        let current = tokens.next();
        Self {
            tokens,
            eof: current.is_none(),
            current,
            prev_end: 0,
            current_start: 0,
            groups: vec![],
            children: vec![],
            unterminated_group: false,
            stray_terminator: false,
        }
    }

    /// End the parsing process and return the parsed children.
    pub fn finish(self) -> Vec<SyntaxNode> {
        self.children
    }

    /// End the parsing process and return the parsed children and whether the
    /// last token was terminated if all groups were terminated correctly or
    /// `None` otherwise.
    pub fn consume(self) -> Option<(Vec<SyntaxNode>, bool)> {
        self.terminated().then(|| (self.children, self.tokens.terminated()))
    }

    /// Create a new marker.
    pub fn marker(&mut self) -> Marker {
        Marker(self.children.len())
    }

    /// Create a marker right before the trailing trivia.
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
            self.children.push(SyntaxNode::default());
            self.children.extend(children.drain(until.0 ..));
            self.children[idx] = InnerNode::with_children(kind, children).into();
        } else {
            self.children.push(InnerNode::with_children(kind, children).into());
        }

        output
    }

    /// Whether the end of the source string or group is reached.
    pub fn eof(&self) -> bool {
        self.eof
    }

    /// Consume the current token and also trailing trivia.
    pub fn eat(&mut self) {
        self.stray_terminator |= match self.current {
            Some(NodeKind::RightParen) => !self.inside(Group::Paren),
            Some(NodeKind::RightBracket) => !self.inside(Group::Bracket),
            Some(NodeKind::RightBrace) => !self.inside(Group::Brace),
            _ => false,
        };

        self.prev_end = self.tokens.cursor();
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
    pub fn eat_if(&mut self, kind: NodeKind) -> bool {
        let at = self.at(kind);
        if at {
            self.eat();
        }
        at
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

    /// Eat if the current token is the given one and produce an error if not.
    pub fn expect(&mut self, kind: NodeKind) -> ParseResult {
        let at = self.peek() == Some(&kind);
        if at {
            self.eat();
            Ok(())
        } else {
            self.expected(kind.as_str());
            Err(ParseError)
        }
    }

    /// Eat, debug-asserting that the token is the given one.
    #[track_caller]
    pub fn assert(&mut self, kind: NodeKind) {
        debug_assert_eq!(self.peek(), Some(&kind));
        self.eat();
    }

    /// Whether the current token is of the given type.
    pub fn at(&self, kind: NodeKind) -> bool {
        self.peek() == Some(&kind)
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
        self.get(self.current_start() .. self.current_end())
    }

    /// Obtain a range of the source code.
    pub fn get(&self, range: Range<usize>) -> &'s str {
        self.tokens.scanner().get(range)
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
        self.tokens.cursor()
    }

    /// Determine the column index for the given byte index.
    pub fn column(&self, index: usize) -> usize {
        self.tokens.column(index)
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
            Group::Bracket | Group::Strong | Group::Emph => TokenMode::Markup,
            Group::Brace | Group::Paren | Group::Expr | Group::Imports => TokenMode::Code,
        });

        match kind {
            Group::Brace => self.assert(NodeKind::LeftBrace),
            Group::Bracket => self.assert(NodeKind::LeftBracket),
            Group::Paren => self.assert(NodeKind::LeftParen),
            Group::Strong => self.assert(NodeKind::Star),
            Group::Emph => self.assert(NodeKind::Underscore),
            Group::Expr => self.repeek(),
            Group::Imports => self.repeek(),
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

        let mut rescan = self.tokens.mode() != group_mode;

        // Eat the end delimiter if there is one.
        if let Some((end, required)) = match group.kind {
            Group::Paren => Some((NodeKind::RightParen, true)),
            Group::Bracket => Some((NodeKind::RightBracket, true)),
            Group::Brace => Some((NodeKind::RightBrace, true)),
            Group::Strong => Some((NodeKind::Star, true)),
            Group::Emph => Some((NodeKind::Underscore, true)),
            Group::Expr => Some((NodeKind::Semicolon, false)),
            Group::Imports => None,
        } {
            if self.current.as_ref() == Some(&end) {
                // If another group closes after a group with the missing terminator,
                // its scope of influence ends here and no longer taints the rest of the
                // reparse.
                self.unterminated_group = false;

                // Bump the delimeter and return. No need to rescan in this
                // case. Also, we know that the delimiter is not stray even
                // though we already removed the group.
                let s = self.stray_terminator;
                self.eat();
                self.stray_terminator = s;
                rescan = false;
            } else if required {
                self.expected(end.as_str());
                self.unterminated_group = true;
            }
        }

        // Rescan the peeked token if the mode changed.
        if rescan {
            let mut target = self.prev_end();
            if group_mode == TokenMode::Code {
                let start = self.trivia_start().0;
                target = self.current_start
                    - self.children[start ..].iter().map(SyntaxNode::len).sum::<usize>();
                self.children.truncate(start);
            }

            self.tokens.jump(target);
            self.prev_end = self.tokens.cursor();
            self.current_start = self.tokens.cursor();
            self.current = self.tokens.next();
        }

        self.repeek();
    }

    /// Checks if all groups were correctly terminated.
    fn terminated(&self) -> bool {
        self.groups.is_empty() && !self.unterminated_group && !self.stray_terminator
    }

    /// Low-level bump that consumes exactly one token without special trivia
    /// handling.
    fn bump(&mut self) {
        let kind = self.current.take().unwrap();
        let len = self.tokens.cursor() - self.current_start;
        self.children.push(NodeData::new(kind, len).into());
        self.current_start = self.tokens.cursor();
        self.current = self.tokens.next();
    }

    /// Take another look at the current token to recheck whether it ends a
    /// group.
    fn repeek(&mut self) {
        self.eof = match &self.current {
            Some(NodeKind::RightBrace) => self.inside(Group::Brace),
            Some(NodeKind::RightBracket) => self.inside(Group::Bracket),
            Some(NodeKind::RightParen) => self.inside(Group::Paren),
            Some(NodeKind::Star) => self.inside(Group::Strong),
            Some(NodeKind::Underscore) => self.inside(Group::Emph),
            Some(NodeKind::Semicolon) => self.inside(Group::Expr),
            Some(NodeKind::From) => self.inside(Group::Imports),
            Some(NodeKind::Space(n)) => self.space_ends_group(*n),
            Some(_) => false,
            None => true,
        };
    }

    /// Returns whether the given type can be skipped over.
    fn is_trivia(&self, token: &NodeKind) -> bool {
        match token {
            NodeKind::Space(n) => !self.space_ends_group(*n),
            NodeKind::LineComment => true,
            NodeKind::BlockComment => true,
            _ => false,
        }
    }

    /// Whether a space with the given number of newlines ends the current group.
    fn space_ends_group(&self, n: usize) -> bool {
        if n == 0 {
            return false;
        }

        match self.groups.last().map(|group| group.kind) {
            Some(Group::Strong | Group::Emph) => n >= 2,
            Some(Group::Imports) => n >= 1,
            Some(Group::Expr) if n >= 1 => {
                // Allow else and method call to continue on next line.
                self.groups.iter().nth_back(1).map(|group| group.kind)
                    != Some(Group::Brace)
                    || !matches!(
                        self.tokens.clone().next(),
                        Some(NodeKind::Else | NodeKind::Dot)
                    )
            }
            _ => false,
        }
    }

    /// Whether we are inside the given group (can be nested).
    fn inside(&self, kind: Group) -> bool {
        self.groups
            .iter()
            .rev()
            .take_while(|g| !kind.is_weak() || g.kind.is_weak())
            .any(|g| g.kind == kind)
    }
}

/// Error handling.
impl Parser<'_> {
    /// Eat the current token and add an error that it is unexpected.
    pub fn unexpected(&mut self) {
        if let Some(found) = self.peek() {
            let msg = format_eco!("unexpected {}", found);
            let error = NodeKind::Error(ErrorPos::Full, msg);
            self.perform(error, Self::eat);
        }
    }

    /// Add an error that the `thing` was expected at the end of the last
    /// non-trivia token.
    pub fn expected(&mut self, thing: &str) {
        self.expected_at(self.trivia_start(), thing);
    }

    /// Insert an error message that `what` was expected at the marker position.
    pub fn expected_at(&mut self, marker: Marker, what: &str) {
        let msg = format_eco!("expected {}", what);
        let error = NodeKind::Error(ErrorPos::Full, msg);
        self.children.insert(marker.0, NodeData::new(error, 0).into());
    }

    /// Eat the current token and add an error that it is not the expected
    /// `thing`.
    pub fn expected_found(&mut self, thing: &str) {
        match self.peek() {
            Some(found) => {
                let msg = format_eco!("expected {}, found {}", thing, found);
                let error = NodeKind::Error(ErrorPos::Full, msg);
                self.perform(error, Self::eat);
            }
            None => self.expected(thing),
        }
    }
}

/// Marks a location in a parser's child list.
#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub struct Marker(usize);

impl Marker {
    /// Peek at the child directly before the marker.
    pub fn before<'a>(self, p: &'a Parser) -> Option<&'a SyntaxNode> {
        p.children.get(self.0.checked_sub(1)?)
    }

    /// Peek at the child directly after the marker.
    pub fn after<'a>(self, p: &'a Parser) -> Option<&'a SyntaxNode> {
        p.children.get(self.0)
    }

    /// Convert the child directly after marker.
    pub fn convert(self, p: &mut Parser, kind: NodeKind) {
        if let Some(child) = p.children.get_mut(self.0) {
            child.convert(kind);
        }
    }

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
            .insert(self.0, InnerNode::with_children(kind, children).into());
    }

    /// Wrap all children that do not fulfill the predicate in error nodes.
    pub fn filter_children<F>(self, p: &mut Parser, mut f: F)
    where
        F: FnMut(&SyntaxNode) -> Result<(), &'static str>,
    {
        for child in &mut p.children[self.0 ..] {
            // Don't expose errors.
            if child.kind().is_error() {
                continue;
            }

            // Don't expose trivia in code.
            if p.tokens.mode() == TokenMode::Code && child.kind().is_trivia() {
                continue;
            }

            if let Err(msg) = f(child) {
                let mut msg = EcoString::from(msg);
                if msg.starts_with("expected") {
                    msg.push_str(", found ");
                    msg.push_str(child.kind().as_str());
                }
                let error = NodeKind::Error(ErrorPos::Full, msg);
                let inner = mem::take(child);
                *child = InnerNode::with_child(error, inner).into();
            }
        }
    }
}

/// A logical group of tokens, e.g. `[...]`.
#[derive(Debug)]
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
    /// A curly-braced group: `{...}`.
    Brace,
    /// A bracketed group: `[...]`.
    Bracket,
    /// A parenthesized group: `(...)`.
    Paren,
    /// A group surrounded with stars: `*...*`.
    Strong,
    /// A group surrounded with underscore: `_..._`.
    Emph,
    /// A group ended by a semicolon or a line break: `;`, `\n`.
    Expr,
    /// A group for import items, ended by a semicolon, line break or `from`.
    Imports,
}

impl Group {
    /// Whether the group can only force other weak groups to end.
    fn is_weak(self) -> bool {
        matches!(self, Group::Strong | Group::Emph)
    }
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
