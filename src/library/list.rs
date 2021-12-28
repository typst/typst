//! Unordered (bulleted) and ordered (numbered) lists.

use std::hash::Hash;

use super::prelude::*;
use super::{GridNode, TextNode, TrackSizing};

/// An unordered or ordered list.
#[derive(Debug, Hash)]
pub struct ListNode<L> {
    /// The node that produces the item's body.
    pub child: PackedNode,
    /// The list labelling style -- unordered or ordered.
    pub labelling: L,
}

#[properties]
impl<L: Labelling> ListNode<L> {
    /// The indentation of each item's label.
    pub const LABEL_INDENT: Linear = Relative::new(0.0).into();
    /// The space between the label and the body of each item.
    pub const BODY_INDENT: Linear = Relative::new(0.5).into();
}

impl<L: Labelling> Construct for ListNode<L> {
    fn construct(_: &mut EvalContext, args: &mut Args) -> TypResult<Node> {
        Ok(args
            .all()
            .map(|node: Node| {
                Node::block(Self {
                    child: node.into_block(),
                    labelling: L::default(),
                })
            })
            .sum())
    }
}

impl<L: Labelling> Set for ListNode<L> {
    fn set(args: &mut Args, styles: &mut Styles) -> TypResult<()> {
        styles.set_opt(Self::LABEL_INDENT, args.named("label-indent")?);
        styles.set_opt(Self::BODY_INDENT, args.named("body-indent")?);
        Ok(())
    }
}

impl<L: Labelling> Layout for ListNode<L> {
    fn layout(
        &self,
        ctx: &mut LayoutContext,
        regions: &Regions,
    ) -> Vec<Constrained<Rc<Frame>>> {
        let em = ctx.styles.get(TextNode::SIZE).abs;
        let label_indent = ctx.styles.get(Self::LABEL_INDENT).resolve(em);
        let body_indent = ctx.styles.get(Self::BODY_INDENT).resolve(em);

        let columns = vec![
            TrackSizing::Linear(label_indent.into()),
            TrackSizing::Auto,
            TrackSizing::Linear(body_indent.into()),
            TrackSizing::Auto,
        ];

        let children = vec![
            PackedNode::default(),
            Node::Text(self.labelling.label()).into_block(),
            PackedNode::default(),
            self.child.clone(),
        ];

        GridNode {
            tracks: Spec::new(columns, vec![]),
            gutter: Spec::default(),
            children,
        }
        .layout(ctx, regions)
    }
}

/// How to label a list.
pub trait Labelling: Debug + Default + Hash + 'static {
    /// Return the item's label.
    fn label(&self) -> EcoString;
}

/// Unordered list labelling style.
#[derive(Debug, Default, Hash)]
pub struct Unordered;

impl Labelling for Unordered {
    fn label(&self) -> EcoString {
        '•'.into()
    }
}

/// Ordered list labelling style.
#[derive(Debug, Default, Hash)]
pub struct Ordered(pub Option<usize>);

impl Labelling for Ordered {
    fn label(&self) -> EcoString {
        format_eco!("{}.", self.0.unwrap_or(1))
    }
}
