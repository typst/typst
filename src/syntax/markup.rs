use super::{Expr, Ident, NodeKind, RedNode, RedTicket, Span, TypedNode};
use crate::node;
use crate::util::EcoString;
use std::fmt::Write;

/// The syntactical root capable of representing a full parsed document.
pub type Markup = Vec<MarkupNode>;

impl TypedNode for Markup {
    fn cast_from(node: RedTicket) -> Option<Self> {
        if node.kind() != &NodeKind::Markup {
            return None;
        }

        let children = node.own().children().filter_map(TypedNode::cast_from).collect();
        Some(children)
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
    fn cast_from(node: RedTicket) -> Option<Self> {
        match node.kind() {
            NodeKind::Space(_) => Some(MarkupNode::Space),
            NodeKind::Linebreak => Some(MarkupNode::Linebreak),
            NodeKind::Parbreak => Some(MarkupNode::Parbreak),
            NodeKind::Strong => Some(MarkupNode::Strong),
            NodeKind::Emph => Some(MarkupNode::Emph),
            NodeKind::Text(s) => Some(MarkupNode::Text(s.clone())),
            NodeKind::UnicodeEscape(u) => {
                Some(MarkupNode::Text(if let Some(s) = u.character {
                    s.into()
                } else {
                    let mut eco = EcoString::with_capacity(u.sequence.len() + 4);
                    write!(&mut eco, "\\u{{{}}}", u.sequence).unwrap();
                    eco
                }))
            }
            NodeKind::EnDash => Some(MarkupNode::Text(EcoString::from("\u{2013}"))),
            NodeKind::EmDash => Some(MarkupNode::Text(EcoString::from("\u{2014}"))),
            NodeKind::NonBreakingSpace => {
                Some(MarkupNode::Text(EcoString::from("\u{00A0}")))
            }
            NodeKind::Raw(_) => Some(MarkupNode::Raw(RawNode::cast_from(node).unwrap())),
            NodeKind::Heading => {
                Some(MarkupNode::Heading(HeadingNode::cast_from(node).unwrap()))
            }
            NodeKind::List => Some(MarkupNode::List(ListNode::cast_from(node).unwrap())),
            NodeKind::Enum => Some(MarkupNode::Enum(EnumNode::cast_from(node).unwrap())),
            NodeKind::Error(_, _) => None,
            _ => Some(MarkupNode::Expr(Expr::cast_from(node)?)),
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
    fn cast_from(node: RedTicket) -> Option<Self> {
        if let NodeKind::Raw(raw) = node.kind() {
            let span = node.own().span();
            let start = span.start + raw.backticks as usize;
            Some(Self {
                block: raw.block,
                lang: raw.lang.as_ref().and_then(|x| {
                    let span = Span::new(span.source, start, start + x.len());
                    Ident::new(x, span)
                }),
                text: raw.text.clone(),
            })
        } else {
            None
        }
    }
}

node!(
    /// A section heading: `= Introduction`.
    Heading => HeadingNode
);

impl HeadingNode {
    /// The contents of the heading.
    pub fn body(&self) -> Markup {
        self.0
            .cast_first_child()
            .expect("heading node is missing markup body")
    }

    /// The section depth (numer of equals signs).
    pub fn level(&self) -> HeadingLevel {
        self.0
            .cast_first_child()
            .expect("heading node is missing heading level")
    }
}

#[derive(Debug, Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct HeadingLevel(pub usize);

impl TypedNode for HeadingLevel {
    fn cast_from(node: RedTicket) -> Option<Self> {
        if let NodeKind::HeadingLevel(l) = node.kind() {
            Some(Self((*l).into()))
        } else {
            None
        }
    }
}

node!(
    /// An item in an unordered list: `- ...`.
    List => ListNode
);

impl ListNode {
    /// The contents of the list item.
    pub fn body(&self) -> Markup {
        self.0.cast_first_child().expect("list node is missing body")
    }
}

node!(
    /// An item in an enumeration (ordered list): `1. ...`.
    Enum => EnumNode
);

impl EnumNode {
    /// The contents of the list item.
    pub fn body(&self) -> Markup {
        self.0.cast_first_child().expect("enumeration node is missing body")
    }

    /// The number, if any.
    pub fn number(&self) -> EnumNumber {
        self.0.cast_first_child().expect("enumeration node is missing number")
    }
}

#[derive(Debug, Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct EnumNumber(pub Option<usize>);

impl TypedNode for EnumNumber {
    fn cast_from(node: RedTicket) -> Option<Self> {
        if let NodeKind::EnumNumbering(x) = node.kind() {
            Some(Self(*x))
        } else {
            None
        }
    }
}
