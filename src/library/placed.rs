//! Absolute placement of nodes.

use super::prelude::*;
use super::AlignNode;

/// `place`: Place content at an absolute position.
pub fn place(_: &mut EvalContext, args: &mut Args) -> TypResult<Value> {
    let aligns = args.find().unwrap_or(Spec::new(Some(Align::Left), None));
    let tx = args.named("dx")?.unwrap_or_default();
    let ty = args.named("dy")?.unwrap_or_default();
    let body: PackedNode = args.expect("body")?;
    Ok(Value::block(PlacedNode(
        body.moved(Point::new(tx, ty)).aligned(aligns),
    )))
}

/// A node that places its child absolutely.
#[derive(Debug, Hash)]
pub struct PlacedNode(pub PackedNode);

impl PlacedNode {
    /// Whether this node wants to be placed relative to its its parent's base
    /// origin. instead of relative to the parent's current flow/cursor
    /// position.
    pub fn out_of_flow(&self) -> bool {
        self.0
            .downcast::<AlignNode>()
            .map_or(false, |node| node.aligns.y.is_some())
    }
}

impl Layout for PlacedNode {
    fn layout(
        &self,
        ctx: &mut LayoutContext,
        regions: &Regions,
    ) -> Vec<Constrained<Rc<Frame>>> {
        let out_of_flow = self.out_of_flow();

        // The pod is the base area of the region because for absolute
        // placement we don't really care about the already used area (current).
        let pod = {
            let finite = regions.base.map(Length::is_finite);
            let expand = finite & (regions.expand | out_of_flow);
            Regions::one(regions.base, regions.base, expand)
        };

        let mut frames = self.0.layout(ctx, &pod);
        let Constrained { item: frame, cts } = &mut frames[0];

        // If expansion is off, zero all sizes so that we don't take up any
        // space in our parent. Otherwise, respect the expand settings.
        let target = regions.expand.select(regions.current, Size::zero());
        Rc::make_mut(frame).resize(target, Align::LEFT_TOP);

        // Set base constraint because our pod size is base and exact
        // constraints if we needed to expand or offset.
        *cts = Constraints::new(regions.expand);
        cts.base = regions.base.map(Some);
        cts.exact = regions.current.filter(regions.expand | out_of_flow);

        frames
    }
}
