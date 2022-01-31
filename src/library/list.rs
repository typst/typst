//! Unordered (bulleted) and ordered (numbered) lists.

use super::prelude::*;
use super::{GridNode, TextNode, TrackSizing};

/// An unordered or ordered list.
#[derive(Debug, Hash)]
pub struct ListNode<L: ListKind> {
    /// The list labelling style -- unordered or ordered.
    pub kind: L,
    /// The node that produces the item's body.
    pub child: PackedNode,
}

#[class]
impl<L: ListKind> ListNode<L> {
    /// The indentation of each item's label.
    pub const LABEL_INDENT: Linear = Relative::new(0.0).into();
    /// The space between the label and the body of each item.
    pub const BODY_INDENT: Linear = Relative::new(0.5).into();

    fn construct(_: &mut EvalContext, args: &mut Args) -> TypResult<Node> {
        Ok(args
            .all()
            .map(|child: PackedNode| Node::block(Self { kind: L::default(), child }))
            .sum())
    }

    fn set(args: &mut Args, styles: &mut StyleMap) -> TypResult<()> {
        styles.set_opt(Self::LABEL_INDENT, args.named("label-indent")?);
        styles.set_opt(Self::BODY_INDENT, args.named("body-indent")?);
        Ok(())
    }
}

impl<L: ListKind> Layout for ListNode<L> {
    fn layout(
        &self,
        ctx: &mut LayoutContext,
        regions: &Regions,
        styles: StyleChain,
    ) -> Vec<Constrained<Arc<Frame>>> {
        let em = styles.get(TextNode::SIZE).abs;
        let label_indent = styles.get(Self::LABEL_INDENT).resolve(em);
        let body_indent = styles.get(Self::BODY_INDENT).resolve(em);

        let grid = GridNode {
            tracks: Spec::with_x(vec![
                TrackSizing::Linear(label_indent.into()),
                TrackSizing::Auto,
                TrackSizing::Linear(body_indent.into()),
                TrackSizing::Auto,
            ]),
            gutter: Spec::default(),
            children: vec![
                PackedNode::default(),
                Node::Text(self.kind.label()).into_block(),
                PackedNode::default(),
                self.child.clone(),
            ],
        };

        grid.layout(ctx, regions, styles)
    }
}

/// How to label a list.
pub trait ListKind: Debug + Default + Hash + Sync + Send + 'static {
    /// Return the item's label.
    fn label(&self) -> EcoString;
}

/// Unordered list labelling style.
#[derive(Debug, Default, Hash)]
pub struct Unordered;

impl ListKind for Unordered {
    fn label(&self) -> EcoString {
        '•'.into()
    }
}

/// Ordered list labelling style.
#[derive(Debug, Default, Hash)]
pub struct Ordered(pub Option<usize>);

impl ListKind for Ordered {
    fn label(&self) -> EcoString {
        format_eco!("{}.", self.0.unwrap_or(1))
    }
}
