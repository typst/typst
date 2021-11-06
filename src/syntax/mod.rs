//! Syntax types.

pub mod ast;
mod highlight;
mod pretty;
mod span;

use std::fmt::{self, Debug, Display, Formatter};
use std::rc::Rc;

pub use highlight::*;
pub use pretty::*;
pub use span::*;

use self::ast::{MathNode, RawNode, TypedNode};
use crate::diag::Error;
use crate::geom::{AngularUnit, LengthUnit};
use crate::parse::{
    parse_atomic, parse_code, parse_markup, parse_markup_elements, TokenMode,
};
use crate::source::SourceId;
use crate::util::EcoString;

/// An inner or leaf node in the untyped green tree.
#[derive(Clone, PartialEq)]
pub enum Green {
    /// A reference-counted inner node.
    Node(Rc<GreenNode>),
    /// A terminal, owned token.
    Token(GreenData),
}

impl Green {
    /// Returns the metadata of the node.
    fn data(&self) -> &GreenData {
        match self {
            Green::Node(n) => &n.data,
            Green::Token(t) => t,
        }
    }

    /// The type of the node.
    pub fn kind(&self) -> &NodeKind {
        self.data().kind()
    }

    /// The length of the node.
    pub fn len(&self) -> usize {
        self.data().len()
    }

    /// Whether the node or its children contain an error.
    pub fn erroneous(&self) -> bool {
        match self {
            Self::Node(node) => node.erroneous,
            Self::Token(data) => data.kind.is_error(),
        }
    }

    /// The node's children.
    pub fn children(&self) -> &[Green] {
        match self {
            Green::Node(n) => n.children(),
            Green::Token(_) => &[],
        }
    }

    /// Change the type of the node.
    pub fn convert(&mut self, kind: NodeKind) {
        match self {
            Self::Node(node) => {
                let node = Rc::make_mut(node);
                node.erroneous |= kind.is_error();
                node.data.kind = kind;
            }
            Self::Token(data) => data.kind = kind,
        }
    }

    /// Find the innermost child that is incremental safe.
    pub fn incremental_int(
        &mut self,
        edit: &str,
        replace: Span,
        replacement_len: usize,
        offset: usize,
        parent_mode: TokenMode,
        outermost: bool,
    ) -> bool {
        match self {
            Green::Node(n) => Rc::make_mut(n).incremental_int(
                edit,
                replace,
                replacement_len,
                offset,
                parent_mode,
                outermost,
            ),
            Green::Token(_) => false,
        }
    }

    /// The error messages for this node and its descendants.
    pub fn errors(&self) -> Vec<NodeKind> {
        match self {
            Green::Node(n) => n.errors(),
            Green::Token(t) => {
                if t.kind().is_error() {
                    vec![t.kind().clone()]
                } else {
                    vec![]
                }
            }
        }
    }
}

impl Default for Green {
    fn default() -> Self {
        Self::Token(GreenData::new(NodeKind::None, 0))
    }
}

impl Debug for Green {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        match self {
            Self::Node(node) => node.fmt(f),
            Self::Token(token) => token.fmt(f),
        }
    }
}

/// An inner node in the untyped green tree.
#[derive(Clone, PartialEq)]
pub struct GreenNode {
    /// Node metadata.
    data: GreenData,
    /// This node's children, losslessly make up this node.
    children: Vec<Green>,
    /// Whether this node or any of its children are erroneous.
    pub erroneous: bool,
}

impl GreenNode {
    /// Creates a new node with the given kind and a single child.
    pub fn with_child(kind: NodeKind, child: impl Into<Green>) -> Self {
        Self::with_children(kind, vec![child.into()])
    }

    /// Creates a new node with the given kind and children.
    pub fn with_children(kind: NodeKind, children: Vec<Green>) -> Self {
        let mut erroneous = kind.is_error();
        let len = children
            .iter()
            .inspect(|c| erroneous |= c.erroneous())
            .map(Green::len)
            .sum();

        Self {
            data: GreenData::new(kind, len),
            children,
            erroneous,
        }
    }

    /// The node's children.
    pub fn children(&self) -> &[Green] {
        &self.children
    }

    /// The node's children, mutably.
    pub fn children_mut(&mut self) -> &mut [Green] {
        &mut self.children
    }

    /// The node's metadata.
    pub fn data(&self) -> &GreenData {
        &self.data
    }

    /// The node's type.
    pub fn kind(&self) -> &NodeKind {
        self.data().kind()
    }

    /// The node's length.
    pub fn len(&self) -> usize {
        self.data().len()
    }

    /// The error messages for this node and its descendants.
    pub fn errors(&self) -> Vec<NodeKind> {
        let mut res = self.children.iter().flat_map(|c| c.errors()).collect::<Vec<_>>();

        if self.kind().is_error() {
            res.push(self.kind().clone());
        }

        res
    }

    /// Find the innermost child that is incremental safe.
    pub fn incremental(
        &mut self,
        edit: &str,
        replace: Span,
        replacement_len: usize,
    ) -> bool {
        self.incremental_int(edit, replace, replacement_len, 0, TokenMode::Markup, true)
    }

    fn incremental_int(
        &mut self,
        src: &str,
        replace: Span,
        replacement_len: usize,
        mut offset: usize,
        parent_mode: TokenMode,
        outermost: bool,
    ) -> bool {
        let kind = self.kind().clone();
        let mode = kind.mode().apply(parent_mode);
        eprintln!("in {:?} (mode {:?})", kind, mode);

        let mut loop_result = None;
        let mut child_at_start = true;
        let last = self.children.len() - 1;
        for (i, child) in self.children.iter_mut().enumerate() {
            let child_span = Span::new(replace.source, offset, offset + child.len());
            if child_span.surrounds(replace) {
                eprintln!("found correct child");

                // First, we try if the child has another, more specific applicable child.
                if kind.incremental_safety() != IncrementalSafety::Unsafe
                    && child.incremental_int(
                        src,
                        replace,
                        replacement_len,
                        offset,
                        mode,
                        i == last && outermost,
                    )
                {
                    eprintln!("child was successful");
                    return true;
                }

                // This didn't work, so we try to replace the child at this
                // level.
                let (function, policy) =
                    if let Some(p) = child.kind().reparsing_function(mode) {
                        p
                    } else {
                        return false;
                    };
                loop_result = Some((i, child_span, function, policy));
                break;
            }

            offset += child.len();
            child_at_start = child.kind().is_at_start(child_at_start);
        }


        // We now have a child that we can replace and a function to do so if
        // the loop found any results at all.
        let (child_idx, child_span, func, policy) = if let Some(loop_result) = loop_result
        {
            loop_result
        } else {
            // No child fully contains the replacement.
            eprintln!("no child match");
            return false;
        };

        eprintln!("aquired function, policy {:?}", policy);

        let src_span = child_span.inserted(replace, replacement_len);

        let new_children =
            if let Some(new_children) = func(&src[src_span.to_range()], child_at_start) {
                new_children
            } else {
                eprintln!("function failed");
                return false;
            };
        let child_mode = self.children[child_idx].kind().mode().apply(mode);
        eprintln!("child mode {:?}", child_mode);

        // Check if the children / child has the right type.
        let require_single = match policy {
            IncrementalSafety::AtomicPrimary | IncrementalSafety::SameKind => true,
            IncrementalSafety::SameKindInCode if child_mode == TokenMode::Code => true,
            _ => false,
        };

        if require_single {
            eprintln!("must be a single replacement");
            if new_children.len() != 1 {
                eprintln!("not a single replacement");
                return false;
            }

            if match policy {
                IncrementalSafety::SameKind => true,
                IncrementalSafety::SameKindInCode if child_mode == TokenMode::Code => {
                    true
                }
                _ => false,
            } {
                if self.children[child_idx].kind() != new_children[0].kind() {
                    eprintln!("not the same kind");
                    return false;
                }
            }
        }

        // Do not accept unclosed nodes if the old node did not use to be at the
        // right edge of the tree.
        if !outermost
            && new_children
                .iter()
                .flat_map(|x| x.errors())
                .any(|x| matches!(x, NodeKind::Error(ErrorPos::End, _)))
        {
            eprintln!("unclosed node");
            return false;
        }

        // Check if the neighbor invariants are still true.
        if mode == TokenMode::Markup {
            if child_idx > 0 {
                if self.children[child_idx - 1].kind().incremental_safety()
                    == IncrementalSafety::EnsureRightWhitespace
                    && !new_children[0].kind().is_whitespace()
                {
                    eprintln!("left whitespace missing");
                    return false;
                }
            }

            let mut new_at_start = child_at_start;
            for child in &new_children {
                new_at_start = child.kind().is_at_start(new_at_start);
            }

            for child in &self.children[child_idx + 1 ..] {
                if child.kind().is_trivia() {
                    new_at_start = child.kind().is_at_start(new_at_start);
                    continue;
                }

                match child.kind().incremental_safety() {
                    IncrementalSafety::EnsureAtStart if !new_at_start => return false,
                    IncrementalSafety::EnsureNotAtStart if new_at_start => return false,
                    _ => {}
                }
                break;
            }
        }

        eprintln!("... replacing");

        self.children.splice(child_idx .. child_idx + 1, new_children);
        true
    }
}

impl From<GreenNode> for Green {
    fn from(node: GreenNode) -> Self {
        Rc::new(node).into()
    }
}

impl From<Rc<GreenNode>> for Green {
    fn from(node: Rc<GreenNode>) -> Self {
        Self::Node(node)
    }
}

impl Debug for GreenNode {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        self.data.fmt(f)?;
        if !self.children.is_empty() {
            f.write_str(" ")?;
            f.debug_list().entries(&self.children).finish()?;
        }
        Ok(())
    }
}

/// Data shared between inner and leaf nodes.
#[derive(Clone, PartialEq)]
pub struct GreenData {
    /// What kind of node this is (each kind would have its own struct in a
    /// strongly typed AST).
    kind: NodeKind,
    /// The byte length of the node in the source.
    len: usize,
}

impl GreenData {
    /// Create new node metadata.
    pub fn new(kind: NodeKind, len: usize) -> Self {
        Self { len, kind }
    }

    /// The type of the node.
    pub fn kind(&self) -> &NodeKind {
        &self.kind
    }

    /// The length of the node.
    pub fn len(&self) -> usize {
        self.len
    }
}

impl From<GreenData> for Green {
    fn from(token: GreenData) -> Self {
        Self::Token(token)
    }
}

impl Debug for GreenData {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        write!(f, "{:?}: {}", self.kind, self.len)
    }
}

/// A owned wrapper for a green node with span information.
///
/// Owned variant of [`RedRef`]. Can be [cast](Self::cast) to an AST node.
#[derive(Clone, PartialEq)]
pub struct RedNode {
    id: SourceId,
    offset: usize,
    green: Green,
}

impl RedNode {
    /// Create a new red node from a root [`GreenNode`].
    pub fn from_root(root: Rc<GreenNode>, id: SourceId) -> Self {
        Self { id, offset: 0, green: root.into() }
    }

    /// Convert to a borrowed representation.
    pub fn as_ref(&self) -> RedRef<'_> {
        RedRef {
            id: self.id,
            offset: self.offset,
            green: &self.green,
        }
    }

    /// The type of the node.
    pub fn kind(&self) -> &NodeKind {
        self.as_ref().kind()
    }

    /// The length of the node.
    pub fn len(&self) -> usize {
        self.as_ref().len()
    }

    /// The span of the node.
    pub fn span(&self) -> Span {
        self.as_ref().span()
    }

    /// The error messages for this node and its descendants.
    pub fn errors(&self) -> Vec<Error> {
        self.as_ref().errors()
    }

    /// Convert the node to a typed AST node.
    pub fn cast<T>(self) -> Option<T>
    where
        T: TypedNode,
    {
        self.as_ref().cast()
    }

    /// The children of the node.
    pub fn children(&self) -> Children<'_> {
        self.as_ref().children()
    }

    /// Get the first child that can cast to some AST type.
    pub fn cast_first_child<T: TypedNode>(&self) -> Option<T> {
        self.as_ref().cast_first_child()
    }

    /// Get the last child that can cast to some AST type.
    pub fn cast_last_child<T: TypedNode>(&self) -> Option<T> {
        self.as_ref().cast_last_child()
    }
}

impl Debug for RedNode {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        self.as_ref().fmt(f)
    }
}

/// A borrowed wrapper for a [`GreenNode`] with span information.
///
/// Borrowed variant of [`RedNode`]. Can be [cast](Self::cast) to an AST node.
#[derive(Copy, Clone, PartialEq)]
pub struct RedRef<'a> {
    id: SourceId,
    offset: usize,
    green: &'a Green,
}

impl<'a> RedRef<'a> {
    /// Convert to an owned representation.
    pub fn own(self) -> RedNode {
        RedNode {
            id: self.id,
            offset: self.offset,
            green: self.green.clone(),
        }
    }

    /// The type of the node.
    pub fn kind(self) -> &'a NodeKind {
        self.green.kind()
    }

    /// The length of the node.
    pub fn len(self) -> usize {
        self.green.len()
    }

    /// The span of the node.
    pub fn span(self) -> Span {
        Span::new(self.id, self.offset, self.offset + self.green.len())
    }

    /// Whether the node or its children contain an error.
    pub fn erroneous(self) -> bool {
        self.green.erroneous()
    }

    /// The node's children.
    pub fn children(self) -> Children<'a> {
        let children = match &self.green {
            Green::Node(node) => node.children(),
            Green::Token(_) => &[],
        };

        Children {
            id: self.id,
            iter: children.iter(),
            front: self.offset,
            back: self.offset + self.len(),
        }
    }


    /// The error messages for this node and its descendants.
    pub fn errors(self) -> Vec<Error> {
        if !self.green.erroneous() {
            return vec![];
        }

        match self.kind() {
            NodeKind::Error(pos, msg) => {
                let span = match pos {
                    ErrorPos::Start => self.span().at_start(),
                    ErrorPos::Full => self.span(),
                    ErrorPos::End => self.span().at_end(),
                };

                vec![Error::new(span, msg.to_string())]
            }
            _ => self
                .children()
                .filter(|red| red.green.erroneous())
                .flat_map(|red| red.errors())
                .collect(),
        }
    }

    /// Convert the node to a typed AST node.
    pub fn cast<T>(self) -> Option<T>
    where
        T: TypedNode,
    {
        T::from_red(self)
    }

    /// Get the first child that can cast to some AST type.
    pub fn cast_first_child<T: TypedNode>(self) -> Option<T> {
        self.children().find_map(RedRef::cast)
    }

    /// Get the last child that can cast to some AST type.
    pub fn cast_last_child<T: TypedNode>(self) -> Option<T> {
        self.children().rev().find_map(RedRef::cast)
    }
}

impl Debug for RedRef<'_> {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        write!(f, "{:?}: {:?}", self.kind(), self.span())?;
        let mut children = self.children().peekable();
        if children.peek().is_some() {
            f.write_str(" ")?;
            f.debug_list().entries(children.map(RedRef::own)).finish()?;
        }
        Ok(())
    }
}

/// An iterator over the children of a red node.
pub struct Children<'a> {
    id: SourceId,
    iter: std::slice::Iter<'a, Green>,
    front: usize,
    back: usize,
}

impl<'a> Iterator for Children<'a> {
    type Item = RedRef<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        self.iter.next().map(|green| {
            let offset = self.front;
            self.front += green.len();
            RedRef { id: self.id, offset, green }
        })
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        self.iter.size_hint()
    }
}

impl DoubleEndedIterator for Children<'_> {
    fn next_back(&mut self) -> Option<Self::Item> {
        self.iter.next_back().map(|green| {
            self.back -= green.len();
            RedRef { id: self.id, offset: self.back, green }
        })
    }
}

impl ExactSizeIterator for Children<'_> {}

/// All syntactical building blocks that can be part of a Typst document.
///
/// Can be emitted as a token by the tokenizer or as part of a green node by
/// the parser.
#[derive(Debug, Clone, PartialEq)]
pub enum NodeKind {
    /// A left square bracket: `[`.
    LeftBracket,
    /// A right square bracket: `]`.
    RightBracket,
    /// A left curly brace: `{`.
    LeftBrace,
    /// A right curly brace: `}`.
    RightBrace,
    /// A left round parenthesis: `(`.
    LeftParen,
    /// A right round parenthesis: `)`.
    RightParen,
    /// An asterisk: `*`.
    Star,
    /// A comma: `,`.
    Comma,
    /// A semicolon: `;`.
    Semicolon,
    /// A colon: `:`.
    Colon,
    /// A plus: `+`.
    Plus,
    /// A hyphen: `-`.
    Minus,
    /// A slash: `/`.
    Slash,
    /// A single equals sign: `=`.
    Eq,
    /// Two equals signs: `==`.
    EqEq,
    /// An exclamation mark followed by an equals sign: `!=`.
    ExclEq,
    /// A less-than sign: `<`.
    Lt,
    /// A less-than sign followed by an equals sign: `<=`.
    LtEq,
    /// A greater-than sign: `>`.
    Gt,
    /// A greater-than sign followed by an equals sign: `>=`.
    GtEq,
    /// A plus followed by an equals sign: `+=`.
    PlusEq,
    /// A hyphen followed by an equals sign: `-=`.
    HyphEq,
    /// An asterisk followed by an equals sign: `*=`.
    StarEq,
    /// A slash followed by an equals sign: `/=`.
    SlashEq,
    /// The `not` operator.
    Not,
    /// The `and` operator.
    And,
    /// The `or` operator.
    Or,
    /// The `with` operator.
    With,
    /// Two dots: `..`.
    Dots,
    /// An equals sign followed by a greater-than sign: `=>`.
    Arrow,
    /// The none literal: `none`.
    None,
    /// The auto literal: `auto`.
    Auto,
    /// The `let` keyword.
    Let,
    /// The `set` keyword.
    Set,
    /// The `if` keyword.
    If,
    /// The `else` keyword.
    Else,
    /// The `for` keyword.
    For,
    /// The `in` keyword.
    In,
    /// The `while` keyword.
    While,
    /// The `break` keyword.
    Break,
    /// The `continue` keyword.
    Continue,
    /// The `return` keyword.
    Return,
    /// The `import` keyword.
    Import,
    /// The `include` keyword.
    Include,
    /// The `from` keyword.
    From,
    /// Template markup.
    Markup,
    /// One or more whitespace characters.
    Space(usize),
    /// A forced line break: `\`.
    Linebreak,
    /// A paragraph break: Two or more newlines.
    Parbreak,
    /// A consecutive non-markup string.
    Text(EcoString),
    /// A text node that cannot appear at the beginning of a source line.
    TextInLine(EcoString),
    /// A non-breaking space: `~`.
    NonBreakingSpace,
    /// An en-dash: `--`.
    EnDash,
    /// An em-dash: `---`.
    EmDash,
    /// A slash and the letter "u" followed by a hexadecimal unicode entity
    /// enclosed in curly braces: `\u{1F5FA}`.
    Escape(char),
    /// Strong text was enabled / disabled: `*`.
    Strong,
    /// Emphasized text was enabled / disabled: `_`.
    Emph,
    /// A section heading: `= Introduction`.
    Heading,
    /// An item in an enumeration (ordered list): `1. ...`.
    Enum,
    /// A numbering: `23.`.
    ///
    /// Can also exist without the number: `.`.
    EnumNumbering(Option<usize>),
    /// An item in an unordered list: `- ...`.
    List,
    /// An arbitrary number of backticks followed by inner contents, terminated
    /// with the same number of backticks: `` `...` ``.
    Raw(Rc<RawNode>),
    /// Dollar signs surrounding inner contents.
    Math(Rc<MathNode>),
    /// An identifier: `center`.
    Ident(EcoString),
    /// A boolean: `true`, `false`.
    Bool(bool),
    /// An integer: `120`.
    Int(i64),
    /// A floating-point number: `1.2`, `10e-4`.
    Float(f64),
    /// A length: `12pt`, `3cm`.
    Length(f64, LengthUnit),
    /// An angle: `90deg`.
    Angle(f64, AngularUnit),
    /// A percentage: `50%`.
    ///
    /// _Note_: `50%` is stored as `50.0` here, as in the corresponding
    /// [literal](ast::LitKind::Percent).
    Percentage(f64),
    /// A fraction unit: `3fr`.
    Fraction(f64),
    /// A quoted string: `"..."`.
    Str(EcoString),
    /// An array expression: `(1, "hi", 12cm)`.
    Array,
    /// A dictionary expression: `(thickness: 3pt, pattern: dashed)`.
    Dict,
    /// A named pair: `thickness: 3pt`.
    Named,
    /// A template expression: `[*Hi* there!]`.
    Template,
    /// A grouped expression: `(1 + 2)`.
    Group,
    /// A block expression: `{ let x = 1; x + 2 }`.
    Block,
    /// A unary operation: `-x`.
    Unary,
    /// A binary operation: `a + b`.
    Binary,
    /// An invocation of a function: `f(x, y)`.
    Call,
    /// A function call's argument list: `(x, y)`.
    CallArgs,
    /// Spreaded arguments or a parameter sink: `..x`.
    Spread,
    /// A closure expression: `(x, y) => z`.
    Closure,
    /// A closure's parameters: `(x, y)`.
    ClosureParams,
    /// A with expression: `f with (x, y: 1)`.
    WithExpr,
    /// A let expression: `let x = 1`.
    LetExpr,
    /// A set expression: `set text(...)`.
    SetExpr,
    /// An if-else expression: `if x { y } else { z }`.
    IfExpr,
    /// A while loop expression: `while x { ... }`.
    WhileExpr,
    /// A for loop expression: `for x in y { ... }`.
    ForExpr,
    /// A for loop's destructuring pattern: `x` or `x, y`.
    ForPattern,
    /// An import expression: `import a, b, c from "utils.typ"`.
    ImportExpr,
    /// Items to import: `a, b, c`.
    ImportItems,
    /// An include expression: `include "chapter1.typ"`.
    IncludeExpr,
    /// A line comment, two slashes followed by inner contents, terminated with
    /// a newline: `//<str>\n`.
    LineComment,
    /// A block comment, a slash and a star followed by inner contents,
    /// terminated with a star and a slash: `/*<str>*/`.
    ///
    /// The comment can contain nested block comments.
    BlockComment,
    /// Tokens that appear in the wrong place.
    Error(ErrorPos, EcoString),
    /// Unknown character sequences.
    Unknown(EcoString),
}

/// Where in a node an error should be annotated.
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum ErrorPos {
    /// At the start of the node.
    Start,
    /// Over the full width of the node.
    Full,
    /// At the end of the node.
    End,
}

impl NodeKind {
    /// Whether this is some kind of bracket.
    pub fn is_bracket(&self) -> bool {
        matches!(self, Self::LeftBracket | Self::RightBracket)
    }

    /// Whether this is some kind of brace.
    pub fn is_brace(&self) -> bool {
        matches!(self, Self::LeftBrace | Self::RightBrace)
    }

    /// Whether this is some kind of parenthesis.
    pub fn is_paren(&self) -> bool {
        matches!(self, Self::LeftParen | Self::RightParen)
    }

    /// Whether this is whitespace.
    pub fn is_whitespace(&self) -> bool {
        match self {
            Self::Space(_) | Self::Parbreak => true,
            _ => false,
        }
    }

    /// Whether this is trivia.
    pub fn is_trivia(&self) -> bool {
        match self {
            _ if self.is_whitespace() => true,
            Self::LineComment | Self::BlockComment => true,
            _ => false,
        }
    }

    /// Whether this is some kind of error.
    pub fn is_error(&self) -> bool {
        matches!(self, NodeKind::Error(_, _) | NodeKind::Unknown(_))
    }

    /// Whether this node is `at_start` given the previous value of the property.
    pub fn is_at_start(&self, prev: bool) -> bool {
        match self {
            Self::Space(n) if *n > 0 => true,
            Self::Parbreak => true,
            Self::LineComment | Self::BlockComment => prev,
            _ => false,
        }
    }

    /// Whether this token appears in Markup.
    pub fn mode(&self) -> NodeMode {
        match self {
            Self::Markup
            | Self::Space(_)
            | Self::Parbreak
            | Self::Text(_)
            | Self::TextInLine(_)
            | Self::NonBreakingSpace
            | Self::EnDash
            | Self::EmDash
            | Self::Escape(_)
            | Self::Strong
            | Self::Emph
            | Self::Math(_) => NodeMode::Markup,
            Self::Template
            | Self::Block
            | Self::None
            | Self::Auto
            | Self::Ident(_)
            | Self::Bool(_)
            | Self::Int(_)
            | Self::Float(_)
            | Self::Length(_, _)
            | Self::Angle(_, _)
            | Self::Percentage(_)
            | Self::Str(_)
            | Self::Fraction(_)
            | Self::Array
            | Self::Dict
            | Self::Group
            | Self::Call
            | Self::LineComment
            | Self::BlockComment
            | Self::Error(_, _)
            | Self::Minus
            | Self::Eq => NodeMode::Universal,
            _ => NodeMode::Code,
        }
    }

    pub fn reparsing_function(
        &self,
        parent_mode: TokenMode,
    ) -> Option<(fn(&str, bool) -> Option<Vec<Green>>, IncrementalSafety)> {
        let policy = self.incremental_safety();
        if policy == IncrementalSafety::Unsafe {
            return None;
        }

        let mode = self.mode();
        if mode == NodeMode::Code && policy == IncrementalSafety::UnsafeLayer {
            return None;
        }

        if mode != NodeMode::Markup
            && parent_mode == TokenMode::Code
            && policy == IncrementalSafety::AtomicPrimary
        {
            return Some((parse_atomic, policy));
        }

        let parser: fn(&str, bool) -> _ = match mode {
            NodeMode::Code => parse_code,
            NodeMode::Markup if self == &Self::Markup => parse_markup,
            NodeMode::Markup => parse_markup_elements,
            NodeMode::Universal if parent_mode == TokenMode::Code => parse_code,
            NodeMode::Universal => parse_markup_elements,
        };

        Some((parser, policy))
    }

    /// Whether it is safe to do incremental parsing on this node. Never allow
    /// non-termination errors if this is not already the last leaf node.
    pub fn incremental_safety(&self) -> IncrementalSafety {
        match self {
            // Replacing parenthesis changes if the expression is balanced and
            // is therefore not safe.
            Self::LeftBracket
            | Self::RightBracket
            | Self::LeftBrace
            | Self::RightBrace
            | Self::LeftParen
            | Self::RightParen => IncrementalSafety::Unsafe,

            // Replacing an operator can change whether the parent is an
            // operation which makes it unsafe. The star can appear in markup.
            Self::Star
            | Self::Comma
            | Self::Semicolon
            | Self::Colon
            | Self::Plus
            | Self::Minus
            | Self::Slash
            | Self::Eq
            | Self::EqEq
            | Self::ExclEq
            | Self::Lt
            | Self::LtEq
            | Self::Gt
            | Self::GtEq
            | Self::PlusEq
            | Self::HyphEq
            | Self::StarEq
            | Self::SlashEq
            | Self::Not
            | Self::And
            | Self::Or
            | Self::With
            | Self::Dots
            | Self::Arrow => IncrementalSafety::Unsafe,

            // These keywords are literals and can be safely be substituted with
            // other expressions.
            Self::None | Self::Auto => IncrementalSafety::AtomicPrimary,

            // These keywords change what kind of expression the parent is.
            Self::Let
            | Self::If
            | Self::Else
            | Self::For
            | Self::In
            | Self::While
            | Self::Break
            | Self::Continue
            | Self::Return
            | Self::Set
            | Self::Import
            | Self::Include
            | Self::From => IncrementalSafety::Unsafe,

            // This is a backslash followed by a space. But changing it to
            // anything else is fair game.
            Self::Linebreak => IncrementalSafety::EnsureRightWhitespace,

            Self::Markup => IncrementalSafety::SameKind,

            Self::Space(_) => IncrementalSafety::SameKindInCode,

            // These are all replaceable by other tokens.
            Self::Parbreak
            | Self::Text(_)
            | Self::NonBreakingSpace
            | Self::EnDash
            | Self::EmDash
            | Self::Escape(_)
            | Self::Strong
            | Self::Emph => IncrementalSafety::Safe,

            // This is text that needs to be not `at_start`, otherwise it would
            // start one of the below items.
            Self::TextInLine(_) => IncrementalSafety::EnsureNotAtStart,

            // These have to be `at_start` so they must be preceeded with a
            // Space(n) with n > 0 or a Parbreak.
            Self::Heading | Self::Enum | Self::List => IncrementalSafety::EnsureAtStart,

            // Changing the heading level, enum numbering, or list bullet
            // changes the next layer.
            Self::EnumNumbering(_) => IncrementalSafety::Unsafe,

            Self::Raw(_) | Self::Math(_) => IncrementalSafety::Safe,

            // These are expressions that can be replaced by other expressions.
            Self::Ident(_)
            | Self::Bool(_)
            | Self::Int(_)
            | Self::Float(_)
            | Self::Length(_, _)
            | Self::Angle(_, _)
            | Self::Percentage(_)
            | Self::Str(_)
            | Self::Fraction(_)
            | Self::Array
            | Self::Dict
            | Self::Group => IncrementalSafety::AtomicPrimary,

            Self::Call | Self::Unary | Self::Binary | Self::SetExpr => {
                IncrementalSafety::UnsafeLayer
            }

            Self::CallArgs | Self::Named | Self::Spread => IncrementalSafety::UnsafeLayer,

            // The closure is a bit magic with the let expression, and also it
            // is not atomic.
            Self::Closure | Self::ClosureParams => IncrementalSafety::UnsafeLayer,

            // These can appear as bodies and would trigger an error if they
            // became something else.
            Self::Template | Self::Block => IncrementalSafety::SameKindInCode,

            Self::ForExpr
            | Self::WhileExpr
            | Self::IfExpr
            | Self::LetExpr
            | Self::ImportExpr
            | Self::IncludeExpr => IncrementalSafety::AtomicPrimary,

            Self::WithExpr | Self::ForPattern | Self::ImportItems => {
                IncrementalSafety::UnsafeLayer
            }

            // These can appear everywhere and must not change to other stuff
            // because that could change the outer expression.
            Self::LineComment | Self::BlockComment => IncrementalSafety::SameKind,

            Self::Error(_, _) | Self::Unknown(_) => IncrementalSafety::Unsafe,
        }
    }

    /// A human-readable name for the kind.
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::LeftBracket => "opening bracket",
            Self::RightBracket => "closing bracket",
            Self::LeftBrace => "opening brace",
            Self::RightBrace => "closing brace",
            Self::LeftParen => "opening paren",
            Self::RightParen => "closing paren",
            Self::Star => "star",
            Self::Comma => "comma",
            Self::Semicolon => "semicolon",
            Self::Colon => "colon",
            Self::Plus => "plus",
            Self::Minus => "minus",
            Self::Slash => "slash",
            Self::Eq => "assignment operator",
            Self::EqEq => "equality operator",
            Self::ExclEq => "inequality operator",
            Self::Lt => "less-than operator",
            Self::LtEq => "less-than or equal operator",
            Self::Gt => "greater-than operator",
            Self::GtEq => "greater-than or equal operator",
            Self::PlusEq => "add-assign operator",
            Self::HyphEq => "subtract-assign operator",
            Self::StarEq => "multiply-assign operator",
            Self::SlashEq => "divide-assign operator",
            Self::Not => "operator `not`",
            Self::And => "operator `and`",
            Self::Or => "operator `or`",
            Self::With => "operator `with`",
            Self::Dots => "dots",
            Self::Arrow => "arrow",
            Self::None => "`none`",
            Self::Auto => "`auto`",
            Self::Let => "keyword `let`",
            Self::Set => "keyword `set`",
            Self::If => "keyword `if`",
            Self::Else => "keyword `else`",
            Self::For => "keyword `for`",
            Self::In => "keyword `in`",
            Self::While => "keyword `while`",
            Self::Break => "keyword `break`",
            Self::Continue => "keyword `continue`",
            Self::Return => "keyword `return`",
            Self::Import => "keyword `import`",
            Self::Include => "keyword `include`",
            Self::From => "keyword `from`",
            Self::Markup => "markup",
            Self::Space(_) => "space",
            Self::Linebreak => "forced linebreak",
            Self::Parbreak => "paragraph break",
            Self::Text(_) | Self::TextInLine(_) => "text",
            Self::NonBreakingSpace => "non-breaking space",
            Self::EnDash => "en dash",
            Self::EmDash => "em dash",
            Self::Escape(_) => "escape sequence",
            Self::Strong => "strong",
            Self::Emph => "emphasis",
            Self::Heading => "heading",
            Self::Enum => "enumeration item",
            Self::EnumNumbering(_) => "enumeration item numbering",
            Self::List => "list item",
            Self::Raw(_) => "raw block",
            Self::Math(_) => "math formula",
            Self::Ident(_) => "identifier",
            Self::Bool(_) => "boolean",
            Self::Int(_) => "integer",
            Self::Float(_) => "float",
            Self::Length(_, _) => "length",
            Self::Angle(_, _) => "angle",
            Self::Percentage(_) => "percentage",
            Self::Fraction(_) => "`fr` value",
            Self::Str(_) => "string",
            Self::Array => "array",
            Self::Dict => "dictionary",
            Self::Named => "named argument",
            Self::Template => "template",
            Self::Group => "group",
            Self::Block => "block",
            Self::Unary => "unary expression",
            Self::Binary => "binary expression",
            Self::Call => "call",
            Self::CallArgs => "call arguments",
            Self::Spread => "parameter sink",
            Self::Closure => "closure",
            Self::ClosureParams => "closure parameters",
            Self::WithExpr => "`with` expression",
            Self::LetExpr => "`let` expression",
            Self::SetExpr => "`set` expression",
            Self::IfExpr => "`if` expression",
            Self::WhileExpr => "while-loop expression",
            Self::ForExpr => "for-loop expression",
            Self::ForPattern => "for-loop destructuring pattern",
            Self::ImportExpr => "`import` expression",
            Self::ImportItems => "import items",
            Self::IncludeExpr => "`include` expression",
            Self::LineComment => "line comment",
            Self::BlockComment => "block comment",
            Self::Error(_, _) => "parse error",
            Self::Unknown(src) => match src.as_str() {
                "*/" => "end of block comment",
                _ => "invalid token",
            },
        }
    }
}

impl Display for NodeKind {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.pad(self.as_str())
    }
}

/// This enum describes what conditions a node has for being replaced by a new
/// parse result.
///
/// Safe nodes are replaced by the new parse result from the respective mode.
/// They can be replaced by multiple tokens. If a token is inserted in Markup
/// mode and the next token would not be `at_start` there needs to be a forward
/// check for a `EnsureAtStart` node. If this fails, the parent has to be
/// reparsed. if the direct whitespace sibling of a `EnsureRightWhitespace` is
/// `Unsafe`. Similarly, if a `EnsureRightWhitespace` token is one of the last
/// tokens to be inserted, the edit is invalidated if there is no following
/// whitespace. The atomic nodes may only be replaced by other atomic nodes. The
/// unsafe layers cannot be used but allow children access, the unsafe nodes do
/// neither.
///
/// *Procedure:*
/// 1. Check if the node is safe - if unsafe layer recurse, if unsafe, return
///    None.
/// 2. Reparse with appropriate node kind and `at_start`.
/// 3. Check whether the topmost group is terminated and the range was
///    completely consumed, otherwise return None.
/// 4. Check if the type criteria are met.
/// 5. If the node is not at the end of the tree, check if Strings etc. are
///    terminated.
/// 6. If this is markup, check the following things:
///   - The `at_start` conditions of the next non-comment and non-space(0) node
///     are met.
///   - The first node is whitespace or the previous siblings are not
///     `EnsureRightWhitespace`.
///   - If any of those fails, return None.
#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
pub enum IncrementalSafety {
    /// Changing this node can never have an influence on the other nodes.
    Safe,
    /// This node has to be replaced with a single token of the same kind.
    SameKind,
    /// This node has to be replaced with a single token of the same kind if in
    /// code mode.
    SameKindInCode,
    /// These nodes depend on being at the start of a line. Reparsing of safe
    /// left neighbors has to check this invariant. Otherwise, this node is
    /// safe.
    EnsureAtStart,
    /// These nodes depend on not being at the start of a line. Reparsing of
    /// safe left neighbors has to check this invariant. Otherwise, this node is
    /// safe.
    EnsureNotAtStart,
    /// These nodes must be followed by whitespace.
    EnsureRightWhitespace,
    /// Changing this node into a single atomic expression is allowed if it
    /// appears in code mode, otherwise it is safe.
    AtomicPrimary,
    /// Changing an unsafe layer node changes what the parents or the
    /// surrounding nodes would be and is therefore disallowed. Change the
    /// parents or children instead. If it appears in Markup, however, it is
    /// safe to change.
    UnsafeLayer,
    /// Changing an unsafe node or any of its children will trigger undefined
    /// behavior. Change the parents instead.
    Unsafe,
}

/// This enum describes which mode a token of [`NodeKind`] can appear in.
#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum NodeMode {
    /// The token can only appear in markup mode.
    Markup,
    /// The token can only appear in code mode.
    Code,
    /// The token can appear in either mode. Look at the parent node to decide
    /// which mode it is in.
    Universal,
}

impl NodeMode {
    /// Returns the new [`TokenMode`] given the old one.
    pub fn apply(&self, old: TokenMode) -> TokenMode {
        match self {
            Self::Markup => TokenMode::Markup,
            Self::Code => TokenMode::Code,
            Self::Universal => old,
        }
    }
}
