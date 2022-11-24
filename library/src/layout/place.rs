use super::AlignNode;
use crate::prelude::*;

/// Place content at an absolute position.
#[derive(Debug, Hash)]
pub struct PlaceNode(pub Content);

#[node(LayoutBlock, Behave)]
impl PlaceNode {
    fn construct(_: &Vm, args: &mut Args) -> SourceResult<Content> {
        let aligns = args.find()?.unwrap_or(Axes::with_x(Some(GenAlign::Start)));
        let dx = args.named("dx")?.unwrap_or_default();
        let dy = args.named("dy")?.unwrap_or_default();
        let body = args.expect::<Content>("body")?;
        Ok(Self(body.moved(Axes::new(dx, dy)).aligned(aligns)).pack())
    }
}

impl LayoutBlock for PlaceNode {
    fn layout_block(
        &self,
        world: Tracked<dyn World>,
        regions: &Regions,
        styles: StyleChain,
    ) -> SourceResult<Vec<Frame>> {
        let out_of_flow = self.out_of_flow();

        // The pod is the base area of the region because for absolute
        // placement we don't really care about the already used area.
        let pod = {
            let finite = regions.base.map(Abs::is_finite);
            let expand = finite & (regions.expand | out_of_flow);
            Regions::one(regions.base, regions.base, expand)
        };

        let mut frames = self.0.layout_block(world, &pod, styles)?;

        // If expansion is off, zero all sizes so that we don't take up any
        // space in our parent. Otherwise, respect the expand settings.
        let target = regions.expand.select(regions.first, Size::zero());
        frames[0].resize(target, Align::LEFT_TOP);

        Ok(frames)
    }
}

impl PlaceNode {
    /// Whether this node wants to be placed relative to its its parent's base
    /// origin. Instead of relative to the parent's current flow/cursor
    /// position.
    pub fn out_of_flow(&self) -> bool {
        self.0.to::<AlignNode>().map_or(false, |node| node.aligns.y.is_some())
    }
}

impl Behave for PlaceNode {
    fn behaviour(&self) -> Behaviour {
        Behaviour::Ignorant
    }
}
