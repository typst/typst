use super::{Expr, Ident, Span};
use crate::util::EcoString;

/// The syntactical root of a markup file.
pub type Markup = Vec<MarkupNode>;

/// A single piece of markup.
#[derive(Debug, Clone, PartialEq)]
pub enum MarkupNode {
    /// Whitespace containing less than two newlines.
    Space,
    /// A forced line break: `\`.
    Linebreak(Span),
    /// A paragraph break: Two or more newlines.
    Parbreak(Span),
    /// Strong text was enabled / disabled: `*`.
    Strong(Span),
    /// Emphasized text was enabled / disabled: `_`.
    Emph(Span),
    /// Plain text.
    Text(EcoString),
    /// A raw block with optional syntax highlighting: `` `...` ``.
    Raw(Box<RawNode>),
    /// A section heading: `= Introduction`.
    Heading(Box<HeadingNode>),
    /// An item in an unordered list: `- ...`.
    List(Box<ListNode>),
    /// An item in an enumeration (ordered list): `1. ...`.
    Enum(Box<EnumNode>),
    /// An expression.
    Expr(Expr),
}

/// A raw block with optional syntax highlighting: `` `...` ``.
#[derive(Debug, Clone, PartialEq)]
pub struct RawNode {
    /// The source code location.
    pub span: Span,
    /// An optional identifier specifying the language to syntax-highlight in.
    pub lang: Option<Ident>,
    /// The raw text, determined as the raw string between the backticks trimmed
    /// according to the above rules.
    pub text: EcoString,
    /// Whether the element is block-level, that is, it has 3+ backticks
    /// and contains at least one newline.
    pub block: bool,
}

/// A section heading: `= Introduction`.
#[derive(Debug, Clone, PartialEq)]
pub struct HeadingNode {
    /// The source code location.
    pub span: Span,
    /// The section depth (numer of equals signs).
    pub level: usize,
    /// The contents of the heading.
    pub body: Markup,
}

/// An item in an unordered list: `- ...`.
#[derive(Debug, Clone, PartialEq)]
pub struct ListNode {
    /// The source code location.
    pub span: Span,
    /// The contents of the list item.
    pub body: Markup,
}

/// An item in an enumeration (ordered list): `1. ...`.
#[derive(Debug, Clone, PartialEq)]
pub struct EnumNode {
    /// The source code location.
    pub span: Span,
    /// The number, if any.
    pub number: Option<usize>,
    /// The contents of the list item.
    pub body: Markup,
}
