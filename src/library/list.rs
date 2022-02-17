//! Unordered (bulleted) and ordered (numbered) lists.

use super::prelude::*;
use super::{GridNode, ParNode, TextNode, TrackSizing};

/// An unordered or ordered list.
#[derive(Debug, Hash)]
pub struct ListNode<const L: Labelling> {
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
impl<const L: Labelling> ListNode<L> {
    /// The indentation of each item's label.
    pub const LABEL_INDENT: Linear = Relative::new(0.0).into();
    /// The space between the label and the body of each item.
    pub const BODY_INDENT: Linear = Relative::new(0.5).into();
    /// The spacing between the list items of a non-wide list.
    pub const SPACING: Linear = Linear::zero();

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

impl<const L: Labelling> Show for ListNode<L> {
    fn show(&self, _: &mut Vm, styles: StyleChain) -> TypResult<Template> {
        let mut children = vec![];
        let mut number = self.start;

        for item in &self.items {
            number = item.number.unwrap_or(number);

            let label = match L {
                UNORDERED => '•'.into(),
                ORDERED | _ => format_eco!("{}.", number),
            };

            children.push(LayoutNode::default());
            children.push(Template::Text(label).pack());
            children.push(LayoutNode::default());
            children.push(item.body.clone());

            number += 1;
        }

        let em = styles.get(TextNode::SIZE).abs;
        let label_indent = styles.get(Self::LABEL_INDENT).resolve(em);
        let body_indent = styles.get(Self::BODY_INDENT).resolve(em);
        let leading = styles.get(ParNode::LEADING);
        let spacing = if self.wide {
            styles.get(ParNode::SPACING)
        } else {
            styles.get(Self::SPACING)
        };

        let gutter = (leading + spacing).resolve(em);
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

impl<const L: Labelling> From<ListItem> for ListNode<L> {
    fn from(item: ListItem) -> Self {
        Self { items: vec![item], wide: false, start: 1 }
    }
}

/// How to label a list.
pub type Labelling = usize;

/// Unordered list labelling style.
pub const UNORDERED: Labelling = 0;

/// Ordered list labelling style.
pub const ORDERED: Labelling = 1;
