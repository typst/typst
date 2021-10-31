use std::ops::Range;
use std::rc::Rc;

use super::{TokenMode, Tokens};
use crate::source::{SourceFile, SourceId};
use crate::syntax::{ErrorPosition, Green, GreenData, GreenNode, NodeKind};
use crate::util::EcoString;

/// A convenient token-based parser.
pub struct Parser<'s> {
    /// The parsed file.
    source: &'s SourceFile,
    /// An iterator over the source tokens.
    tokens: Tokens<'s>,
    /// The stack of open groups.
    groups: Vec<GroupEntry>,
    /// The next token.
    next: Option<NodeKind>,
    /// The peeked token.
    /// (Same as `next` except if we are at the end of group, then `None`).
    peeked: Option<NodeKind>,
    /// The end index of the last (non-whitespace if in code mode) token.
    prev_end: usize,
    /// The start index of the peeked token.
    next_start: usize,
    /// A stack of outer children vectors.
    stack: Vec<Vec<Green>>,
    /// The children of the currently built node.
    children: Vec<Green>,
    /// Whether the last parsing step was successful.
    success: bool,
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
        let mut tokens = Tokens::new(source, TokenMode::Markup);
        let next = tokens.next();
        Self {
            source,
            tokens,
            groups: vec![],
            next: next.clone(),
            peeked: next,
            prev_end: 0,
            next_start: 0,
            stack: vec![],
            children: vec![],
            success: true,
        }
    }

    /// The id of the parsed source file.
    pub fn id(&self) -> SourceId {
        self.source.id()
    }

    /// Start a nested node.
    ///
    /// Each start call has to be matched with a call to `end`,
    /// `end_with_custom_children`, `lift`, `abort`, or `end_or_abort`.
    pub fn start(&mut self) {
        self.stack.push(std::mem::take(&mut self.children));
    }

    /// Start a nested node, preserving a number of the current children.
    pub fn start_with(&mut self, preserve: usize) {
        let preserved = self.children.drain(self.children.len() - preserve ..).collect();
        self.stack.push(std::mem::replace(&mut self.children, preserved));
    }

    /// Filter the last children using the given predicate.
    pub fn filter_children<F, G>(&mut self, count: usize, f: F, error: G)
    where
        F: Fn(&Green) -> bool,
        G: Fn(&NodeKind) -> (ErrorPosition, EcoString),
    {
        for child in &mut self.children[count ..] {
            if !((self.tokens.mode() != TokenMode::Code
                || Self::skip_type_ext(child.kind(), false))
                || child.kind().is_error()
                || f(&child))
            {
                let (pos, msg) = error(child.kind());
                let inner = std::mem::take(child);
                *child =
                    GreenNode::with_child(NodeKind::Error(pos, msg), inner.len(), inner)
                        .into();
            }
        }
    }

    pub fn child(&self, child: usize) -> Option<&Green> {
        self.node_index_from_back(child).map(|i| &self.children[i])
    }

    fn node_index_from_back(&self, child: usize) -> Option<usize> {
        let len = self.children.len();
        let code = self.tokens.mode() == TokenMode::Code;
        let mut seen = 0;
        for x in (0 .. len).rev() {
            if self.skip_type(self.children[x].kind()) && code {
                continue;
            }
            if seen == child {
                return Some(x);
            }
            seen += 1;
        }

        None
    }

    /// End the current node as a node of given `kind`.
    pub fn end(&mut self, kind: NodeKind) {
        let outer = self.stack.pop().unwrap();
        let mut children = std::mem::replace(&mut self.children, outer);

        // have trailing whitespace continue to sit in self.children in code
        // mode.
        let mut remains = vec![];
        if self.tokens.mode() == TokenMode::Code {
            let len = children.len();
            for n in (0 .. len).rev() {
                if !self.skip_type(&children[n].kind()) {
                    break;
                }

                remains.push(children.pop().unwrap());
            }
            remains.reverse();
        }

        let len = children.iter().map(|c| c.len()).sum();
        self.children
            .push(GreenNode::with_children(kind, len, children).into());
        self.children.extend(remains);
        self.success = true;
    }

    /// End the current node as a node of given `kind`, and start a new node
    /// with the ended node as a first child. The function returns how many
    /// children the stack frame had before and how many were appended (accounts
    /// for trivia).
    pub fn end_and_start_with(&mut self, kind: NodeKind) -> (usize, usize) {
        let stack_offset = self.stack.last().unwrap().len();
        self.end(kind);
        let diff = self.children.len() - stack_offset;
        self.start_with(diff);
        (stack_offset, diff)
    }

    pub fn wrap(&mut self, index: usize, kind: NodeKind) {
        let index = self.node_index_from_back(index).unwrap();
        let child = std::mem::take(&mut self.children[index]);
        let item = GreenNode::with_child(kind, child.len(), child);
        self.children[index] = item.into();
    }

    pub fn convert(&mut self, kind: NodeKind) {
        self.start();
        self.eat();
        self.end(kind);
    }

    /// End the current node and undo its existence, inling all accumulated
    /// children into its parent.
    pub fn lift(&mut self) {
        let outer = self.stack.pop().unwrap();
        let children = std::mem::replace(&mut self.children, outer);
        self.children.extend(children);
        self.success = true;
    }

    /// End the current node and undo its existence, deleting all accumulated
    /// children.
    pub fn abort(&mut self, msg: impl Into<String>) {
        self.end(NodeKind::Error(ErrorPosition::Full, msg.into().into()));
        self.success = false;
    }

    pub fn may_lift_abort(&mut self) -> bool {
        if !self.success {
            self.lift();
            self.success = false;
            true
        } else {
            false
        }
    }

    pub fn may_end_abort(&mut self, kind: NodeKind) -> bool {
        if !self.success {
            self.end(kind);
            self.success = false;
            true
        } else {
            false
        }
    }

    /// End the current node as a node of given `kind` if the last parse was
    /// successful, otherwise, abort.
    pub fn end_or_abort(&mut self, kind: NodeKind) -> bool {
        if self.success {
            self.end(kind);
            true
        } else {
            self.may_end_abort(kind);
            false
        }
    }

    pub fn finish(&mut self) -> Rc<GreenNode> {
        match self.children.pop().unwrap() {
            Green::Node(n) => n,
            _ => panic!(),
        }
    }

    /// Whether the end of the source string or group is reached.
    pub fn eof(&self) -> bool {
        self.peek().is_none()
    }

    fn eat_peeked(&mut self) -> Option<NodeKind> {
        let token = self.peek()?.clone();
        self.eat();
        Some(token)
    }

    /// Consume the next token if it is the given one.
    pub fn eat_if(&mut self, t: &NodeKind) -> bool {
        if self.peek() == Some(t) {
            self.eat();
            true
        } else {
            false
        }
    }

    /// Consume the next token if the closure maps it a to `Some`-variant.
    pub fn eat_map<T, F>(&mut self, f: F) -> Option<T>
    where
        F: FnOnce(&NodeKind) -> Option<T>,
    {
        let token = self.peek()?;
        let mapped = f(token);
        if mapped.is_some() {
            self.eat();
        }
        mapped
    }

    /// Consume the next token if it is the given one and produce an error if
    /// not.
    pub fn eat_expect(&mut self, t: &NodeKind) -> bool {
        let eaten = self.eat_if(t);
        if !eaten {
            self.expected_at(t.as_str());
        }
        eaten
    }

    /// Consume the next token, debug-asserting that it is one of the given ones.
    pub fn eat_assert(&mut self, t: &NodeKind) {
        let next = self.eat_peeked();
        debug_assert_eq!(next.as_ref(), Some(t));
    }

    /// Consume tokens while the condition is true.
    pub fn eat_while<F>(&mut self, mut f: F)
    where
        F: FnMut(&NodeKind) -> bool,
    {
        while self.peek().map_or(false, |t| f(t)) {
            self.eat();
        }
    }

    /// Peek at the next token without consuming it.
    pub fn peek(&self) -> Option<&NodeKind> {
        self.peeked.as_ref()
    }

    /// Peek at the next token if it follows immediately after the last one
    /// without any whitespace in between.
    pub fn peek_direct(&self) -> Option<&NodeKind> {
        if self.next_start() == self.prev_end() {
            self.peeked.as_ref()
        } else {
            None
        }
    }

    /// Peek at the source of the next token.
    pub fn peek_src(&self) -> &'s str {
        self.get(self.next_start() .. self.next_end())
    }

    /// The byte index at which the last token ended.
    ///
    /// Refers to the end of the last _non-whitespace_ token in code mode.
    pub fn prev_end(&self) -> usize {
        self.prev_end
    }

    /// The byte index at which the next token starts.
    pub fn next_start(&self) -> usize {
        self.next_start
    }

    /// The byte index at which the next token will end.
    ///
    /// Is the same as [`next_start()`][Self::next_start] if `peek()` returns
    /// `None`.
    pub fn next_end(&self) -> usize {
        self.tokens.index()
    }

    /// Determine the column index for the given byte index.
    pub fn column(&self, index: usize) -> usize {
        self.source.byte_to_column(index).unwrap()
    }

    /// Slice out part of the source string.
    pub fn get(&self, range: Range<usize>) -> &'s str {
        self.source.get(range).unwrap()
    }

    /// Continue parsing in a group.
    ///
    /// When the end delimiter of the group is reached, all subsequent calls to
    /// `eat()` and `peek()` return `None`. Parsing can only continue with
    /// a matching call to `end_group`.
    ///
    /// This panics if the next token does not start the given group.
    pub fn start_group(&mut self, kind: Group, mode: TokenMode) {
        self.groups.push(GroupEntry { kind, prev_mode: self.tokens.mode() });

        self.tokens.set_mode(mode);
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
    pub fn end_group(&mut self) {
        let prev_mode = self.tokens.mode();
        let group = self.groups.pop().expect("no started group");
        self.tokens.set_mode(group.prev_mode);
        self.repeek();

        let mut rescan = self.tokens.mode() != prev_mode;

        // Eat the end delimiter if there is one.
        if let Some((end, required)) = match group.kind {
            Group::Paren => Some((NodeKind::RightParen, true)),
            Group::Bracket => Some((NodeKind::RightBracket, true)),
            Group::Brace => Some((NodeKind::RightBrace, true)),
            Group::Stmt => Some((NodeKind::Semicolon, false)),
            Group::Expr => None,
            Group::Imports => None,
        } {
            if self.next == Some(end.clone()) {
                // Bump the delimeter and return. No need to rescan in this case.
                self.eat();
                rescan = false;
            } else if required {
                self.start();
                self.abort(format!("expected {}", end));
            }
        }

        // Rescan the peeked token if the mode changed.
        if rescan {
            self.tokens.jump(self.prev_end());

            if prev_mode == TokenMode::Code {
                let len = self.children.len();
                for n in (0 .. len).rev() {
                    if !self.skip_type(self.children[n].kind()) {
                        break;
                    }

                    self.children.pop();
                }
            }

            self.fast_forward();
        }
    }

    /// Add an error that `what` was expected at the given span.
    pub fn expected_at(&mut self, what: &str) {
        let mut found = self.children.len();
        for (i, node) in self.children.iter().enumerate().rev() {
            if !self.skip_type(node.kind()) {
                break;
            }
            found = i;
        }

        self.expected_at_child(found, what);
    }

    /// Add an error that `what` was expected at the given child index.
    pub fn expected_at_child(&mut self, index: usize, what: &str) {
        self.children.insert(
            index,
            GreenData::new(
                NodeKind::Error(ErrorPosition::Full, format!("expected {}", what).into()),
                0,
            )
            .into(),
        );
    }

    /// Eat the next token and add an error that it is not the expected `thing`.
    pub fn expected(&mut self, what: &str) {
        self.start();
        match self.eat_peeked() {
            Some(found) => self.abort(format!("expected {}, found {}", what, found)),
            None => {
                self.lift();
                self.expected_at(what);
            }
        }
    }

    /// Eat the next token and add an error that it is unexpected.
    pub fn unexpected(&mut self) {
        self.start();
        match self.eat_peeked() {
            Some(found) => self.abort(format!("unexpected {}", found)),
            None => self.abort("unexpected end of file"),
        }
    }

    pub fn skip_type_ext(token: &NodeKind, stop_at_newline: bool) -> bool {
        match token {
            NodeKind::Space(n) => n < &1 || !stop_at_newline,
            NodeKind::LineComment => true,
            NodeKind::BlockComment => true,
            _ => false,
        }
    }

    fn skip_type(&self, token: &NodeKind) -> bool {
        Self::skip_type_ext(token, self.stop_at_newline())
    }

    /// Move to the next token.
    pub fn eat(&mut self) {
        self.children.push(
            GreenData::new(
                self.next.clone().unwrap(),
                self.tokens.index() - self.next_start,
            )
            .into(),
        );

        self.fast_forward();
    }

    pub fn fast_forward(&mut self) {
        if !self.next.as_ref().map_or(false, |x| self.skip_type(x)) {
            self.prev_end = self.tokens.index().into();
        }
        self.next_start = self.tokens.index().into();
        self.next = self.tokens.next();

        if self.tokens.mode() == TokenMode::Code {
            // Skip whitespace and comments.
            while self.next.as_ref().map_or(false, |x| self.skip_type(x)) {
                self.eat();
            }
        }

        self.repeek();
    }

    /// Take another look at the next token to recheck whether it ends a group.
    fn repeek(&mut self) {
        self.peeked = self.next.clone();
        let token = match self.next.as_ref() {
            Some(token) => token,
            None => return,
        };

        if match token {
            NodeKind::RightParen => self.inside(Group::Paren),
            NodeKind::RightBracket => self.inside(Group::Bracket),
            NodeKind::RightBrace => self.inside(Group::Brace),
            NodeKind::Semicolon => self.inside(Group::Stmt),
            NodeKind::From => self.inside(Group::Imports),
            NodeKind::Space(n) => n > &0 && self.stop_at_newline(),
            _ => false,
        } {
            self.peeked = None;
        }
    }

    /// Whether the active group ends at a newline.
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

    pub fn last_child(&self) -> Option<&Green> {
        self.children.last()
    }

    pub fn success(&mut self) -> bool {
        let s = self.success;
        self.success = true;
        s
    }

    pub fn unsuccessful(&mut self) {
        self.success = false;
    }

    pub fn child_count(&self) -> usize {
        self.children.len()
    }
}
