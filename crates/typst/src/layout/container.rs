use crate::diag::SourceResult;
use crate::engine::Engine;
use crate::foundations::{
    cast, elem, AutoValue, Content, Packed, Resolve, Smart, StyleChain, Value,
};
use crate::layout::{
    Abs, Axes, Corners, Em, Fr, Fragment, Frame, FrameKind, LayoutMultiple, Length,
    Ratio, Regions, Rel, Sides, Size, Spacing, VElem,
};
use crate::util::Numeric;
use crate::visualize::{clip_rect, Paint, Stroke};

/// An inline-level container that sizes content.
///
/// All elements except inline math, text, and boxes are block-level and cannot
/// occur inside of a paragraph. The box function can be used to integrate such
/// elements into a paragraph. Boxes take the size of their contents by default
/// but can also be sized explicitly.
///
/// # Example
/// ```example
/// Refer to the docs
/// #box(
///   height: 9pt,
///   image("docs.svg")
/// )
/// for more information.
/// ```
#[elem]
pub struct BoxElem {
    /// The width of the box.
    ///
    /// Boxes can have [fractional]($fraction) widths, as the example below
    /// demonstrates.
    ///
    /// _Note:_ Currently, only boxes and only their widths might be fractionally
    /// sized within paragraphs. Support for fractionally sized images, shapes,
    /// and more might be added in the future.
    ///
    /// ```example
    /// Line in #box(width: 1fr, line(length: 100%)) between.
    /// ```
    pub width: Sizing,

    /// The height of the box.
    pub height: Smart<Rel<Length>>,

    /// An amount to shift the box's baseline by.
    ///
    /// ```example
    /// Image: #box(baseline: 40%, image("tiger.jpg", width: 2cm)).
    /// ```
    #[resolve]
    pub baseline: Rel<Length>,

    /// The box's background color. See the
    /// [rectangle's documentation]($rect.fill) for more details.
    pub fill: Option<Paint>,

    /// The box's border color. See the
    /// [rectangle's documentation]($rect.stroke) for more details.
    #[resolve]
    #[fold]
    pub stroke: Sides<Option<Option<Stroke>>>,

    /// How much to round the box's corners. See the
    /// [rectangle's documentation]($rect.radius) for more details.
    #[resolve]
    #[fold]
    pub radius: Corners<Option<Rel<Length>>>,

    /// How much to pad the box's content.
    ///
    /// _Note:_ When the box contains text, its exact size depends on the
    /// current [text edges]($text.top-edge).
    ///
    /// ```example
    /// #rect(inset: 0pt)[Tight]
    /// ```
    #[resolve]
    #[fold]
    pub inset: Sides<Option<Rel<Length>>>,

    /// How much to expand the box's size without affecting the layout.
    ///
    /// This is useful to prevent padding from affecting line layout. For a
    /// generalized version of the example below, see the documentation for the
    /// [raw text's block parameter]($raw.block).
    ///
    /// ```example
    /// An inline
    /// #box(
    ///   fill: luma(235),
    ///   inset: (x: 3pt, y: 0pt),
    ///   outset: (y: 3pt),
    ///   radius: 2pt,
    /// )[rectangle].
    /// ```
    #[resolve]
    #[fold]
    pub outset: Sides<Option<Rel<Length>>>,

    /// Whether to clip the content inside the box.
    #[default(false)]
    pub clip: bool,

    /// The contents of the box.
    #[positional]
    pub body: Option<Content>,
}

impl Packed<BoxElem> {
    #[typst_macros::time(name = "box", span = self.span())]
    pub fn layout(
        &self,
        engine: &mut Engine,
        styles: StyleChain,
        regions: Regions,
    ) -> SourceResult<Frame> {
        let width = match self.width(styles) {
            Sizing::Auto => Smart::Auto,
            Sizing::Rel(rel) => Smart::Custom(rel),
            Sizing::Fr(_) => Smart::Custom(Ratio::one().into()),
        };

        // Resolve the sizing to a concrete size.
        let sizing = Axes::new(width, self.height(styles));
        let expand = sizing.as_ref().map(Smart::is_custom);
        let size = sizing
            .resolve(styles)
            .zip_map(regions.base(), |s, b| s.map(|v| v.relative_to(b)))
            .unwrap_or(regions.base());

        // Apply inset.
        let mut body = self.body(styles).unwrap_or_default();
        let inset = self.inset(styles).unwrap_or_default();
        if inset.iter().any(|v| !v.is_zero()) {
            body = body.padded(inset.map(|side| side.map(Length::from)));
        }

        // Select the appropriate base and expansion for the child depending
        // on whether it is automatically or relatively sized.
        let pod = Regions::one(size, expand);
        let mut frame = body.layout(engine, styles, pod)?.into_frame();

        // Enforce correct size.
        *frame.size_mut() = expand.select(size, frame.size());

        // Apply baseline shift.
        let shift = self.baseline(styles).relative_to(frame.height());
        if !shift.is_zero() {
            frame.set_baseline(frame.baseline() - shift);
        }

        // Prepare fill and stroke.
        let fill = self.fill(styles);
        let stroke = self
            .stroke(styles)
            .unwrap_or_default()
            .map(|s| s.map(Stroke::unwrap_or_default));

        // Clip the contents
        if self.clip(styles) {
            let outset =
                self.outset(styles).unwrap_or_default().relative_to(frame.size());
            let size = frame.size() + outset.sum_by_axis();
            let radius = self.radius(styles).unwrap_or_default();
            frame.clip(clip_rect(size, radius, &stroke));
        }

        // Add fill and/or stroke.
        if fill.is_some() || stroke.iter().any(Option::is_some) {
            let outset = self.outset(styles).unwrap_or_default();
            let radius = self.radius(styles).unwrap_or_default();
            frame.fill_and_stroke(fill, stroke, outset, radius, self.span());
        }

        // Apply metadata.
        frame.set_kind(FrameKind::Hard);

        Ok(frame)
    }
}

/// A block-level container.
///
/// Such a container can be used to separate content, size it, and give it a
/// background or border.
///
/// # Examples
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
/// #show heading: it => it.body
/// = Blockless
/// More text.
///
/// #show heading: it => block(it.body)
/// = Blocky
/// More text.
/// ```
#[elem(LayoutMultiple)]
pub struct BlockElem {
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
    pub width: Smart<Rel<Length>>,

    /// The block's height. When the height is larger than the remaining space
    /// on a page and [`breakable`]($block.breakable) is `{true}`, the
    /// block will continue on the next page with the remaining height.
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
    pub height: Smart<Rel<Length>>,

    /// Whether the block can be broken and continue on the next page.
    ///
    /// ```example
    /// #set page(height: 80pt)
    /// The following block will
    /// jump to its own page.
    /// #block(
    ///   breakable: false,
    ///   lorem(15),
    /// )
    /// ```
    #[default(true)]
    pub breakable: bool,

    /// The block's background color. See the
    /// [rectangle's documentation]($rect.fill) for more details.
    pub fill: Option<Paint>,

    /// The block's border color. See the
    /// [rectangle's documentation]($rect.stroke) for more details.
    #[resolve]
    #[fold]
    pub stroke: Sides<Option<Option<Stroke>>>,

    /// How much to round the block's corners. See the
    /// [rectangle's documentation]($rect.radius) for more details.
    #[resolve]
    #[fold]
    pub radius: Corners<Option<Rel<Length>>>,

    /// How much to pad the block's content. See the
    /// [box's documentation]($box.inset) for more details.
    #[resolve]
    #[fold]
    pub inset: Sides<Option<Rel<Length>>>,

    /// How much to expand the block's size without affecting the layout. See
    /// the [box's documentation]($box.outset) for more details.
    #[resolve]
    #[fold]
    pub outset: Sides<Option<Rel<Length>>>,

    /// The spacing around this block. This is shorthand to set `above` and
    /// `below` to the same value.
    ///
    /// ```example
    /// #set align(center)
    /// #show math.equation: set block(above: 8pt, below: 16pt)
    ///
    /// This sum of $x$ and $y$:
    /// $ x + y = z $
    /// A second paragraph.
    /// ```
    #[external]
    #[default(Em::new(1.2).into())]
    pub spacing: Spacing,

    /// The spacing between this block and its predecessor. Takes precedence
    /// over `spacing`. Can be used in combination with a show rule to adjust
    /// the spacing around arbitrary block-level elements.
    #[external]
    #[default(Em::new(1.2).into())]
    pub above: Spacing,
    #[internal]
    #[parse(
        let spacing = args.named("spacing")?;
        args.named("above")?
            .map(VElem::block_around)
            .or_else(|| spacing.map(VElem::block_spacing))
    )]
    #[default(VElem::block_spacing(Em::new(1.2).into()))]
    pub above: VElem,

    /// The spacing between this block and its successor. Takes precedence
    /// over `spacing`.
    #[external]
    #[default(Em::new(1.2).into())]
    pub below: Spacing,
    #[internal]
    #[parse(
        args.named("below")?
            .map(VElem::block_around)
            .or_else(|| spacing.map(VElem::block_spacing))
    )]
    #[default(VElem::block_spacing(Em::new(1.2).into()))]
    pub below: VElem,

    /// Whether to clip the content inside the block.
    #[default(false)]
    pub clip: bool,

    /// The contents of the block.
    #[positional]
    pub body: Option<Content>,

    /// Whether this block must stick to the following one.
    ///
    /// Use this to prevent page breaks between e.g. a heading and its body.
    #[internal]
    #[default(false)]
    #[ghost]
    pub sticky: bool,
}

impl LayoutMultiple for Packed<BlockElem> {
    #[typst_macros::time(name = "block", span = self.span())]
    fn layout(
        &self,
        engine: &mut Engine,
        styles: StyleChain,
        regions: Regions,
    ) -> SourceResult<Fragment> {
        // Apply inset.
        let mut body = self.body(styles).unwrap_or_default();
        let inset = self.inset(styles).unwrap_or_default();
        if inset.iter().any(|v| !v.is_zero()) {
            body = body.clone().padded(inset.map(|side| side.map(Length::from)));
        }

        // Resolve the sizing to a concrete size.
        let sizing = Axes::new(self.width(styles), self.height(styles));
        let mut expand = sizing.as_ref().map(Smart::is_custom);
        let mut size = sizing
            .resolve(styles)
            .zip_map(regions.base(), |s, b| s.map(|v| v.relative_to(b)))
            .unwrap_or(regions.base());

        // Layout the child.
        let mut frames = if self.breakable(styles) {
            // Measure to ensure frames for all regions have the same width.
            if sizing.x == Smart::Auto {
                let pod = Regions::one(size, Axes::splat(false));
                let frame = body.measure(engine, styles, pod)?.into_frame();
                size.x = frame.width();
                expand.x = true;
            }

            let mut pod = regions;
            pod.size.x = size.x;
            pod.expand = expand;

            if expand.y {
                pod.full = size.y;
            }

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

                if let Some(last) = heights.last_mut() {
                    *last += remaining;
                }

                pod.size.y = heights[0];
                pod.backlog = &heights[1..];
                pod.last = None;
            }

            let mut frames = body.layout(engine, styles, pod)?.into_frames();
            for (frame, &height) in frames.iter_mut().zip(&heights) {
                *frame.size_mut() =
                    expand.select(Size::new(size.x, height), frame.size());
            }
            frames
        } else {
            let pod = Regions::one(size, expand);
            let mut frames = body.layout(engine, styles, pod)?.into_frames();
            *frames[0].size_mut() = expand.select(size, frames[0].size());
            frames
        };

        // Prepare fill and stroke.
        let fill = self.fill(styles);
        let stroke = self
            .stroke(styles)
            .unwrap_or_default()
            .map(|s| s.map(Stroke::unwrap_or_default));

        // Clip the contents
        if self.clip(styles) {
            for frame in frames.iter_mut() {
                let outset =
                    self.outset(styles).unwrap_or_default().relative_to(frame.size());
                let size = frame.size() + outset.sum_by_axis();
                let radius = self.radius(styles).unwrap_or_default();
                frame.clip(clip_rect(size, radius, &stroke));
            }
        }

        // Add fill and/or stroke.
        if fill.is_some() || stroke.iter().any(Option::is_some) {
            let mut skip = false;
            if let [first, rest @ ..] = frames.as_slice() {
                skip = first.is_empty() && rest.iter().any(|frame| !frame.is_empty());
            }

            let outset = self.outset(styles).unwrap_or_default();
            let radius = self.radius(styles).unwrap_or_default();
            for frame in frames.iter_mut().skip(skip as usize) {
                frame.fill_and_stroke(
                    fill.clone(),
                    stroke.clone(),
                    outset,
                    radius,
                    self.span(),
                );
            }
        }

        // Apply metadata.
        for frame in &mut frames {
            frame.set_kind(FrameKind::Hard);
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

cast! {
    Sizing,
    self => match self {
        Self::Auto => Value::Auto,
        Self::Rel(rel) => rel.into_value(),
        Self::Fr(fr) => fr.into_value(),
    },
    _: AutoValue => Self::Auto,
    v: Rel<Length> => Self::Rel(v),
    v: Fr => Self::Fr(v),
}
