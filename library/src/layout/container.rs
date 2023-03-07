use super::VNode;
use crate::layout::Spacing;
use crate::prelude::*;

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
/// Display: Box
/// Category: layout
#[node(Layout)]
pub struct BoxNode {
    /// The contents of the box.
    #[positional]
    #[default]
    pub body: Content,

    /// The width of the box.
    ///
    /// Boxes can have [fractional]($type/fraction) widths, as the example
    /// below demonstrates.
    ///
    /// _Note:_ Currently, only boxes and only their widths might be fractionally
    /// sized within paragraphs. Support for fractionally sized images, shapes,
    /// and more might be added in the future.
    ///
    /// ```example
    /// Line in #box(width: 1fr, line(length: 100%)) between.
    /// ```
    #[named]
    #[default]
    pub width: Sizing,

    /// The height of the box.
    #[named]
    #[default]
    pub height: Smart<Rel<Length>>,

    /// An amount to shift the box's baseline by.
    ///
    /// ```example
    /// Image: #box(baseline: 40%, image("tiger.jpg", width: 2cm)).
    /// ```
    #[settable]
    #[resolve]
    #[default]
    pub baseline: Rel<Length>,

    /// The box's background color. See the
    /// [rectangle's documentation]($func/rect.fill) for more details.
    #[settable]
    #[default]
    pub fill: Option<Paint>,

    /// The box's border color. See the
    /// [rectangle's documentation]($func/rect.stroke) for more details.
    #[settable]
    #[resolve]
    #[fold]
    #[default]
    pub stroke: Sides<Option<Option<PartialStroke>>>,

    /// How much to round the box's corners. See the [rectangle's
    /// documentation]($func/rect.radius) for more details.
    #[settable]
    #[resolve]
    #[fold]
    #[default]
    pub radius: Corners<Option<Rel<Length>>>,

    /// How much to pad the box's content. See the [rectangle's
    /// documentation]($func/rect.inset) for more details.
    #[settable]
    #[resolve]
    #[fold]
    #[default]
    pub inset: Sides<Option<Rel<Length>>>,

    /// How much to expand the box's size without affecting the layout.
    ///
    /// This is useful to prevent padding from affecting line layout. For a
    /// generalized version of the example below, see the documentation for the
    /// [raw text's block parameter]($func/raw.block).
    ///
    /// ```example
    /// An inline
    /// #box(
    ///   fill: luma(235),
    ///   inset: (x: 3pt, y: 0pt),
    ///   outset: (y: 3pt),
    ///   radius: 2pt,
    /// )[rectangle].
    #[settable]
    #[resolve]
    #[fold]
    #[default]
    pub outset: Sides<Option<Rel<Length>>>,
}

impl Layout for BoxNode {
    fn layout(
        &self,
        vt: &mut Vt,
        styles: StyleChain,
        regions: Regions,
    ) -> SourceResult<Fragment> {
        let width = match self.width() {
            Sizing::Auto => Smart::Auto,
            Sizing::Rel(rel) => Smart::Custom(rel),
            Sizing::Fr(_) => Smart::Custom(Ratio::one().into()),
        };

        // Resolve the sizing to a concrete size.
        let sizing = Axes::new(width, self.height());
        let expand = sizing.as_ref().map(Smart::is_custom);
        let size = sizing
            .resolve(styles)
            .zip(regions.base())
            .map(|(s, b)| s.map(|v| v.relative_to(b)))
            .unwrap_or(regions.base());

        // Apply inset.
        let mut child = self.body();
        let inset = styles.get(Self::INSET);
        if inset.iter().any(|v| !v.is_zero()) {
            child = child.padded(inset.map(|side| side.map(Length::from)));
        }

        // Select the appropriate base and expansion for the child depending
        // on whether it is automatically or relatively sized.
        let pod = Regions::one(size, expand);
        let mut frame = child.layout(vt, styles, pod)?.into_frame();

        // Apply baseline shift.
        let shift = styles.get(Self::BASELINE).relative_to(frame.height());
        if !shift.is_zero() {
            frame.set_baseline(frame.baseline() - shift);
        }

        // Prepare fill and stroke.
        let fill = styles.get(Self::FILL);
        let stroke = styles
            .get(Self::STROKE)
            .map(|s| s.map(PartialStroke::unwrap_or_default));

        // Add fill and/or stroke.
        if fill.is_some() || stroke.iter().any(Option::is_some) {
            let outset = styles.get(Self::OUTSET);
            let radius = styles.get(Self::RADIUS);
            frame.fill_and_stroke(fill, stroke, outset, radius);
        }

        // Apply metadata.
        frame.meta(styles);

        Ok(Fragment::frame(frame))
    }
}

/// A block-level container.
///
/// Such a container can be used to separate content, size it and give it a
/// background or border.
///
/// ## Examples
/// With a block, you can give a background to content while still allowing it
/// to break across multiple pages.
/// ```example
/// #set page(height: 100pt)
/// #block(
///   fill: luma(230),
///   inset: 8pt,
///   radius: 4pt,
///   lorem(30),
/// )
/// ```
///
/// Blocks are also useful to force elements that would otherwise be inline to
/// become block-level, especially when writing show rules.
/// ```example
/// #show heading: it => it.title
/// = Blockless
/// More text.
///
/// #show heading: it => block(it.title)
/// = Blocky
/// More text.
/// ```
///
/// ## Parameters
/// - spacing: `Spacing` (named, settable)
///   The spacing around this block. This is shorthand to set `above` and
///   `below` to the same value.
///
///   ```example
///   #set align(center)
///   #show math.formula: set block(above: 8pt, below: 16pt)
///
///   This sum of $x$ and $y$:
///   $ x + y = z $
///   A second paragraph.
///   ```
///
/// Display: Block
/// Category: layout
#[node(Layout)]
#[set({
    let spacing = args.named("spacing")?;
    styles.set_opt(
        Self::ABOVE,
        args.named("above")?
            .map(VNode::block_around)
            .or_else(|| spacing.map(VNode::block_spacing)),
    );
    styles.set_opt(
        Self::BELOW,
        args.named("below")?
            .map(VNode::block_around)
            .or_else(|| spacing.map(VNode::block_spacing)),
    );
})]
pub struct BlockNode {
    /// The contents of the block.
    #[positional]
    #[default]
    pub body: Content,

    /// The block's width.
    ///
    /// ```example
    /// #set align(center)
    /// #block(
    ///   width: 60%,
    ///   inset: 8pt,
    ///   fill: silver,
    ///   lorem(10),
    /// )
    /// ```
    #[named]
    #[default]
    pub width: Smart<Rel<Length>>,

    /// The block's height. When the height is larger than the remaining space on
    /// a page and [`breakable`]($func/block.breakable) is `{true}`, the block
    /// will continue on the next page with the remaining height.
    ///
    /// ```example
    /// #set page(height: 80pt)
    /// #set align(center)
    /// #block(
    ///   width: 80%,
    ///   height: 150%,
    ///   fill: aqua,
    /// )
    /// ```
    #[named]
    #[default]
    pub height: Smart<Rel<Length>>,

    /// Whether the block can be broken and continue on the next page.
    ///
    /// Defaults to `{true}`.
    /// ```example
    /// #set page(height: 80pt)
    /// The following block will
    /// jump to its own page.
    /// #block(
    ///   breakable: false,
    ///   lorem(15),
    /// )
    /// ```
    #[settable]
    #[default(true)]
    pub breakable: bool,

    /// The block's background color. See the
    /// [rectangle's documentation]($func/rect.fill) for more details.
    #[settable]
    #[default]
    pub fill: Option<Paint>,

    /// The block's border color. See the
    /// [rectangle's documentation]($func/rect.stroke) for more details.
    #[settable]
    #[resolve]
    #[fold]
    #[default]
    pub stroke: Sides<Option<Option<PartialStroke>>>,

    /// How much to round the block's corners. See the [rectangle's
    /// documentation]($func/rect.radius) for more details.
    #[settable]
    #[resolve]
    #[fold]
    #[default]
    pub radius: Corners<Option<Rel<Length>>>,

    /// How much to pad the block's content. See the [rectangle's
    /// documentation]($func/rect.inset) for more details.
    #[settable]
    #[resolve]
    #[fold]
    #[default]
    pub inset: Sides<Option<Rel<Length>>>,

    /// How much to expand the block's size without affecting the layout. See
    /// the [rectangle's documentation]($func/rect.outset) for more details.
    #[settable]
    #[resolve]
    #[fold]
    #[default]
    pub outset: Sides<Option<Rel<Length>>>,

    /// The spacing between this block and its predecessor. Takes precedence over
    /// `spacing`. Can be used in combination with a show rule to adjust the
    /// spacing around arbitrary block-level elements.
    ///
    /// The default value is `{1.2em}`.
    #[settable]
    #[skip]
    #[default(VNode::block_spacing(Em::new(1.2).into()))]
    pub above: VNode,

    /// The spacing between this block and its successor. Takes precedence
    /// over `spacing`.
    ///
    /// The default value is `{1.2em}`.
    #[settable]
    #[skip]
    #[default(VNode::block_spacing(Em::new(1.2).into()))]
    pub below: VNode,

    /// Whether this block must stick to the following one.
    ///
    /// Use this to prevent page breaks between e.g. a heading and its body.
    #[settable]
    #[skip]
    #[default(false)]
    pub sticky: bool,
}

impl Layout for BlockNode {
    fn layout(
        &self,
        vt: &mut Vt,
        styles: StyleChain,
        regions: Regions,
    ) -> SourceResult<Fragment> {
        // Apply inset.
        let mut child = self.body();
        let inset = styles.get(Self::INSET);
        if inset.iter().any(|v| !v.is_zero()) {
            child = child.clone().padded(inset.map(|side| side.map(Length::from)));
        }

        // Resolve the sizing to a concrete size.
        let sizing = Axes::new(self.width(), self.height());
        let mut expand = sizing.as_ref().map(Smart::is_custom);
        let mut size = sizing
            .resolve(styles)
            .zip(regions.base())
            .map(|(s, b)| s.map(|v| v.relative_to(b)))
            .unwrap_or(regions.base());

        // Layout the child.
        let mut frames = if styles.get(Self::BREAKABLE) {
            // Measure to ensure frames for all regions have the same width.
            if sizing.x == Smart::Auto {
                let pod = Regions::one(size, Axes::splat(false));
                let frame = child.layout(vt, styles, pod)?.into_frame();
                size.x = frame.width();
                expand.x = true;
            }

            let mut pod = regions;
            pod.size.x = size.x;
            pod.expand = expand;

            // Generate backlog for fixed height.
            let mut heights = vec![];
            if sizing.y.is_custom() {
                let mut remaining = size.y;
                for region in regions.iter() {
                    let limited = region.y.min(remaining);
                    heights.push(limited);
                    remaining -= limited;
                    if Abs::zero().fits(remaining) {
                        break;
                    }
                }

                pod.size.y = heights[0];
                pod.backlog = &heights[1..];
                pod.last = None;
            }

            child.layout(vt, styles, pod)?.into_frames()
        } else {
            let pod = Regions::one(size, expand);
            child.layout(vt, styles, pod)?.into_frames()
        };

        // Prepare fill and stroke.
        let fill = styles.get(Self::FILL);
        let stroke = styles
            .get(Self::STROKE)
            .map(|s| s.map(PartialStroke::unwrap_or_default));

        // Add fill and/or stroke.
        if fill.is_some() || stroke.iter().any(Option::is_some) {
            let mut skip = false;
            if let [first, rest @ ..] = frames.as_slice() {
                skip = first.is_empty() && rest.iter().any(|frame| !frame.is_empty());
            }

            let outset = styles.get(Self::OUTSET);
            let radius = styles.get(Self::RADIUS);
            for frame in frames.iter_mut().skip(skip as usize) {
                frame.fill_and_stroke(fill, stroke, outset, radius);
            }
        }

        // Apply metadata.
        for frame in &mut frames {
            frame.meta(styles);
        }

        Ok(Fragment::frames(frames))
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
}

impl Default for Sizing {
    fn default() -> Self {
        Self::Auto
    }
}

impl<T: Into<Spacing>> From<T> for Sizing {
    fn from(spacing: T) -> Self {
        match spacing.into() {
            Spacing::Rel(rel) => Self::Rel(rel),
            Spacing::Fr(fr) => Self::Fr(fr),
        }
    }
}

cast_from_value! {
    Sizing,
    _: Smart<Never> => Self::Auto,
    v: Rel<Length> => Self::Rel(v),
    v: Fr => Self::Fr(v),
}

cast_to_value! {
    v: Sizing => match v {
        Sizing::Auto => Value::Auto,
        Sizing::Rel(rel) => Value::Relative(rel),
        Sizing::Fr(fr) => Value::Fraction(fr),
    }
}
