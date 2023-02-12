use super::VNode;
use crate::layout::Spacing;
use crate::prelude::*;

/// # Box
/// An inline-level container that sizes content.
///
/// All elements except inline math, text, and boxes are block-level and cannot
/// occur inside of a paragraph. The box function can be used to integrate such
/// elements into a paragraph. Boxes take the size of their contents by default
/// but can also be sized explicitly.
///
/// ## Example
/// ```example
/// Refer to the docs
/// #box(
///   height: 9pt,
///   image("docs.svg")
/// )
/// for more information.
/// ```
///
/// ## Parameters
/// - body: `Content` (positional)
///   The contents of the box.
///
/// - width: `Sizing` (named)
///   The width of the box.
///
///   Boxes can have [fractional]($type/fraction) widths, as the example
///   below demonstrates.
///
///   _Note:_ Currently, only boxes and only their widths might be fractionally
///   sized within paragraphs. Support for fractionally sized images, shapes,
///   and more might be added in the future.
///
///   ```example
///   Line in #box(width: 1fr, line(length: 100%)) between.
///   ```
///
/// - height: `Rel<Length>` (named)
///   The height of the box.
///
/// - baseline: `Rel<Length>` (named)
///   An amount to shift the box's baseline by.
///
///   ```example
///   Image: #box(baseline: 40%, image("tiger.jpg", width: 2cm)).
///   ```
///
/// ## Category
/// layout
#[func]
#[capable(Layout)]
#[derive(Debug, Hash)]
pub struct BoxNode {
    /// The content to be sized.
    pub body: Content,
    /// The box's width.
    pub width: Sizing,
    /// The box's height.
    pub height: Smart<Rel<Length>>,
    /// The box's baseline shift.
    pub baseline: Rel<Length>,
}

#[node]
impl BoxNode {
    fn construct(_: &Vm, args: &mut Args) -> SourceResult<Content> {
        let body = args.eat::<Content>()?.unwrap_or_default();
        let width = args.named("width")?.unwrap_or_default();
        let height = args.named("height")?.unwrap_or_default();
        let baseline = args.named("baseline")?.unwrap_or_default();
        Ok(Self { body, width, height, baseline }.pack())
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
        let width = match self.width {
            Sizing::Auto => Smart::Auto,
            Sizing::Rel(rel) => Smart::Custom(rel),
            Sizing::Fr(_) => Smart::Custom(Ratio::one().into()),
        };

        // Resolve the sizing to a concrete size.
        let sizing = Axes::new(width, self.height);
        let size = sizing
            .resolve(styles)
            .zip(regions.base())
            .map(|(s, b)| s.map(|v| v.relative_to(b)))
            .unwrap_or(regions.size);

        // Select the appropriate base and expansion for the child depending
        // on whether it is automatically or relatively sized.
        let is_auto = sizing.as_ref().map(Smart::is_auto);
        let expand = regions.expand | !is_auto;
        let pod = Regions::one(size, expand);
        let mut frame = self.body.layout(vt, styles, pod)?.into_frame();

        // Apply baseline shift.
        let shift = self.baseline.resolve(styles).relative_to(frame.height());
        if !shift.is_zero() {
            frame.set_baseline(frame.baseline() - shift);
        }

        Ok(Fragment::frame(frame))
    }
}

/// # Block
/// A block-level container that places content into a separate flow.
///
/// This can be used to force elements that would otherwise be inline to become
/// block-level. This is especially useful when writing show rules.
///
/// ## Example
/// ```example
/// #[
///   #show heading: it => it.title
///   = No block
///   Some text
/// ]
///
/// #[
///   #show heading: it => block(it.title)
///   = Block
///   Some more text
/// ]
/// ```
///
/// ## Parameters
/// - body: `Content` (positional)
///   The contents of the block.
///
/// - spacing: `Spacing` (named, settable)
///   The spacing around this block.
///
/// - above: `Spacing` (named, settable)
///   The spacing between this block and its predecessor. Takes precedence over
///   `spacing`.
///
///   The default value is `{1.2em}`.
///
/// - below: `Spacing` (named, settable)
///   The spacing between this block and its successor. Takes precedence
///   over `spacing`.
///
///   The default value is `{1.2em}`.
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

/// Defines how to size a grid cell along an axis.
#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
pub enum Sizing {
    /// A track that fits its cell's contents.
    Auto,
    /// A track size specified in absolute terms and relative to the parent's
    /// size.
    Rel(Rel<Length>),
    /// A track size specified as a fraction of the remaining free space in the
    /// parent.
    Fr(Fr),
}

impl Sizing {
    /// Whether this is fractional sizing.
    pub fn is_fractional(self) -> bool {
        matches!(self, Self::Fr(_))
    }

    pub fn encode(self) -> Value {
        match self {
            Self::Auto => Value::Auto,
            Self::Rel(rel) => Spacing::Rel(rel).encode(),
            Self::Fr(fr) => Spacing::Fr(fr).encode(),
        }
    }

    pub fn encode_slice(vec: &[Sizing]) -> Value {
        Value::Array(vec.iter().copied().map(Self::encode).collect())
    }
}

impl Default for Sizing {
    fn default() -> Self {
        Self::Auto
    }
}

impl From<Spacing> for Sizing {
    fn from(spacing: Spacing) -> Self {
        match spacing {
            Spacing::Rel(rel) => Self::Rel(rel),
            Spacing::Fr(fr) => Self::Fr(fr),
        }
    }
}
