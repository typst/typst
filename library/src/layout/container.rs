use super::VNode;
use crate::layout::Spacing;
use crate::prelude::*;

/// # Box
/// An inline-level container that sizes content.
///
/// All elements except inline math, text, and boxes are block-level and cannot
/// occur inside of a paragraph. The box element is an inline-level container.
/// Boxes take the size of their contents by default but can also be sized
/// explicitly.
///
/// _Note:_ While the behavior above will be the default in the future, the
/// transformation functions [`scale`](@scale), [`rotate`](@rotate), and
/// [`move`](@move) will currently yield inline nodes within paragraphs.
///
/// ## Example
/// ```
/// Refer to the docs
/// #box(
///   height: 9pt,
///   image("docs.svg")
/// )
/// for more information.
/// ```
///
/// ## Parameters
/// - body: Content (positional)
///   The contents of the box.
///
/// - width: Rel<Length> (named)
///   The width of the box.
///
/// - height: Rel<Length> (named)
///   The height of the box.
///
/// ## Category
/// layout
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

    fn field(&self, name: &str) -> Option<Value> {
        match name {
            "body" => Some(Value::Content(self.body.clone())),
            _ => None,
        }
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

/// # Block
/// A block-level container that places content into a separate flow.
///
/// This can be used to force elements that would otherwise be inline to become
/// block-level. This is especially useful when writing show rules.
///
/// ## Example
/// ```
/// [
///   #show heading: it => it.title
///   = No block
///   Some text
/// ]
///
/// [
///   #show heading: it => block(it.title)
///   = Block
///   Some more text
/// ]
/// ```
///
/// ## Parameters
/// - body: Content (positional)
///   The contents of the block.
///
/// - spacing: Spacing (named, settable)
///   The spacing around this block.
///
/// - above: Spacing (named, settable)
///   The spacing between this block and its predecessor. Takes precedence over
///   `spacing`.
///
/// - below: Spacing (named, settable)
///   The spacing between this block and its successor. Takes precedence
///   over `spacing`.
///
/// ## Category
/// layout
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
    ///
    /// Use this to prevent page breaks between e.g. a heading and its body.
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

    fn field(&self, name: &str) -> Option<Value> {
        match name {
            "body" => Some(Value::Content(self.0.clone())),
            _ => None,
        }
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
