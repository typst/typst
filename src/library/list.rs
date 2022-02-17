//! Unordered (bulleted) and ordered (numbered) lists.

use super::prelude::*;
use super::{GridNode, TextNode, TrackSizing};

/// An unordered or ordered list.
#[derive(Debug, Hash)]
pub struct ListNode<const L: Labelling> {
    /// The number of the item.
    pub number: Option<usize>,
    /// The node that produces the item's body.
    pub child: LayoutNode,
}

#[class]
impl<const L: Labelling> ListNode<L> {
    /// The indentation of each item's label.
    pub const LABEL_INDENT: Linear = Relative::new(0.0).into();
    /// The space between the label and the body of each item.
    pub const BODY_INDENT: Linear = Relative::new(0.5).into();

    fn construct(_: &mut EvalContext, args: &mut Args) -> TypResult<Template> {
        Ok(args
            .all()?
            .into_iter()
            .enumerate()
            .map(|(i, child)| Template::show(Self { number: Some(1 + i), child }))
            .sum())
    }
}

impl<const L: Labelling> Show for ListNode<L> {
    fn show(&self, styles: StyleChain) -> Template {
        let em = styles.get(TextNode::SIZE).abs;
        let label_indent = styles.get(Self::LABEL_INDENT).resolve(em);
        let body_indent = styles.get(Self::BODY_INDENT).resolve(em);

        let label = match L {
            UNORDERED => '•'.into(),
            ORDERED | _ => format_eco!("{}.", self.number.unwrap_or(1)),
        };

        Template::block(GridNode {
            tracks: Spec::with_x(vec![
                TrackSizing::Linear(label_indent.into()),
                TrackSizing::Auto,
                TrackSizing::Linear(body_indent.into()),
                TrackSizing::Auto,
            ]),
            gutter: Spec::default(),
            children: vec![
                LayoutNode::default(),
                Template::Text(label).pack(),
                LayoutNode::default(),
                self.child.clone(),
            ],
        })
    }
}

/// How to label a list.
pub type Labelling = usize;

/// Unordered list labelling style.
pub const UNORDERED: Labelling = 0;

/// Ordered list labelling style.
pub const ORDERED: Labelling = 1;
