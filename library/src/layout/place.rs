use crate::prelude::*;

/// Place content at an absolute position.
#[derive(Debug, Hash)]
pub struct PlaceNode(pub Content, bool);

#[node(Layout, Behave)]
impl PlaceNode {
    fn construct(_: &Vm, args: &mut Args) -> SourceResult<Content> {
        let aligns = args.find()?.unwrap_or(Axes::with_x(Some(GenAlign::Start)));
        let dx = args.named("dx")?.unwrap_or_default();
        let dy = args.named("dy")?.unwrap_or_default();
        let body = args.expect::<Content>("body")?;
        let out_of_flow = aligns.y.is_some();
        Ok(Self(body.moved(Axes::new(dx, dy)).aligned(aligns), out_of_flow).pack())
    }
}

impl Layout for PlaceNode {
    fn layout(
        &self,
        vt: &mut Vt,
        styles: StyleChain,
        regions: Regions,
    ) -> SourceResult<Fragment> {
        let out_of_flow = self.out_of_flow();

        // The pod is the base area of the region because for absolute
        // placement we don't really care about the already used area.
        let pod = {
            let finite = regions.base.map(Abs::is_finite);
            let expand = finite & (regions.expand | out_of_flow);
            Regions::one(regions.base, regions.base, expand)
        };

        let mut frame = self.0.layout(vt, styles, pod)?.into_frame();

        // If expansion is off, zero all sizes so that we don't take up any
        // space in our parent. Otherwise, respect the expand settings.
        let target = regions.expand.select(regions.first, Size::zero());
        frame.resize(target, Align::LEFT_TOP);

        Ok(Fragment::frame(frame))
    }
}

impl PlaceNode {
    /// Whether this node wants to be placed relative to its its parent's base
    /// origin. Instead of relative to the parent's current flow/cursor
    /// position.
    pub fn out_of_flow(&self) -> bool {
        self.1
    }
}

impl Behave for PlaceNode {
    fn behaviour(&self) -> Behaviour {
        Behaviour::Ignorant
    }
}
