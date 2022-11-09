use super::VNode;
use crate::prelude::*;

/// An inline-level container that sizes content.
#[derive(Debug, Clone, Hash)]
pub struct BoxNode {
    /// How to size the content horizontally and vertically.
    pub sizing: Axes<Option<Rel<Length>>>,
    /// The content to be sized.
    pub child: Content,
}

#[node(LayoutInline)]
impl BoxNode {
    fn construct(_: &mut Vm, args: &mut Args) -> SourceResult<Content> {
        let width = args.named("width")?;
        let height = args.named("height")?;
        let body = args.eat::<Content>()?.unwrap_or_default();
        Ok(body.boxed(Axes::new(width, height)))
    }
}

impl LayoutInline for BoxNode {
    fn layout_inline(
        &self,
        world: Tracked<dyn World>,
        regions: &Regions,
        styles: StyleChain,
    ) -> SourceResult<Vec<Frame>> {
        // The "pod" is the region into which the child will be layouted.
        let pod = {
            // Resolve the sizing to a concrete size.
            let size = self
                .sizing
                .resolve(styles)
                .zip(regions.base)
                .map(|(s, b)| s.map(|v| v.relative_to(b)))
                .unwrap_or(regions.first);

            // Select the appropriate base and expansion for the child depending
            // on whether it is automatically or relatively sized.
            let is_auto = self.sizing.as_ref().map(Option::is_none);
            let base = is_auto.select(regions.base, size);
            let expand = regions.expand | !is_auto;

            Regions::one(size, base, expand)
        };

        // Layout the child.
        let mut frames = self.child.layout_inline(world, &pod, styles)?;

        // Ensure frame size matches regions size if expansion is on.
        let frame = &mut frames[0];
        let target = regions.expand.select(regions.first, frame.size());
        frame.resize(target, Align::LEFT_TOP);

        Ok(frames)
    }
}

/// A block-level container that places content into a separate flow.
#[derive(Debug, Clone, Hash)]
pub struct BlockNode(pub Content);

#[node(LayoutBlock)]
impl BlockNode {
    /// The spacing between the previous and this block.
    #[property(skip)]
    pub const ABOVE: VNode = VNode::weak(Em::new(1.2).into());
    /// The spacing between this and the following block.
    #[property(skip)]
    pub const BELOW: VNode = VNode::weak(Em::new(1.2).into());

    fn construct(_: &mut Vm, args: &mut Args) -> SourceResult<Content> {
        Ok(Self(args.eat()?.unwrap_or_default()).pack())
    }

    fn set(...) {
        let spacing = args.named("spacing")?.map(VNode::weak);
        styles.set_opt(Self::ABOVE, args.named("above")?.map(VNode::strong).or(spacing));
        styles.set_opt(Self::BELOW, args.named("below")?.map(VNode::strong).or(spacing));
    }
}

impl LayoutBlock for BlockNode {
    fn layout_block(
        &self,
        world: Tracked<dyn World>,
        regions: &Regions,
        styles: StyleChain,
    ) -> SourceResult<Vec<Frame>> {
        self.0.layout_block(world, regions, styles)
    }
}
