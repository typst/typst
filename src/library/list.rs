//! Unordered (bulleted) and ordered (numbered) lists.

use super::prelude::*;
use super::{GridNode, TextNode, TrackSizing};

/// An unordered or ordered list.
#[derive(Debug, Hash)]
pub struct ListNode<L: ListLabel> {
    /// The list label -- unordered or ordered with index.
    pub label: L,
    /// The node that produces the item's body.
    pub child: LayoutNode,
}

#[class]
impl<L: ListLabel> ListNode<L> {
    /// The indentation of each item's label.
    pub const LABEL_INDENT: Linear = Relative::new(0.0).into();
    /// The space between the label and the body of each item.
    pub const BODY_INDENT: Linear = Relative::new(0.5).into();

    fn construct(_: &mut EvalContext, args: &mut Args) -> TypResult<Template> {
        Ok(args
            .all()
            .enumerate()
            .map(|(i, child)| Template::show(Self { label: L::new(1 + i), child }))
            .sum())
    }
}

impl<L: ListLabel> Show for ListNode<L> {
    fn show(&self, styles: StyleChain) -> Template {
        let em = styles.get(TextNode::SIZE).abs;
        let label_indent = styles.get(Self::LABEL_INDENT).resolve(em);
        let body_indent = styles.get(Self::BODY_INDENT).resolve(em);

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
                Template::Text(self.label.label()).pack(),
                LayoutNode::default(),
                self.child.clone(),
            ],
        })
    }
}

/// How to label a list.
pub trait ListLabel: Debug + Default + Hash + Sync + Send + 'static {
    /// Create a new list label.
    fn new(number: usize) -> Self;

    /// Return the item's label.
    fn label(&self) -> EcoString;
}

/// Unordered list labelling style.
#[derive(Debug, Default, Hash)]
pub struct Unordered;

impl ListLabel for Unordered {
    fn new(_: usize) -> Self {
        Self
    }

    fn label(&self) -> EcoString {
        '•'.into()
    }
}

/// Ordered list labelling style.
#[derive(Debug, Default, Hash)]
pub struct Ordered(pub Option<usize>);

impl ListLabel for Ordered {
    fn new(number: usize) -> Self {
        Self(Some(number))
    }

    fn label(&self) -> EcoString {
        format_eco!("{}.", self.0.unwrap_or(1))
    }
}
