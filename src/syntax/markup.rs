use super::{Expr, Ident, NodeKind, RedNode, RedRef, Span, TypedNode};
use crate::node;
use crate::util::EcoString;

node! {
    /// The syntactical root capable of representing a full parsed document.
    Markup
}

impl Markup {
    pub fn nodes<'a>(&'a self) -> impl Iterator<Item = MarkupNode> + 'a {
        self.0.children().filter_map(RedRef::cast)
    }
}

/// A single piece of markup.
#[derive(Debug, Clone, PartialEq)]
pub enum MarkupNode {
    /// Whitespace containing less than two newlines.
    Space,
    /// A forced line break: `\`.
    Linebreak,
    /// A paragraph break: Two or more newlines.
    Parbreak,
    /// Strong text was enabled / disabled: `*`.
    Strong,
    /// Emphasized text was enabled / disabled: `_`.
    Emph,
    /// Plain text.
    Text(EcoString),
    /// A raw block with optional syntax highlighting: `` `...` ``.
    Raw(RawNode),
    /// A section heading: `= Introduction`.
    Heading(HeadingNode),
    /// An item in an unordered list: `- ...`.
    List(ListNode),
    /// An item in an enumeration (ordered list): `1. ...`.
    Enum(EnumNode),
    /// An expression.
    Expr(Expr),
}

impl TypedNode for MarkupNode {
    fn cast_from(node: RedRef) -> Option<Self> {
        match node.kind() {
            NodeKind::Space(_) => Some(MarkupNode::Space),
            NodeKind::Linebreak => Some(MarkupNode::Linebreak),
            NodeKind::Parbreak => Some(MarkupNode::Parbreak),
            NodeKind::Strong => Some(MarkupNode::Strong),
            NodeKind::Emph => Some(MarkupNode::Emph),
            NodeKind::Text(s) => Some(MarkupNode::Text(s.clone())),
            NodeKind::UnicodeEscape(u) => Some(MarkupNode::Text(u.character.into())),
            NodeKind::EnDash => Some(MarkupNode::Text(EcoString::from("\u{2013}"))),
            NodeKind::EmDash => Some(MarkupNode::Text(EcoString::from("\u{2014}"))),
            NodeKind::NonBreakingSpace => {
                Some(MarkupNode::Text(EcoString::from("\u{00A0}")))
            }
            NodeKind::Raw(_) => node.cast().map(MarkupNode::Raw),
            NodeKind::Heading => node.cast().map(MarkupNode::Heading),
            NodeKind::List => node.cast().map(MarkupNode::List),
            NodeKind::Enum => node.cast().map(MarkupNode::Enum),
            NodeKind::Error(_, _) => None,
            _ => node.cast().map(MarkupNode::Expr),
        }
    }
}

/// A raw block with optional syntax highlighting: `` `...` ``.
#[derive(Debug, Clone, PartialEq)]
pub struct RawNode {
    /// An optional identifier specifying the language to syntax-highlight in.
    pub lang: Option<Ident>,
    /// The raw text, determined as the raw string between the backticks trimmed
    /// according to the above rules.
    pub text: EcoString,
    /// Whether the element is block-level, that is, it has 3+ backticks
    /// and contains at least one newline.
    pub block: bool,
}

impl TypedNode for RawNode {
    fn cast_from(node: RedRef) -> Option<Self> {
        match node.kind() {
            NodeKind::Raw(raw) => {
                let span = node.span();
                let start = span.start + raw.backticks as usize;
                Some(Self {
                    block: raw.block,
                    lang: raw.lang.as_ref().and_then(|x| {
                        let span = Span::new(span.source, start, start + x.len());
                        Ident::new(x, span)
                    }),
                    text: raw.text.clone(),
                })
            }
            _ => None,
        }
    }
}

node! {
    /// A section heading: `= Introduction`.
    Heading => HeadingNode
}

impl HeadingNode {
    /// The contents of the heading.
    pub fn body(&self) -> Markup {
        self.0
            .cast_first_child()
            .expect("heading node is missing markup body")
    }

    /// The section depth (numer of equals signs).
    pub fn level(&self) -> u8 {
        self.0
            .children()
            .find_map(|node| match node.kind() {
                NodeKind::HeadingLevel(heading) => Some(*heading),
                _ => None,
            })
            .expect("heading node is missing heading level")
    }
}

node! {
    /// An item in an unordered list: `- ...`.
    List => ListNode
}

impl ListNode {
    /// The contents of the list item.
    pub fn body(&self) -> Markup {
        self.0.cast_first_child().expect("list node is missing body")
    }
}

node! {
    /// An item in an enumeration (ordered list): `1. ...`.
    Enum => EnumNode
}

impl EnumNode {
    /// The contents of the list item.
    pub fn body(&self) -> Markup {
        self.0.cast_first_child().expect("enumeration node is missing body")
    }

    /// The number, if any.
    pub fn number(&self) -> Option<usize> {
        self.0
            .children()
            .find_map(|node| match node.kind() {
                NodeKind::EnumNumbering(num) => Some(num.clone()),
                _ => None,
            })
            .expect("enumeration node is missing number")
    }
}
