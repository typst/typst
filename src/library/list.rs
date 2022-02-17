//! Unordered (bulleted) and ordered (numbered) lists.

use super::prelude::*;
use super::{GridNode, Numbering, ParNode, TextNode, TrackSizing};

use crate::parse::Scanner;

/// An unordered or ordered list.
#[derive(Debug, Hash)]
pub struct ListNode<const L: ListKind> {
    /// The individual bulleted or numbered items.
    pub items: Vec<ListItem>,
    /// If true, there is paragraph spacing between the items, if false
    /// there is list spacing between the items.
    pub wide: bool,
    /// Where the list starts.
    pub start: usize,
}

/// An item in a list.
#[derive(Debug, Clone, PartialEq, Hash)]
pub struct ListItem {
    /// The number of the item.
    pub number: Option<usize>,
    /// The node that produces the item's body.
    pub body: LayoutNode,
}

#[class]
impl<const L: ListKind> ListNode<L> {
    /// How the list is labelled.
    pub const LABEL: Label = Label::Default;
    /// The spacing between the list items of a non-wide list.
    pub const SPACING: Linear = Linear::zero();
    /// The indentation of each item's label.
    pub const LABEL_INDENT: Linear = Relative::new(0.0).into();
    /// The space between the label and the body of each item.
    pub const BODY_INDENT: Linear = Relative::new(0.5).into();

    fn construct(_: &mut Vm, args: &mut Args) -> TypResult<Template> {
        Ok(Template::show(Self {
            items: args
                .all()?
                .into_iter()
                .map(|body| ListItem { number: None, body })
                .collect(),
            wide: args.named("wide")?.unwrap_or(false),
            start: args.named("start")?.unwrap_or(0),
        }))
    }
}

impl<const L: ListKind> Show for ListNode<L> {
    fn show(&self, vm: &mut Vm, styles: StyleChain) -> TypResult<Template> {
        let mut children = vec![];
        let mut number = self.start;

        let label = styles.get_ref(Self::LABEL);

        for item in &self.items {
            number = item.number.unwrap_or(number);
            if L == UNORDERED {
                number = 1;
            }

            children.push(LayoutNode::default());
            children.push(label.resolve(vm, L, number)?.pack());
            children.push(LayoutNode::default());
            children.push(item.body.clone());
            number += 1;
        }

        let em = styles.get(TextNode::SIZE).abs;
        let leading = styles.get(ParNode::LEADING);
        let spacing = if self.wide {
            styles.get(ParNode::SPACING)
        } else {
            styles.get(Self::SPACING)
        };

        let gutter = (leading + spacing).resolve(em);
        let label_indent = styles.get(Self::LABEL_INDENT).resolve(em);
        let body_indent = styles.get(Self::BODY_INDENT).resolve(em);

        Ok(Template::block(GridNode {
            tracks: Spec::with_x(vec![
                TrackSizing::Linear(label_indent.into()),
                TrackSizing::Auto,
                TrackSizing::Linear(body_indent.into()),
                TrackSizing::Auto,
            ]),
            gutter: Spec::with_y(vec![TrackSizing::Linear(gutter.into())]),
            children,
        }))
    }
}

impl<const L: ListKind> From<ListItem> for ListNode<L> {
    fn from(item: ListItem) -> Self {
        Self { items: vec![item], wide: false, start: 1 }
    }
}

/// How to label a list.
pub type ListKind = usize;

/// Unordered list labelling style.
pub const UNORDERED: ListKind = 0;

/// Ordered list labelling style.
pub const ORDERED: ListKind = 1;

/// Either a template or a closure mapping to a template.
#[derive(Debug, Clone, PartialEq, Hash)]
pub enum Label {
    /// The default labelling.
    Default,
    /// A pattern with prefix, numbering, lower / upper case and suffix.
    Pattern(EcoString, Numbering, bool, EcoString),
    /// A bare template.
    Template(Template),
    /// A simple mapping from an item number to a template.
    Mapping(fn(usize) -> Template),
    /// A closure mapping from an item number to a value.
    Func(Func, Span),
}

impl Label {
    /// Resolve the value based on the level.
    pub fn resolve(
        &self,
        vm: &mut Vm,
        kind: ListKind,
        number: usize,
    ) -> TypResult<Template> {
        Ok(match self {
            Self::Default => match kind {
                UNORDERED => Template::Text('•'.into()),
                ORDERED | _ => Template::Text(format_eco!("{}.", number)),
            },
            Self::Pattern(prefix, numbering, upper, suffix) => {
                let fmt = numbering.apply(number);
                let mid = if *upper { fmt.to_uppercase() } else { fmt.to_lowercase() };
                Template::Text(format_eco!("{}{}{}", prefix, mid, suffix))
            }
            Self::Template(template) => template.clone(),
            Self::Mapping(mapping) => mapping(number),
            &Self::Func(ref func, span) => {
                let args = Args {
                    span,
                    items: vec![Arg {
                        span,
                        name: None,
                        value: Spanned::new(Value::Int(number as i64), span),
                    }],
                };
                func.call(vm, args)?.cast().at(span)?
            }
        })
    }
}

impl Cast<Spanned<Value>> for Label {
    fn is(value: &Spanned<Value>) -> bool {
        matches!(&value.v, Value::Template(_) | Value::Func(_))
    }

    fn cast(value: Spanned<Value>) -> StrResult<Self> {
        match value.v {
            Value::Str(pattern) => {
                let mut s = Scanner::new(&pattern);
                let mut prefix;
                let numbering = loop {
                    prefix = s.eaten();
                    match s.eat().map(|c| c.to_ascii_lowercase()) {
                        Some('1') => break Numbering::Arabic,
                        Some('a') => break Numbering::Letter,
                        Some('i') => break Numbering::Roman,
                        Some('*') => break Numbering::Symbol,
                        Some(_) => {}
                        None => Err("invalid pattern")?,
                    }
                };
                let upper = s.prev(0).map_or(false, char::is_uppercase);
                let suffix = s.rest().into();
                Ok(Self::Pattern(prefix.into(), numbering, upper, suffix))
            }
            Value::Template(v) => Ok(Self::Template(v)),
            Value::Func(v) => Ok(Self::Func(v, value.span)),
            _ => Err("expected pattern, template or function")?,
        }
    }
}
