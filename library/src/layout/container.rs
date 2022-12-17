use super::VNode;
use crate::layout::Spacing;
use crate::prelude::*;

/// An inline-level container that sizes content.
///
/// # Parameters
/// - body: Content (positional)
///   The contents of the box.
/// - width: Rel<Length> (named)
///   The width of the box.
/// - height: Rel<Length> (named)
///   The height of the box.
///
/// # Tags
/// - layout
#[func]
#[capable(Layout, Inline)]
#[derive(Debug, Hash)]
pub struct BoxNode {
    /// How to size the content horizontally and vertically.
    pub sizing: Axes<Option<Rel<Length>>>,
    /// The content to be sized.
    pub body: Content,
}

#[node]
impl BoxNode {
    fn construct(_: &Vm, args: &mut Args) -> SourceResult<Content> {
        let width = args.named("width")?;
        let height = args.named("height")?;
        let body = args.eat::<Content>()?.unwrap_or_default();
        Ok(Self { sizing: Axes::new(width, height), body }.pack())
    }
}

impl Layout for BoxNode {
    fn layout(
        &self,
        vt: &mut Vt,
        styles: StyleChain,
        regions: Regions,
    ) -> SourceResult<Fragment> {
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
        let mut frame = self.body.layout(vt, styles, pod)?.into_frame();

        // Ensure frame size matches regions size if expansion is on.
        let target = regions.expand.select(regions.first, frame.size());
        frame.resize(target, Align::LEFT_TOP);

        Ok(Fragment::frame(frame))
    }
}

impl Inline for BoxNode {}

/// A block-level container that places content into a separate flow.
///
/// # Parameters
/// - body: Content (positional)
///   The contents of the block.
/// - spacing: Spacing (named, settable)
///   The spacing around this block.
/// - above: Spacing (named, settable)
///   The spacing between the previous and this block. Takes precedence over
///   `spacing`.
/// - below: Spacing (named, settable)
///   The spacing between this block and the following one. Takes precedence
///   over `spacing`.
///
/// # Tags
/// - layout
#[func]
#[capable(Layout)]
#[derive(Debug, Hash)]
pub struct BlockNode(pub Content);

#[node]
impl BlockNode {
    /// The spacing between the previous and this block.
    #[property(skip)]
    pub const ABOVE: VNode = VNode::block_spacing(Em::new(1.2).into());
    /// The spacing between this and the following block.
    #[property(skip)]
    pub const BELOW: VNode = VNode::block_spacing(Em::new(1.2).into());
    /// Whether this block must stick to the following one.
    #[property(skip)]
    pub const STICKY: bool = false;

    fn construct(_: &Vm, args: &mut Args) -> SourceResult<Content> {
        Ok(Self(args.eat()?.unwrap_or_default()).pack())
    }

    fn set(...) {
        let spacing = args.named("spacing")?.map(VNode::block_spacing);
        styles.set_opt(
            Self::ABOVE,
            args.named("above")?.map(VNode::block_around).or(spacing),
        );
        styles.set_opt(
            Self::BELOW,
            args.named("below")?.map(VNode::block_around).or(spacing),
        );
    }
}

impl Layout for BlockNode {
    fn layout(
        &self,
        vt: &mut Vt,
        styles: StyleChain,
        regions: Regions,
    ) -> SourceResult<Fragment> {
        self.0.layout(vt, styles, regions)
    }
}
