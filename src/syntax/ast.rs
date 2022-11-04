//! A typed layer over the untyped syntax tree.
//!
//! The AST is rooted in the [`MarkupNode`].

use std::num::NonZeroUsize;
use std::ops::Deref;

use super::{NodeData, NodeKind, RawKind, Span, SyntaxNode, Unit};
use crate::util::EcoString;

/// A typed AST node.
pub trait TypedNode: Sized {
    /// Convert a node into its typed variant.
    fn from_untyped(node: &SyntaxNode) -> Option<Self>;

    /// A reference to the underlying syntax node.
    fn as_untyped(&self) -> &SyntaxNode;

    /// The source code location.
    fn span(&self) -> Span {
        self.as_untyped().span()
    }
}

macro_rules! node {
    ($(#[$attr:meta])* $name:ident) => {
        node!{ $(#[$attr])* $name: NodeKind::$name { .. } }
    };
    ($(#[$attr:meta])* $name:ident: $variants:pat) => {
        #[derive(Debug, Clone, PartialEq, Hash)]
        #[repr(transparent)]
        $(#[$attr])*
        pub struct $name(SyntaxNode);

        impl TypedNode for $name {
            fn from_untyped(node: &SyntaxNode) -> Option<Self> {
                if matches!(node.kind(), $variants) {
                    Some(Self(node.clone()))
                } else {
                    None
                }
            }

            fn as_untyped(&self) -> &SyntaxNode {
                &self.0
            }
        }
    };
}

node! {
    /// The syntactical root capable of representing a full parsed document.
    Markup
}

impl Markup {
    /// The children.
    pub fn children(&self) -> impl DoubleEndedIterator<Item = MarkupNode> + '_ {
        self.0.children().filter_map(SyntaxNode::cast)
    }
}

/// A single piece of markup.
#[derive(Debug, Clone, PartialEq)]
pub enum MarkupNode {
    /// Whitespace containing less than two newlines.
    Space(Space),
    /// A forced line break.
    Linebreak(Linebreak),
    /// Plain text.
    Text(Text),
    /// An escape sequence: `\#`, `\u{1F5FA}`.
    Escape(Escape),
    /// A shorthand for a unicode codepoint. For example, `~` for non-breaking
    /// space or `-?` for a soft hyphen.
    Shorthand(Shorthand),
    /// A smart quote: `'` or `"`.
    SmartQuote(SmartQuote),
    /// Strong markup: `*Strong*`.
    Strong(Strong),
    /// Emphasized markup: `_Emphasized_`.
    Emph(Emph),
    /// A raw block with optional syntax highlighting: `` `...` ``.
    Raw(Raw),
    /// A hyperlink: `https://typst.org`.
    Link(Link),
    /// A label: `<label>`.
    Label(Label),
    /// A reference: `@target`.
    Ref(Ref),
    /// A section heading: `= Introduction`.
    Heading(Heading),
    /// An item in an unordered list: `- ...`.
    List(ListItem),
    /// An item in an enumeration (ordered list): `+ ...` or `1. ...`.
    Enum(EnumItem),
    /// An item in a description list: `/ Term: Details`.
    Desc(DescItem),
    /// A math formula: `$x$`, `$ x^2 $`.
    Math(Math),
    /// An expression.
    Expr(Expr),
}

impl TypedNode for MarkupNode {
    fn from_untyped(node: &SyntaxNode) -> Option<Self> {
        match node.kind() {
            NodeKind::Space { .. } => node.cast().map(Self::Space),
            NodeKind::Linebreak => node.cast().map(Self::Linebreak),
            NodeKind::Text(_) => node.cast().map(Self::Text),
            NodeKind::Escape(_) => node.cast().map(Self::Escape),
            NodeKind::Shorthand(_) => node.cast().map(Self::Shorthand),
            NodeKind::SmartQuote { .. } => node.cast().map(Self::SmartQuote),
            NodeKind::Strong => node.cast().map(Self::Strong),
            NodeKind::Emph => node.cast().map(Self::Emph),
            NodeKind::Raw(_) => node.cast().map(Self::Raw),
            NodeKind::Link(_) => node.cast().map(Self::Link),
            NodeKind::Label(_) => node.cast().map(Self::Label),
            NodeKind::Ref(_) => node.cast().map(Self::Ref),
            NodeKind::Heading => node.cast().map(Self::Heading),
            NodeKind::ListItem => node.cast().map(Self::List),
            NodeKind::EnumItem => node.cast().map(Self::Enum),
            NodeKind::DescItem => node.cast().map(Self::Desc),
            NodeKind::Math => node.cast().map(Self::Math),
            _ => node.cast().map(Self::Expr),
        }
    }

    fn as_untyped(&self) -> &SyntaxNode {
        match self {
            Self::Space(v) => v.as_untyped(),
            Self::Linebreak(v) => v.as_untyped(),
            Self::Text(v) => v.as_untyped(),
            Self::Escape(v) => v.as_untyped(),
            Self::Shorthand(v) => v.as_untyped(),
            Self::SmartQuote(v) => v.as_untyped(),
            Self::Strong(v) => v.as_untyped(),
            Self::Emph(v) => v.as_untyped(),
            Self::Raw(v) => v.as_untyped(),
            Self::Link(v) => v.as_untyped(),
            Self::Label(v) => v.as_untyped(),
            Self::Ref(v) => v.as_untyped(),
            Self::Heading(v) => v.as_untyped(),
            Self::List(v) => v.as_untyped(),
            Self::Enum(v) => v.as_untyped(),
            Self::Desc(v) => v.as_untyped(),
            Self::Math(v) => v.as_untyped(),
            Self::Expr(v) => v.as_untyped(),
        }
    }
}

node! {
    /// Whitespace.
    Space
}

impl Space {
    /// Get the number of newlines.
    pub fn newlines(&self) -> usize {
        match self.0.kind() {
            &NodeKind::Space { newlines } => newlines,
            _ => panic!("space is of wrong kind"),
        }
    }
}

node! {
    /// A forced line break: `\`.
    Linebreak
}

node! {
    /// Plain text.
    Text
}

impl Text {
    /// Get the text.
    pub fn get(&self) -> &EcoString {
        match self.0.kind() {
            NodeKind::Text(v) => v,
            _ => panic!("text is of wrong kind"),
        }
    }
}

node! {
    /// An escape sequence: `\#`, `\u{1F5FA}`.
    Escape
}

impl Escape {
    /// Get the escaped character.
    pub fn get(&self) -> char {
        match self.0.kind() {
            &NodeKind::Escape(v) => v,
            _ => panic!("escape is of wrong kind"),
        }
    }
}

node! {
    /// A shorthand for a unicode codepoint. For example, `~` for non-breaking
    /// space or `-?` for a soft hyphen.
    Shorthand
}

impl Shorthand {
    /// Get the shorthanded character.
    pub fn get(&self) -> char {
        match self.0.kind() {
            &NodeKind::Shorthand(v) => v,
            _ => panic!("shorthand is of wrong kind"),
        }
    }
}

node! {
    /// A smart quote: `'` or `"`.
    SmartQuote
}

impl SmartQuote {
    /// Whether this is a double quote.
    pub fn double(&self) -> bool {
        match self.0.kind() {
            &NodeKind::SmartQuote { double } => double,
            _ => panic!("smart quote is of wrong kind"),
        }
    }
}

node! {
    /// Strong content: `*Strong*`.
    Strong
}

impl Strong {
    /// The contents of the strong node.
    pub fn body(&self) -> Markup {
        self.0.cast_first_child().expect("strong node is missing markup body")
    }
}

node! {
    /// Emphasized content: `_Emphasized_`.
    Emph
}

impl Emph {
    /// The contents of the emphasis node.
    pub fn body(&self) -> Markup {
        self.0
            .cast_first_child()
            .expect("emphasis node is missing markup body")
    }
}

node! {
    /// A raw block with optional syntax highlighting: `` `...` ``.
    Raw
}

impl Raw {
    /// The raw text.
    pub fn text(&self) -> &EcoString {
        &self.get().text
    }

    /// An optional identifier specifying the language to syntax-highlight in.
    pub fn lang(&self) -> Option<&EcoString> {
        self.get().lang.as_ref()
    }

    /// Whether the raw block is block-level.
    pub fn block(&self) -> bool {
        self.get().block
    }

    /// The raw fields.
    fn get(&self) -> &RawKind {
        match self.0.kind() {
            NodeKind::Raw(v) => v.as_ref(),
            _ => panic!("raw is of wrong kind"),
        }
    }
}

node! {
    /// A hyperlink: `https://typst.org`.
    Link
}

impl Link {
    /// Get the URL.
    pub fn url(&self) -> &EcoString {
        match self.0.kind() {
            NodeKind::Link(url) => url,
            _ => panic!("link is of wrong kind"),
        }
    }
}

node! {
    /// A label: `<label>`.
    Label
}

impl Label {
    /// Get the label.
    pub fn get(&self) -> &EcoString {
        match self.0.kind() {
            NodeKind::Label(v) => v,
            _ => panic!("label is of wrong kind"),
        }
    }
}

node! {
    /// A reference: `@target`.
    Ref
}

impl Ref {
    /// Get the target.
    pub fn get(&self) -> &EcoString {
        match self.0.kind() {
            NodeKind::Ref(v) => v,
            _ => panic!("reference is of wrong kind"),
        }
    }
}

node! {
    /// A section heading: `= Introduction`.
    Heading
}

impl Heading {
    /// The contents of the heading.
    pub fn body(&self) -> Markup {
        self.0.cast_first_child().expect("heading is missing markup body")
    }

    /// The section depth (numer of equals signs).
    pub fn level(&self) -> NonZeroUsize {
        self.0
            .children()
            .filter(|n| n.kind() == &NodeKind::Eq)
            .count()
            .try_into()
            .expect("heading is missing equals sign")
    }
}

node! {
    /// An item in an unordered list: `- ...`.
    ListItem
}

impl ListItem {
    /// The contents of the list item.
    pub fn body(&self) -> Markup {
        self.0.cast_first_child().expect("list item is missing body")
    }
}

node! {
    /// An item in an enumeration (ordered list): `1. ...`.
    EnumItem
}

impl EnumItem {
    /// The number, if any.
    pub fn number(&self) -> Option<usize> {
        self.0.children().find_map(|node| match node.kind() {
            NodeKind::EnumNumbering(num) => Some(*num),
            _ => None,
        })
    }

    /// The contents of the list item.
    pub fn body(&self) -> Markup {
        self.0.cast_first_child().expect("enum item is missing body")
    }
}

node! {
    /// An item in a description list: `/ Term: Details`.
    DescItem
}

impl DescItem {
    /// The term described by the item.
    pub fn term(&self) -> Markup {
        self.0
            .cast_first_child()
            .expect("description list item is missing term")
    }

    /// The description of the term.
    pub fn body(&self) -> Markup {
        self.0
            .cast_last_child()
            .expect("description list item is missing body")
    }
}

node! {
    /// A math formula: `$x$`, `$ x^2 $`.
    Math
}

impl Math {
    /// The children.
    pub fn children(&self) -> impl DoubleEndedIterator<Item = MathNode> + '_ {
        self.0.children().filter_map(SyntaxNode::cast)
    }

    /// Whether this is a display-level math formula.
    pub fn display(&self) -> bool {
        matches!(self.children().next(), Some(MathNode::Space(_)))
            && matches!(self.children().last(), Some(MathNode::Space(_)))
    }
}

/// A single piece of a math formula.
#[derive(Debug, Clone, PartialEq, Hash)]
pub enum MathNode {
    /// Whitespace.
    Space(Space),
    /// A forced line break: `\`.
    Linebreak(Linebreak),
    /// An escape sequence: `\#`, `\u{1F5FA}`.
    Escape(Escape),
    /// A atom: `x`, `+`, `12`.
    Atom(Atom),
    /// A base with an optional sub- and superscript: `a_1^2`.
    Script(Script),
    /// A fraction: `x/2`.
    Frac(Frac),
    /// An alignment indicator: `&`, `&&`.
    Align(Align),
    /// Grouped mathematical material.
    Group(Math),
    /// An expression.
    Expr(Expr),
}

impl TypedNode for MathNode {
    fn from_untyped(node: &SyntaxNode) -> Option<Self> {
        match node.kind() {
            NodeKind::Space { .. } => node.cast().map(Self::Space),
            NodeKind::Linebreak => node.cast().map(Self::Linebreak),
            NodeKind::Escape(_) => node.cast().map(Self::Escape),
            NodeKind::Atom(_) => node.cast().map(Self::Atom),
            NodeKind::Script => node.cast().map(Self::Script),
            NodeKind::Frac => node.cast().map(Self::Frac),
            NodeKind::Align => node.cast().map(Self::Align),
            NodeKind::Math => node.cast().map(Self::Group),
            _ => node.cast().map(Self::Expr),
        }
    }

    fn as_untyped(&self) -> &SyntaxNode {
        match self {
            Self::Space(v) => v.as_untyped(),
            Self::Linebreak(v) => v.as_untyped(),
            Self::Escape(v) => v.as_untyped(),
            Self::Atom(v) => v.as_untyped(),
            Self::Script(v) => v.as_untyped(),
            Self::Frac(v) => v.as_untyped(),
            Self::Align(v) => v.as_untyped(),
            Self::Group(v) => v.as_untyped(),
            Self::Expr(v) => v.as_untyped(),
        }
    }
}

node! {
    /// A atom in a formula: `x`, `+`, `12`.
    Atom
}

impl Atom {
    /// Get the atom's text.
    pub fn get(&self) -> &EcoString {
        match self.0.kind() {
            NodeKind::Atom(v) => v,
            _ => panic!("atom is of wrong kind"),
        }
    }
}

node! {
    /// A base with an optional sub- and superscript in a formula: `a_1^2`.
    Script
}

impl Script {
    /// The base of the script.
    pub fn base(&self) -> MathNode {
        self.0.cast_first_child().expect("script node is missing base")
    }

    /// The subscript.
    pub fn sub(&self) -> Option<MathNode> {
        self.0
            .children()
            .skip_while(|node| !matches!(node.kind(), NodeKind::Underscore))
            .nth(1)
            .map(|node| node.cast().expect("script node has invalid subscript"))
    }

    /// The superscript.
    pub fn sup(&self) -> Option<MathNode> {
        self.0
            .children()
            .skip_while(|node| !matches!(node.kind(), NodeKind::Hat))
            .nth(1)
            .map(|node| node.cast().expect("script node has invalid superscript"))
    }
}

node! {
    /// A fraction in a formula: `x/2`
    Frac
}

impl Frac {
    /// The numerator.
    pub fn num(&self) -> MathNode {
        self.0.cast_first_child().expect("fraction is missing numerator")
    }

    /// The denominator.
    pub fn denom(&self) -> MathNode {
        self.0.cast_last_child().expect("fraction is missing denominator")
    }
}

node! {
    /// An alignment indicator in a formula: `&`, `&&`.
    Align
}

impl Align {
    /// The number of ampersands.
    pub fn count(&self) -> usize {
        self.0.children().filter(|n| n.kind() == &NodeKind::Amp).count()
    }
}

/// An expression.
#[derive(Debug, Clone, PartialEq, Hash)]
pub enum Expr {
    /// A literal: `1`, `true`, ...
    Lit(Lit),
    /// An identifier: `left`.
    Ident(Ident),
    /// A code block: `{ let x = 1; x + 2 }`.
    Code(CodeBlock),
    /// A content block: `[*Hi* there!]`.
    Content(ContentBlock),
    /// A grouped expression: `(1 + 2)`.
    Parenthesized(Parenthesized),
    /// An array expression: `(1, "hi", 12cm)`.
    Array(Array),
    /// A dictionary expression: `(thickness: 3pt, pattern: dashed)`.
    Dict(Dict),
    /// A unary operation: `-x`.
    Unary(Unary),
    /// A binary operation: `a + b`.
    Binary(Binary),
    /// A field access: `properties.age`.
    FieldAccess(FieldAccess),
    /// An invocation of a function: `f(x, y)`.
    FuncCall(FuncCall),
    /// An invocation of a method: `array.push(v)`.
    MethodCall(MethodCall),
    /// A closure expression: `(x, y) => z`.
    Closure(Closure),
    /// A let binding: `let x = 1`.
    Let(LetBinding),
    /// A set rule: `set text(...)`.
    Set(SetRule),
    /// A show rule: `show node: heading as [*{nody.body}*]`.
    Show(ShowRule),
    /// A wrap rule: `wrap body in columns(2, body)`.
    Wrap(WrapRule),
    /// An if-else conditional: `if x { y } else { z }`.
    Conditional(Conditional),
    /// A while loop: `while x { y }`.
    While(WhileLoop),
    /// A for loop: `for x in y { z }`.
    For(ForLoop),
    /// A module import: `import a, b, c from "utils.typ"`.
    Import(ModuleImport),
    /// A module include: `include "chapter1.typ"`.
    Include(ModuleInclude),
    /// A break statement: `break`.
    Break(BreakStmt),
    /// A continue statement: `continue`.
    Continue(ContinueStmt),
    /// A return statement: `return`, `return x + 1`.
    Return(ReturnStmt),
}

impl TypedNode for Expr {
    fn from_untyped(node: &SyntaxNode) -> Option<Self> {
        match node.kind() {
            NodeKind::Ident(_) => node.cast().map(Self::Ident),
            NodeKind::CodeBlock => node.cast().map(Self::Code),
            NodeKind::ContentBlock => node.cast().map(Self::Content),
            NodeKind::Parenthesized => node.cast().map(Self::Parenthesized),
            NodeKind::Array => node.cast().map(Self::Array),
            NodeKind::Dict => node.cast().map(Self::Dict),
            NodeKind::Unary => node.cast().map(Self::Unary),
            NodeKind::Binary => node.cast().map(Self::Binary),
            NodeKind::FieldAccess => node.cast().map(Self::FieldAccess),
            NodeKind::FuncCall => node.cast().map(Self::FuncCall),
            NodeKind::MethodCall => node.cast().map(Self::MethodCall),
            NodeKind::Closure => node.cast().map(Self::Closure),
            NodeKind::LetBinding => node.cast().map(Self::Let),
            NodeKind::SetRule => node.cast().map(Self::Set),
            NodeKind::ShowRule => node.cast().map(Self::Show),
            NodeKind::WrapRule => node.cast().map(Self::Wrap),
            NodeKind::Conditional => node.cast().map(Self::Conditional),
            NodeKind::WhileLoop => node.cast().map(Self::While),
            NodeKind::ForLoop => node.cast().map(Self::For),
            NodeKind::ModuleImport => node.cast().map(Self::Import),
            NodeKind::ModuleInclude => node.cast().map(Self::Include),
            NodeKind::BreakStmt => node.cast().map(Self::Break),
            NodeKind::ContinueStmt => node.cast().map(Self::Continue),
            NodeKind::ReturnStmt => node.cast().map(Self::Return),
            _ => node.cast().map(Self::Lit),
        }
    }

    fn as_untyped(&self) -> &SyntaxNode {
        match self {
            Self::Lit(v) => v.as_untyped(),
            Self::Code(v) => v.as_untyped(),
            Self::Content(v) => v.as_untyped(),
            Self::Ident(v) => v.as_untyped(),
            Self::Array(v) => v.as_untyped(),
            Self::Dict(v) => v.as_untyped(),
            Self::Parenthesized(v) => v.as_untyped(),
            Self::Unary(v) => v.as_untyped(),
            Self::Binary(v) => v.as_untyped(),
            Self::FieldAccess(v) => v.as_untyped(),
            Self::FuncCall(v) => v.as_untyped(),
            Self::MethodCall(v) => v.as_untyped(),
            Self::Closure(v) => v.as_untyped(),
            Self::Let(v) => v.as_untyped(),
            Self::Set(v) => v.as_untyped(),
            Self::Show(v) => v.as_untyped(),
            Self::Wrap(v) => v.as_untyped(),
            Self::Conditional(v) => v.as_untyped(),
            Self::While(v) => v.as_untyped(),
            Self::For(v) => v.as_untyped(),
            Self::Import(v) => v.as_untyped(),
            Self::Include(v) => v.as_untyped(),
            Self::Break(v) => v.as_untyped(),
            Self::Continue(v) => v.as_untyped(),
            Self::Return(v) => v.as_untyped(),
        }
    }
}

impl Expr {
    /// Whether the expression can be shortened in markup with a hashtag.
    pub fn has_short_form(&self) -> bool {
        matches!(
            self,
            Self::Ident(_)
                | Self::FuncCall(_)
                | Self::Let(_)
                | Self::Set(_)
                | Self::Show(_)
                | Self::Wrap(_)
                | Self::Conditional(_)
                | Self::While(_)
                | Self::For(_)
                | Self::Import(_)
                | Self::Include(_)
        )
    }
}

node! {
    /// A literal: `1`, `true`, ...
    Lit: NodeKind::None
       | NodeKind::Auto
       | NodeKind::Bool(_)
       | NodeKind::Int(_)
       | NodeKind::Float(_)
       | NodeKind::Numeric(_, _)
       | NodeKind::Str(_)
}

impl Lit {
    /// The kind of literal.
    pub fn kind(&self) -> LitKind {
        match *self.0.kind() {
            NodeKind::None => LitKind::None,
            NodeKind::Auto => LitKind::Auto,
            NodeKind::Bool(v) => LitKind::Bool(v),
            NodeKind::Int(v) => LitKind::Int(v),
            NodeKind::Float(v) => LitKind::Float(v),
            NodeKind::Numeric(v, unit) => LitKind::Numeric(v, unit),
            NodeKind::Str(ref v) => LitKind::Str(v.clone()),
            _ => panic!("literal is of wrong kind"),
        }
    }
}

/// The kind of a literal.
#[derive(Debug, Clone, PartialEq)]
pub enum LitKind {
    /// The none literal: `none`.
    None,
    /// The auto literal: `auto`.
    Auto,
    /// A boolean literal: `true`, `false`.
    Bool(bool),
    /// An integer literal: `120`.
    Int(i64),
    /// A floating-point literal: `1.2`, `10e-4`.
    Float(f64),
    /// A numeric literal with a unit: `12pt`, `3cm`, `2em`, `90deg`, `50%`.
    Numeric(f64, Unit),
    /// A string literal: `"hello!"`.
    Str(EcoString),
}

node! {
    /// A code block: `{ let x = 1; x + 2 }`.
    CodeBlock
}

impl CodeBlock {
    /// The list of expressions contained in the block.
    pub fn exprs(&self) -> impl DoubleEndedIterator<Item = Expr> + '_ {
        self.0.children().filter_map(SyntaxNode::cast)
    }
}

node! {
    /// A content block: `[*Hi* there!]`.
    ContentBlock
}

impl ContentBlock {
    /// The contained markup.
    pub fn body(&self) -> Markup {
        self.0.cast_first_child().expect("content block is missing body")
    }
}

node! {
    /// A parenthesized expression: `(1 + 2)`.
    Parenthesized
}

impl Parenthesized {
    /// The wrapped expression.
    pub fn expr(&self) -> Expr {
        self.0
            .cast_first_child()
            .expect("parenthesized expression is missing expression")
    }
}

node! {
    /// An array: `(1, "hi", 12cm)`.
    Array
}

impl Array {
    /// The array's items.
    pub fn items(&self) -> impl DoubleEndedIterator<Item = ArrayItem> + '_ {
        self.0.children().filter_map(SyntaxNode::cast)
    }
}

/// An item in an array.
#[derive(Debug, Clone, PartialEq, Hash)]
pub enum ArrayItem {
    /// A bare expression: `12`.
    Pos(Expr),
    /// A spreaded expression: `..things`.
    Spread(Expr),
}

impl TypedNode for ArrayItem {
    fn from_untyped(node: &SyntaxNode) -> Option<Self> {
        match node.kind() {
            NodeKind::Spread => node.cast_first_child().map(Self::Spread),
            _ => node.cast().map(Self::Pos),
        }
    }

    fn as_untyped(&self) -> &SyntaxNode {
        match self {
            Self::Pos(v) => v.as_untyped(),
            Self::Spread(v) => v.as_untyped(),
        }
    }
}

node! {
    /// A dictionary: `(thickness: 3pt, pattern: dashed)`.
    Dict
}

impl Dict {
    /// The dictionary's items.
    pub fn items(&self) -> impl DoubleEndedIterator<Item = DictItem> + '_ {
        self.0.children().filter_map(SyntaxNode::cast)
    }
}

/// An item in an dictionary expresssion.
#[derive(Debug, Clone, PartialEq, Hash)]
pub enum DictItem {
    /// A named pair: `thickness: 3pt`.
    Named(Named),
    /// A keyed pair: `"spacy key": true`.
    Keyed(Keyed),
    /// A spreaded expression: `..things`.
    Spread(Expr),
}

impl TypedNode for DictItem {
    fn from_untyped(node: &SyntaxNode) -> Option<Self> {
        match node.kind() {
            NodeKind::Named => node.cast().map(Self::Named),
            NodeKind::Keyed => node.cast().map(Self::Keyed),
            NodeKind::Spread => node.cast_first_child().map(Self::Spread),
            _ => None,
        }
    }

    fn as_untyped(&self) -> &SyntaxNode {
        match self {
            Self::Named(v) => v.as_untyped(),
            Self::Keyed(v) => v.as_untyped(),
            Self::Spread(v) => v.as_untyped(),
        }
    }
}

node! {
    /// A pair of a name and an expression: `thickness: 3pt`.
    Named
}

impl Named {
    /// The name: `thickness`.
    pub fn name(&self) -> Ident {
        self.0.cast_first_child().expect("named pair is missing name")
    }

    /// The right-hand side of the pair: `3pt`.
    pub fn expr(&self) -> Expr {
        self.0.cast_last_child().expect("named pair is missing expression")
    }
}

node! {
    /// A pair of a string key and an expression: `"spacy key": true`.
    Keyed
}

impl Keyed {
    /// The key: `"spacy key"`.
    pub fn key(&self) -> EcoString {
        self.0
            .children()
            .find_map(|node| match node.kind() {
                NodeKind::Str(key) => Some(key.clone()),
                _ => None,
            })
            .expect("keyed pair is missing key")
    }

    /// The right-hand side of the pair: `true`.
    pub fn expr(&self) -> Expr {
        self.0.cast_last_child().expect("keyed pair is missing expression")
    }
}

node! {
    /// A unary operation: `-x`.
    Unary
}

impl Unary {
    /// The operator: `-`.
    pub fn op(&self) -> UnOp {
        self.0
            .children()
            .find_map(|node| UnOp::from_token(node.kind()))
            .expect("unary operation is missing operator")
    }

    /// The expression to operate on: `x`.
    pub fn expr(&self) -> Expr {
        self.0.cast_last_child().expect("unary operation is missing child")
    }
}

/// A unary operator.
#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
pub enum UnOp {
    /// The plus operator: `+`.
    Pos,
    /// The negation operator: `-`.
    Neg,
    /// The boolean `not`.
    Not,
}

impl UnOp {
    /// Try to convert the token into a unary operation.
    pub fn from_token(token: &NodeKind) -> Option<Self> {
        Some(match token {
            NodeKind::Plus => Self::Pos,
            NodeKind::Minus => Self::Neg,
            NodeKind::Not => Self::Not,
            _ => return None,
        })
    }

    /// The precedence of this operator.
    pub fn precedence(self) -> usize {
        match self {
            Self::Pos | Self::Neg => 7,
            Self::Not => 4,
        }
    }

    /// The string representation of this operation.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Pos => "+",
            Self::Neg => "-",
            Self::Not => "not",
        }
    }
}

node! {
    /// A binary operation: `a + b`.
    Binary
}

impl Binary {
    /// The binary operator: `+`.
    pub fn op(&self) -> BinOp {
        let mut not = false;
        self.0
            .children()
            .find_map(|node| match node.kind() {
                NodeKind::Not => {
                    not = true;
                    None
                }
                NodeKind::In if not => Some(BinOp::NotIn),
                _ => BinOp::from_token(node.kind()),
            })
            .expect("binary operation is missing operator")
    }

    /// The left-hand side of the operation: `a`.
    pub fn lhs(&self) -> Expr {
        self.0
            .cast_first_child()
            .expect("binary operation is missing left-hand side")
    }

    /// The right-hand side of the operation: `b`.
    pub fn rhs(&self) -> Expr {
        self.0
            .cast_last_child()
            .expect("binary operation is missing right-hand side")
    }
}

/// A binary operator.
#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
pub enum BinOp {
    /// The addition operator: `+`.
    Add,
    /// The subtraction operator: `-`.
    Sub,
    /// The multiplication operator: `*`.
    Mul,
    /// The division operator: `/`.
    Div,
    /// The short-circuiting boolean `and`.
    And,
    /// The short-circuiting boolean `or`.
    Or,
    /// The equality operator: `==`.
    Eq,
    /// The inequality operator: `!=`.
    Neq,
    /// The less-than operator: `<`.
    Lt,
    /// The less-than or equal operator: `<=`.
    Leq,
    /// The greater-than operator: `>`.
    Gt,
    /// The greater-than or equal operator: `>=`.
    Geq,
    /// The assignment operator: `=`.
    Assign,
    /// The containment operator: `in`.
    In,
    /// The inversed containment operator: `not in`.
    NotIn,
    /// The add-assign operator: `+=`.
    AddAssign,
    /// The subtract-assign oeprator: `-=`.
    SubAssign,
    /// The multiply-assign operator: `*=`.
    MulAssign,
    /// The divide-assign operator: `/=`.
    DivAssign,
}

impl BinOp {
    /// Try to convert the token into a binary operation.
    pub fn from_token(token: &NodeKind) -> Option<Self> {
        Some(match token {
            NodeKind::Plus => Self::Add,
            NodeKind::Minus => Self::Sub,
            NodeKind::Star => Self::Mul,
            NodeKind::Slash => Self::Div,
            NodeKind::And => Self::And,
            NodeKind::Or => Self::Or,
            NodeKind::EqEq => Self::Eq,
            NodeKind::ExclEq => Self::Neq,
            NodeKind::Lt => Self::Lt,
            NodeKind::LtEq => Self::Leq,
            NodeKind::Gt => Self::Gt,
            NodeKind::GtEq => Self::Geq,
            NodeKind::Eq => Self::Assign,
            NodeKind::In => Self::In,
            NodeKind::PlusEq => Self::AddAssign,
            NodeKind::HyphEq => Self::SubAssign,
            NodeKind::StarEq => Self::MulAssign,
            NodeKind::SlashEq => Self::DivAssign,
            _ => return None,
        })
    }

    /// The precedence of this operator.
    pub fn precedence(self) -> usize {
        match self {
            Self::Mul => 6,
            Self::Div => 6,
            Self::Add => 5,
            Self::Sub => 5,
            Self::Eq => 4,
            Self::Neq => 4,
            Self::Lt => 4,
            Self::Leq => 4,
            Self::Gt => 4,
            Self::Geq => 4,
            Self::In => 4,
            Self::NotIn => 4,
            Self::And => 3,
            Self::Or => 2,
            Self::Assign => 1,
            Self::AddAssign => 1,
            Self::SubAssign => 1,
            Self::MulAssign => 1,
            Self::DivAssign => 1,
        }
    }

    /// The associativity of this operator.
    pub fn assoc(self) -> Assoc {
        match self {
            Self::Add => Assoc::Left,
            Self::Sub => Assoc::Left,
            Self::Mul => Assoc::Left,
            Self::Div => Assoc::Left,
            Self::And => Assoc::Left,
            Self::Or => Assoc::Left,
            Self::Eq => Assoc::Left,
            Self::Neq => Assoc::Left,
            Self::Lt => Assoc::Left,
            Self::Leq => Assoc::Left,
            Self::Gt => Assoc::Left,
            Self::Geq => Assoc::Left,
            Self::In => Assoc::Left,
            Self::NotIn => Assoc::Left,
            Self::Assign => Assoc::Right,
            Self::AddAssign => Assoc::Right,
            Self::SubAssign => Assoc::Right,
            Self::MulAssign => Assoc::Right,
            Self::DivAssign => Assoc::Right,
        }
    }

    /// The string representation of this operation.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Add => "+",
            Self::Sub => "-",
            Self::Mul => "*",
            Self::Div => "/",
            Self::And => "and",
            Self::Or => "or",
            Self::Eq => "==",
            Self::Neq => "!=",
            Self::Lt => "<",
            Self::Leq => "<=",
            Self::Gt => ">",
            Self::Geq => ">=",
            Self::In => "in",
            Self::NotIn => "not in",
            Self::Assign => "=",
            Self::AddAssign => "+=",
            Self::SubAssign => "-=",
            Self::MulAssign => "*=",
            Self::DivAssign => "/=",
        }
    }
}

/// The associativity of a binary operator.
#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
pub enum Assoc {
    /// Left-associative: `a + b + c` is equivalent to `(a + b) + c`.
    Left,
    /// Right-associative: `a = b = c` is equivalent to `a = (b = c)`.
    Right,
}

node! {
    /// A field access: `properties.age`.
    FieldAccess
}

impl FieldAccess {
    /// The expression to access the field on.
    pub fn target(&self) -> Expr {
        self.0.cast_first_child().expect("field access is missing object")
    }

    /// The name of the field.
    pub fn field(&self) -> Ident {
        self.0.cast_last_child().expect("field access is missing name")
    }
}

node! {
    /// An invocation of a function: `f(x, y)`.
    FuncCall
}

impl FuncCall {
    /// The function to call.
    pub fn callee(&self) -> Expr {
        self.0.cast_first_child().expect("function call is missing callee")
    }

    /// The arguments to the function.
    pub fn args(&self) -> Args {
        self.0
            .cast_last_child()
            .expect("function call is missing argument list")
    }
}

node! {
    /// An invocation of a method: `array.push(v)`.
    MethodCall
}

impl MethodCall {
    /// The expresion to call the method on.
    pub fn target(&self) -> Expr {
        self.0.cast_first_child().expect("method call is missing target")
    }

    /// The name of the method.
    pub fn method(&self) -> Ident {
        self.0.cast_last_child().expect("method call is missing name")
    }

    /// The arguments to the method.
    pub fn args(&self) -> Args {
        self.0
            .cast_last_child()
            .expect("method call is missing argument list")
    }
}

node! {
    /// The arguments to a function: `12, draw: false`.
    Args
}

impl Args {
    /// The positional and named arguments.
    pub fn items(&self) -> impl DoubleEndedIterator<Item = Arg> + '_ {
        self.0.children().filter_map(SyntaxNode::cast)
    }
}

/// An argument to a function call.
#[derive(Debug, Clone, PartialEq, Hash)]
pub enum Arg {
    /// A positional argument: `12`.
    Pos(Expr),
    /// A named argument: `draw: false`.
    Named(Named),
    /// A spreaded argument: `..things`.
    Spread(Expr),
}

impl TypedNode for Arg {
    fn from_untyped(node: &SyntaxNode) -> Option<Self> {
        match node.kind() {
            NodeKind::Named => node.cast().map(Self::Named),
            NodeKind::Spread => node.cast_first_child().map(Self::Spread),
            _ => node.cast().map(Self::Pos),
        }
    }

    fn as_untyped(&self) -> &SyntaxNode {
        match self {
            Self::Pos(v) => v.as_untyped(),
            Self::Named(v) => v.as_untyped(),
            Self::Spread(v) => v.as_untyped(),
        }
    }
}

node! {
    /// A closure expression: `(x, y) => z`.
    Closure
}

impl Closure {
    /// The name of the closure.
    ///
    /// This only exists if you use the function syntax sugar: `let f(x) = y`.
    pub fn name(&self) -> Option<Ident> {
        self.0.cast_first_child()
    }

    /// The parameter bindings.
    pub fn params(&self) -> impl DoubleEndedIterator<Item = Param> + '_ {
        self.0
            .children()
            .find(|x| x.kind() == &NodeKind::Params)
            .expect("closure is missing parameter list")
            .children()
            .filter_map(SyntaxNode::cast)
    }

    /// The body of the closure.
    pub fn body(&self) -> Expr {
        self.0.cast_last_child().expect("closure is missing body")
    }
}

/// A parameter to a closure.
#[derive(Debug, Clone, PartialEq, Hash)]
pub enum Param {
    /// A positional parameter: `x`.
    Pos(Ident),
    /// A named parameter with a default value: `draw: false`.
    Named(Named),
    /// An argument sink: `..args`.
    Sink(Ident),
}

impl TypedNode for Param {
    fn from_untyped(node: &SyntaxNode) -> Option<Self> {
        match node.kind() {
            NodeKind::Ident(_) => node.cast().map(Self::Pos),
            NodeKind::Named => node.cast().map(Self::Named),
            NodeKind::Spread => node.cast_first_child().map(Self::Sink),
            _ => None,
        }
    }

    fn as_untyped(&self) -> &SyntaxNode {
        match self {
            Self::Pos(v) => v.as_untyped(),
            Self::Named(v) => v.as_untyped(),
            Self::Sink(v) => v.as_untyped(),
        }
    }
}

node! {
    /// A let binding: `let x = 1`.
    LetBinding
}

impl LetBinding {
    /// The binding to assign to.
    pub fn binding(&self) -> Ident {
        match self.0.cast_first_child() {
            Some(Expr::Ident(binding)) => binding,
            Some(Expr::Closure(closure)) => {
                closure.name().expect("let-bound closure is missing name")
            }
            _ => panic!("let is missing binding"),
        }
    }

    /// The expression the binding is initialized with.
    pub fn init(&self) -> Option<Expr> {
        if self.0.cast_first_child::<Ident>().is_some() {
            // This is a normal binding like `let x = 1`.
            self.0.children().filter_map(SyntaxNode::cast).nth(1)
        } else {
            // This is a closure binding like `let f(x) = 1`.
            self.0.cast_first_child()
        }
    }
}

node! {
    /// A set rule: `set text(...)`.
    SetRule
}

impl SetRule {
    /// The function to set style properties for.
    pub fn target(&self) -> Ident {
        self.0.cast_first_child().expect("set rule is missing target")
    }

    /// The style properties to set.
    pub fn args(&self) -> Args {
        self.0.cast_last_child().expect("set rule is missing argument list")
    }
}

node! {
    /// A show rule: `show node: heading as [*{nody.body}*]`.
    ShowRule
}

impl ShowRule {
    /// The binding to assign to.
    pub fn binding(&self) -> Option<Ident> {
        let mut children = self.0.children();
        children
            .find_map(SyntaxNode::cast)
            .filter(|_| children.any(|child| child.kind() == &NodeKind::Colon))
    }

    /// The pattern that this rule matches.
    pub fn pattern(&self) -> Expr {
        self.0
            .children()
            .rev()
            .skip_while(|child| child.kind() != &NodeKind::As)
            .find_map(SyntaxNode::cast)
            .expect("show rule is missing pattern")
    }

    /// The expression that realizes the node.
    pub fn body(&self) -> Expr {
        self.0.cast_last_child().expect("show rule is missing body")
    }
}

node! {
    /// A wrap rule: `wrap body in columns(2, body)`.
    WrapRule
}

impl WrapRule {
    /// The binding to assign the remaining markup to.
    pub fn binding(&self) -> Ident {
        self.0.cast_first_child().expect("wrap rule is missing binding")
    }

    /// The expression to evaluate.
    pub fn body(&self) -> Expr {
        self.0.cast_last_child().expect("wrap rule is missing body")
    }
}

node! {
    /// An if-else conditional: `if x { y } else { z }`.
    Conditional
}

impl Conditional {
    /// The condition which selects the body to evaluate.
    pub fn condition(&self) -> Expr {
        self.0.cast_first_child().expect("conditional is missing condition")
    }

    /// The expression to evaluate if the condition is true.
    pub fn if_body(&self) -> Expr {
        self.0
            .children()
            .filter_map(SyntaxNode::cast)
            .nth(1)
            .expect("conditional is missing body")
    }

    /// The expression to evaluate if the condition is false.
    pub fn else_body(&self) -> Option<Expr> {
        self.0.children().filter_map(SyntaxNode::cast).nth(2)
    }
}

node! {
    /// A while loop: `while x { y }`.
    WhileLoop
}

impl WhileLoop {
    /// The condition which selects whether to evaluate the body.
    pub fn condition(&self) -> Expr {
        self.0.cast_first_child().expect("while loop is missing condition")
    }

    /// The expression to evaluate while the condition is true.
    pub fn body(&self) -> Expr {
        self.0.cast_last_child().expect("while loop is missing body")
    }
}

node! {
    /// A for loop: `for x in y { z }`.
    ForLoop
}

impl ForLoop {
    /// The pattern to assign to.
    pub fn pattern(&self) -> ForPattern {
        self.0.cast_first_child().expect("for loop is missing pattern")
    }

    /// The expression to iterate over.
    pub fn iter(&self) -> Expr {
        self.0.cast_first_child().expect("for loop is missing iterable")
    }

    /// The expression to evaluate for each iteration.
    pub fn body(&self) -> Expr {
        self.0.cast_last_child().expect("for loop is missing body")
    }
}

node! {
    /// A for-in loop: `for x in y { z }`.
    ForPattern
}

impl ForPattern {
    /// The key part of the pattern: index for arrays, name for dictionaries.
    pub fn key(&self) -> Option<Ident> {
        let mut children = self.0.children().filter_map(SyntaxNode::cast);
        let key = children.next();
        if children.next().is_some() {
            key
        } else {
            None
        }
    }

    /// The value part of the pattern.
    pub fn value(&self) -> Ident {
        self.0.cast_last_child().expect("for loop pattern is missing value")
    }
}

node! {
    /// A module import: `import a, b, c from "utils.typ"`.
    ModuleImport
}

impl ModuleImport {
    /// The items to be imported.
    pub fn imports(&self) -> Imports {
        self.0
            .children()
            .find_map(|node| match node.kind() {
                NodeKind::Star => Some(Imports::Wildcard),
                NodeKind::ImportItems => {
                    let items = node.children().filter_map(SyntaxNode::cast).collect();
                    Some(Imports::Items(items))
                }
                _ => None,
            })
            .expect("module import is missing items")
    }

    /// The path to the file that should be imported.
    pub fn path(&self) -> Expr {
        self.0.cast_last_child().expect("module import is missing path")
    }
}

/// The items that ought to be imported from a file.
#[derive(Debug, Clone, PartialEq, Hash)]
pub enum Imports {
    /// All items in the scope of the file should be imported.
    Wildcard,
    /// The specified items from the file should be imported.
    Items(Vec<Ident>),
}

node! {
    /// A module include: `include "chapter1.typ"`.
    ModuleInclude
}

impl ModuleInclude {
    /// The path to the file that should be included.
    pub fn path(&self) -> Expr {
        self.0.cast_last_child().expect("module include is missing path")
    }
}

node! {
    /// A break expression: `break`.
    BreakStmt
}

node! {
    /// A continue expression: `continue`.
    ContinueStmt
}

node! {
    /// A return expression: `return`, `return x + 1`.
    ReturnStmt
}

impl ReturnStmt {
    /// The expression to return.
    pub fn body(&self) -> Option<Expr> {
        self.0.cast_last_child()
    }
}

node! {
    /// An identifier.
    Ident
}

impl Ident {
    /// Get the identifier.
    pub fn get(&self) -> &EcoString {
        match self.0.kind() {
            NodeKind::Ident(id) => id,
            _ => panic!("identifier is of wrong kind"),
        }
    }

    /// Take out the container identifier.
    pub fn take(self) -> EcoString {
        match self.0 {
            SyntaxNode::Leaf(NodeData { kind: NodeKind::Ident(id), .. }) => id,
            _ => panic!("identifier is of wrong kind"),
        }
    }
}

impl Deref for Ident {
    type Target = str;

    fn deref(&self) -> &Self::Target {
        self.get()
    }
}
