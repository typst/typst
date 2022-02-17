//! Aligning nodes in their parent container.

use super::prelude::*;
use super::ParNode;

/// Align a node along the layouting axes.
#[derive(Debug, Hash)]
pub struct AlignNode {
    /// How to align the node horizontally and vertically.
    pub aligns: Spec<Option<Align>>,
    /// The node to be aligned.
    pub child: LayoutNode,
}

#[class]
impl AlignNode {
    fn construct(_: &mut Vm, args: &mut Args) -> TypResult<Template> {
        let aligns: Spec<_> = args.find()?.unwrap_or_default();
        let body: LayoutNode = args.expect("body")?;
        Ok(Template::block(body.aligned(aligns)))
    }
}

impl Layout for AlignNode {
    fn layout(
        &self,
        vm: &mut Vm,
        regions: &Regions,
        styles: StyleChain,
    ) -> TypResult<Vec<Constrained<Arc<Frame>>>> {
        // The child only needs to expand along an axis if there's no alignment.
        let mut pod = regions.clone();
        pod.expand &= self.aligns.map_is_none();

        // Align paragraphs inside the child.
        let mut passed = StyleMap::new();
        if let Some(align) = self.aligns.x {
            passed.set(ParNode::ALIGN, align);
        }

        // Layout the child.
        let mut frames = self.child.layout(vm, &pod, passed.chain(&styles))?;

        for ((current, base), Constrained { item: frame, cts }) in
            regions.iter().zip(&mut frames)
        {
            // Align in the target size. The target size depends on whether we
            // should expand.
            let target = regions.expand.select(current, frame.size);
            let default = Spec::new(Align::Left, Align::Top);
            let aligns = self.aligns.unwrap_or(default);
            Arc::make_mut(frame).resize(target, aligns);

            // Set constraints.
            cts.expand = regions.expand;
            cts.base = base.filter(cts.base.map_is_some());
            cts.exact = current.filter(regions.expand | cts.exact.map_is_some());
        }

        Ok(frames)
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
