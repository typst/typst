use super::AlignNode;
use crate::library::prelude::*;

/// Place a node at an absolute position.
#[derive(Debug, Hash)]
pub struct PlaceNode(pub LayoutNode);

#[class]
impl PlaceNode {
    fn construct(_: &mut Context, args: &mut Args) -> TypResult<Content> {
        let aligns = args.find()?.unwrap_or(Spec::with_x(Some(Align::Left)));
        let tx = args.named("dx")?.unwrap_or_default();
        let ty = args.named("dy")?.unwrap_or_default();
        let body: LayoutNode = args.expect("body")?;
        Ok(Content::block(Self(
            body.moved(Point::new(tx, ty)).aligned(aligns),
        )))
    }
}

impl Layout for PlaceNode {
    fn layout(
        &self,
        ctx: &mut Context,
        regions: &Regions,
        styles: StyleChain,
    ) -> TypResult<Vec<Arc<Frame>>> {
        let out_of_flow = self.out_of_flow();

        // The pod is the base area of the region because for absolute
        // placement we don't really care about the already used area.
        let pod = {
            let finite = regions.base.map(Length::is_finite);
            let expand = finite & (regions.expand | out_of_flow);
            Regions::one(regions.base, regions.base, expand)
        };

        let mut frames = self.0.layout(ctx, &pod, styles)?;

        // If expansion is off, zero all sizes so that we don't take up any
        // space in our parent. Otherwise, respect the expand settings.
        let frame = &mut frames[0];
        let target = regions.expand.select(regions.first, Size::zero());
        Arc::make_mut(frame).resize(target, Align::LEFT_TOP);

        Ok(frames)
    }
}

impl PlaceNode {
    /// Whether this node wants to be placed relative to its its parent's base
    /// origin. Instead of relative to the parent's current flow/cursor
    /// position.
    pub fn out_of_flow(&self) -> bool {
        self.0
            .downcast::<AlignNode>()
            .map_or(false, |node| node.aligns.y.is_some())
    }
}
