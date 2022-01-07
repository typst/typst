//! Aligning nodes in their parent container.

use super::prelude::*;
use super::ParNode;

/// Align a node along the layouting axes.
#[derive(Debug, Hash)]
pub struct AlignNode {
    /// How to align the node horizontally and vertically.
    pub aligns: Spec<Option<Align>>,
    /// The node to be aligned.
    pub child: PackedNode,
}

#[class]
impl AlignNode {
    fn construct(_: &mut EvalContext, args: &mut Args) -> TypResult<Node> {
        let aligns: Spec<_> = args.find().unwrap_or_default();
        let body: PackedNode = args.expect("body")?;
        Ok(Node::block(body.aligned(aligns)))
    }
}

impl Layout for AlignNode {
    fn layout(
        &self,
        ctx: &mut LayoutContext,
        regions: &Regions,
        styles: StyleChain,
    ) -> Vec<Constrained<Rc<Frame>>> {
        // The child only needs to expand along an axis if there's no alignment.
        let mut pod = regions.clone();
        pod.expand &= self.aligns.map_is_none();

        // Align paragraphs inside the child.
        let mut passed = StyleMap::new();
        if let Some(align) = self.aligns.x {
            passed.set(ParNode::ALIGN, align);
        }

        // Layout the child.
        let mut frames = self.child.layout(ctx, &pod, passed.chain(&styles));

        for ((current, base), Constrained { item: frame, cts }) in
            regions.iter().zip(&mut frames)
        {
            // Align in the target size. The target size depends on whether we
            // should expand.
            let target = regions.expand.select(current, frame.size);
            let default = Spec::new(Align::Left, Align::Top);
            let aligns = self.aligns.unwrap_or(default);
            Rc::make_mut(frame).resize(target, aligns);

            // Set constraints.
            cts.expand = regions.expand;
            cts.base = base.filter(cts.base.map_is_some());
            cts.exact = current.filter(regions.expand | cts.exact.map_is_some());
        }

        frames
    }
}

dynamic! {
    Align: "alignment",
}

dynamic! {
    Spec<Align>: "2d alignment",
}

castable! {
    Spec<Option<Align>>,
    Expected: "1d or 2d alignment",
    @align: Align => {
        let mut aligns = Spec::default();
        aligns.set(align.axis(), Some(*align));
        aligns
    },
    @aligns: Spec<Align> => aligns.map(Some),

}
