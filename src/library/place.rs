//! Absolute placement of nodes.

use super::prelude::*;
use super::AlignNode;

/// Place a node at an absolute position.
#[derive(Debug, Hash)]
pub struct PlaceNode(pub LayoutNode);

#[class]
impl PlaceNode {
    fn construct(_: &mut Vm, args: &mut Args) -> TypResult<Template> {
        let aligns = args.find()?.unwrap_or(Spec::with_x(Some(Align::Left)));
        let tx = args.named("dx")?.unwrap_or_default();
        let ty = args.named("dy")?.unwrap_or_default();
        let body: LayoutNode = args.expect("body")?;
        Ok(Template::block(Self(
            body.moved(Point::new(tx, ty)).aligned(aligns),
        )))
    }
}

impl Layout for PlaceNode {
    fn layout(
        &self,
        vm: &mut Vm,
        regions: &Regions,
        styles: StyleChain,
    ) -> Vec<Constrained<Arc<Frame>>> {
        let out_of_flow = self.out_of_flow();

        // The pod is the base area of the region because for absolute
        // placement we don't really care about the already used area (current).
        let pod = {
            let finite = regions.base.map(Length::is_finite);
            let expand = finite & (regions.expand | out_of_flow);
            Regions::one(regions.base, regions.base, expand)
        };

        let mut frames = self.0.layout(vm, &pod, styles);
        let Constrained { item: frame, cts } = &mut frames[0];

        // If expansion is off, zero all sizes so that we don't take up any
        // space in our parent. Otherwise, respect the expand settings.
        let target = regions.expand.select(regions.current, Size::zero());
        Arc::make_mut(frame).resize(target, Align::LEFT_TOP);

        // Set base constraint because our pod size is base and exact
        // constraints if we needed to expand or offset.
        *cts = Constraints::new(regions.expand);
        cts.base = regions.base.map(Some);
        cts.exact = regions.current.filter(regions.expand | out_of_flow);

        frames
    }
}

impl PlaceNode {
    /// Whether this node wants to be placed relative to its its parent's base
    /// origin. instead of relative to the parent's current flow/cursor
    /// position.
    pub fn out_of_flow(&self) -> bool {
        self.0
            .downcast::<AlignNode>()
            .map_or(false, |node| node.aligns.y.is_some())
    }
}
