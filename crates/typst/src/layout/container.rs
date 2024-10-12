use once_cell::unsync::Lazy;
use smallvec::SmallVec;

use crate::diag::{bail, SourceResult};
use crate::engine::Engine;
use crate::foundations::{
    cast, elem, Args, AutoValue, Construct, Content, NativeElement, Packed, Resolve,
    Smart, StyleChain, Value,
};
use crate::introspection::Locator;
use crate::layout::{
    layout_fragment, layout_frame, Abs, Axes, Corners, Em, Fr, Fragment, Frame,
    FrameKind, Length, Region, Regions, Rel, Sides, Size, Spacing,
};
use crate::utils::Numeric;
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
    ///
    /// Clipping is useful when the box's content is larger than the box itself,
    /// as any content that exceeds the box's bounds will be hidden.
    ///
    /// ```example
    /// #box(
    ///   width: 50pt,
    ///   height: 50pt,
    ///   clip: true,
    ///   image("tiger.jpg", width: 100pt, height: 100pt)
    /// )
    /// ```
    #[default(false)]
    pub clip: bool,

    /// The contents of the box.
    #[positional]
    #[borrowed]
    pub body: Option<Content>,
}

impl Packed<BoxElem> {
    /// Layout this box as part of a paragraph.
    #[typst_macros::time(name = "box", span = self.span())]
    pub fn layout(
        &self,
        engine: &mut Engine,
        locator: Locator,
        styles: StyleChain,
        region: Size,
    ) -> SourceResult<Frame> {
        // Fetch sizing properties.
        let width = self.width(styles);
        let height = self.height(styles);
        let inset = self.inset(styles).unwrap_or_default();

        // Build the pod region.
        let pod = unbreakable_pod(&width, &height.into(), &inset, styles, region);

        // Layout the body.
        let mut frame = match self.body(styles) {
            // If we have no body, just create an empty frame. If necessary,
            // its size will be adjusted below.
            None => Frame::hard(Size::zero()),

            // If we have a child, layout it into the body. Boxes are boundaries
            // for gradient relativeness, so we set the `FrameKind` to `Hard`.
            Some(body) => layout_frame(engine, body, locator, styles, pod)?
                .with_kind(FrameKind::Hard),
        };

        // Enforce a correct frame size on the expanded axes. Do this before
        // applying the inset, since the pod shrunk.
        frame.set_size(pod.expand.select(pod.size, frame.size()));

        // Apply the inset.
        if !inset.is_zero() {
            crate::layout::grow(&mut frame, &inset);
        }

        // Prepare fill and stroke.
        let fill = self.fill(styles);
        let stroke = self
            .stroke(styles)
            .unwrap_or_default()
            .map(|s| s.map(Stroke::unwrap_or_default));

        // Only fetch these if necessary (for clipping or filling/stroking).
        let outset = Lazy::new(|| self.outset(styles).unwrap_or_default());
        let radius = Lazy::new(|| self.radius(styles).unwrap_or_default());

        // Clip the contents, if requested.
        if self.clip(styles) {
            let size = frame.size() + outset.relative_to(frame.size()).sum_by_axis();
            frame.clip(clip_rect(size, &radius, &stroke));
        }

        // Add fill and/or stroke.
        if fill.is_some() || stroke.iter().any(Option::is_some) {
            frame.fill_and_stroke(fill, &stroke, &outset, &radius, self.span());
        }

        // Assign label to the frame.
        if let Some(label) = self.label() {
            frame.label(label);
        }

        // Apply baseline shift. Do this after setting the size and applying the
        // inset, so that a relative shift is resolved relative to the final
        // height.
        let shift = self.baseline(styles).relative_to(frame.height());
        if !shift.is_zero() {
            frame.set_baseline(frame.baseline() - shift);
        }

        Ok(frame)
    }
}

/// An inline-level container that can produce arbitrary items that can break
/// across lines.
#[elem(Construct)]
pub struct InlineElem {
    /// A callback that is invoked with the regions to produce arbitrary
    /// inline items.
    #[required]
    #[internal]
    body: callbacks::InlineCallback,
}

impl Construct for InlineElem {
    fn construct(_: &mut Engine, args: &mut Args) -> SourceResult<Content> {
        bail!(args.span, "cannot be constructed manually");
    }
}

impl InlineElem {
    /// Create an inline-level item with a custom layouter.
    #[allow(clippy::type_complexity)]
    pub fn layouter<T: NativeElement>(
        captured: Packed<T>,
        callback: fn(
            content: &Packed<T>,
            engine: &mut Engine,
            locator: Locator,
            styles: StyleChain,
            region: Size,
        ) -> SourceResult<Vec<InlineItem>>,
    ) -> Self {
        Self::new(callbacks::InlineCallback::new(captured, callback))
    }
}

impl Packed<InlineElem> {
    /// Layout the element.
    pub fn layout(
        &self,
        engine: &mut Engine,
        locator: Locator,
        styles: StyleChain,
        region: Size,
    ) -> SourceResult<Vec<InlineItem>> {
        self.body().call(engine, locator, styles, region)
    }
}

/// Layouted items suitable for placing in a paragraph.
#[derive(Debug, Clone)]
pub enum InlineItem {
    /// Absolute spacing between other items, and whether it is weak.
    Space(Abs, bool),
    /// Layouted inline-level content.
    Frame(Frame),
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
#[elem]
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
    pub height: Sizing,

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

    /// The spacing around the block. When `{auto}`, inherits the paragraph
    /// [`spacing`]($par.spacing).
    ///
    /// For two adjacent blocks, the larger of the first block's `above` and the
    /// second block's `below` spacing wins. Moreover, block spacing takes
    /// precedence over paragraph [`spacing`]($par.spacing).
    ///
    /// Note that this is only a shorthand to set `above` and `below` to the
    /// same value. Since the values for `above` and `below` might differ, a
    /// [context] block only provides access to `{block.above}` and
    /// `{block.below}`, not to `{block.spacing}` directly.
    ///
    /// This property can be used in combination with a show rule to adjust the
    /// spacing around arbitrary block-level elements.
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

    /// The spacing between this block and its predecessor.
    #[parse(
        let spacing = args.named("spacing")?;
        args.named("above")?.or(spacing)
    )]
    pub above: Smart<Spacing>,

    /// The spacing between this block and its successor.
    #[parse(args.named("below")?.or(spacing))]
    pub below: Smart<Spacing>,

    /// Whether to clip the content inside the block.
    ///
    /// Clipping is useful when the block's content is larger than the block itself,
    /// as any content that exceeds the block's bounds will be hidden.
    ///
    /// ```example
    /// #block(
    ///   width: 50pt,
    ///   height: 50pt,
    ///   clip: true,
    ///   image("tiger.jpg", width: 100pt, height: 100pt)
    /// )
    /// ```
    #[default(false)]
    pub clip: bool,

    /// Whether this block must stick to the following one, with no break in
    /// between.
    ///
    /// This is, by default, set on heading blocks to prevent orphaned headings
    /// at the bottom of the page.
    ///
    /// ```example
    /// >>> #set page(height: 140pt)
    /// // Disable stickiness of headings.
    /// #show heading: set block(sticky: false)
    /// #lorem(20)
    ///
    /// = Chapter
    /// #lorem(10)
    /// ```
    #[default(false)]
    pub sticky: bool,

    /// The contents of the block.
    #[positional]
    #[borrowed]
    pub body: Option<BlockBody>,
}

impl BlockElem {
    /// Create a block with a custom single-region layouter.
    ///
    /// Such a block must have `breakable: false` (which is set by this
    /// constructor).
    pub fn single_layouter<T: NativeElement>(
        captured: Packed<T>,
        f: fn(
            content: &Packed<T>,
            engine: &mut Engine,
            locator: Locator,
            styles: StyleChain,
            region: Region,
        ) -> SourceResult<Frame>,
    ) -> Self {
        Self::new()
            .with_breakable(false)
            .with_body(Some(BlockBody::SingleLayouter(
                callbacks::BlockSingleCallback::new(captured, f),
            )))
    }

    /// Create a block with a custom multi-region layouter.
    pub fn multi_layouter<T: NativeElement>(
        captured: Packed<T>,
        f: fn(
            content: &Packed<T>,
            engine: &mut Engine,
            locator: Locator,
            styles: StyleChain,
            regions: Regions,
        ) -> SourceResult<Fragment>,
    ) -> Self {
        Self::new().with_body(Some(BlockBody::MultiLayouter(
            callbacks::BlockMultiCallback::new(captured, f),
        )))
    }
}

impl Packed<BlockElem> {
    /// Lay this out as an unbreakable block.
    #[typst_macros::time(name = "block", span = self.span())]
    pub fn layout_single(
        &self,
        engine: &mut Engine,
        locator: Locator,
        styles: StyleChain,
        region: Region,
    ) -> SourceResult<Frame> {
        // Fetch sizing properties.
        let width = self.width(styles);
        let height = self.height(styles);
        let inset = self.inset(styles).unwrap_or_default();

        // Build the pod regions.
        let pod = unbreakable_pod(&width.into(), &height, &inset, styles, region.size);

        // Layout the body.
        let body = self.body(styles);
        let mut frame = match body {
            // If we have no body, just create one frame. Its size will be
            // adjusted below.
            None => Frame::hard(Size::zero()),

            // If we have content as our body, just layout it.
            Some(BlockBody::Content(body)) => {
                layout_frame(engine, body, locator.relayout(), styles, pod)?
            }

            // If we have a child that wants to layout with just access to the
            // base region, give it that.
            Some(BlockBody::SingleLayouter(callback)) => {
                callback.call(engine, locator, styles, pod)?
            }

            // If we have a child that wants to layout with full region access,
            // we layout it.
            Some(BlockBody::MultiLayouter(callback)) => {
                let expand = (pod.expand | region.expand) & pod.size.map(Abs::is_finite);
                let pod = Region { expand, ..pod };
                callback.call(engine, locator, styles, pod.into())?.into_frame()
            }
        };

        // Explicit blocks are boundaries for gradient relativeness.
        if matches!(body, None | Some(BlockBody::Content(_))) {
            frame.set_kind(FrameKind::Hard);
        }

        // Enforce a correct frame size on the expanded axes. Do this before
        // applying the inset, since the pod shrunk.
        frame.set_size(pod.expand.select(pod.size, frame.size()));

        // Apply the inset.
        if !inset.is_zero() {
            crate::layout::grow(&mut frame, &inset);
        }

        // Prepare fill and stroke.
        let fill = self.fill(styles);
        let stroke = self
            .stroke(styles)
            .unwrap_or_default()
            .map(|s| s.map(Stroke::unwrap_or_default));

        // Only fetch these if necessary (for clipping or filling/stroking).
        let outset = Lazy::new(|| self.outset(styles).unwrap_or_default());
        let radius = Lazy::new(|| self.radius(styles).unwrap_or_default());

        // Clip the contents, if requested.
        if self.clip(styles) {
            let size = frame.size() + outset.relative_to(frame.size()).sum_by_axis();
            frame.clip(clip_rect(size, &radius, &stroke));
        }

        // Add fill and/or stroke.
        if fill.is_some() || stroke.iter().any(Option::is_some) {
            frame.fill_and_stroke(fill, &stroke, &outset, &radius, self.span());
        }

        // Assign label to each frame in the fragment.
        if let Some(label) = self.label() {
            frame.label(label);
        }

        Ok(frame)
    }
}

impl Packed<BlockElem> {
    /// Lay this out as a breakable block.
    #[typst_macros::time(name = "block", span = self.span())]
    pub fn layout_multiple(
        &self,
        engine: &mut Engine,
        locator: Locator,
        styles: StyleChain,
        regions: Regions,
    ) -> SourceResult<Fragment> {
        // Fetch sizing properties.
        let width = self.width(styles);
        let height = self.height(styles);
        let inset = self.inset(styles).unwrap_or_default();

        // Allocate a small vector for backlogs.
        let mut buf = SmallVec::<[Abs; 2]>::new();

        // Build the pod regions.
        let pod =
            breakable_pod(&width.into(), &height, &inset, styles, regions, &mut buf);

        // Layout the body.
        let body = self.body(styles);
        let mut fragment = match body {
            // If we have no body, just create one frame plus one per backlog
            // region. We create them zero-sized; if necessary, their size will
            // be adjusted below.
            None => {
                let mut frames = vec![];
                frames.push(Frame::hard(Size::zero()));
                if pod.expand.y {
                    let mut iter = pod;
                    while !iter.backlog.is_empty() {
                        frames.push(Frame::hard(Size::zero()));
                        iter.next();
                    }
                }
                Fragment::frames(frames)
            }

            // If we have content as our body, just layout it.
            Some(BlockBody::Content(body)) => {
                let mut fragment =
                    layout_fragment(engine, body, locator.relayout(), styles, pod)?;

                // If the body is automatically sized and produced more than one
                // fragment, ensure that the width was consistent across all
                // regions. If it wasn't, we need to relayout with expansion.
                if !pod.expand.x
                    && fragment
                        .as_slice()
                        .windows(2)
                        .any(|w| !w[0].width().approx_eq(w[1].width()))
                {
                    let max_width = fragment
                        .iter()
                        .map(|frame| frame.width())
                        .max()
                        .unwrap_or_default();
                    let pod = Regions {
                        size: Size::new(max_width, pod.size.y),
                        expand: Axes::new(true, pod.expand.y),
                        ..pod
                    };
                    fragment = layout_fragment(engine, body, locator, styles, pod)?;
                }

                fragment
            }

            // If we have a child that wants to layout with just access to the
            // base region, give it that.
            Some(BlockBody::SingleLayouter(callback)) => {
                let pod = Region::new(pod.base(), pod.expand);
                callback.call(engine, locator, styles, pod).map(Fragment::frame)?
            }

            // If we have a child that wants to layout with full region access,
            // we layout it.
            //
            // For auto-sized multi-layouters, we propagate the outer expansion
            // so that they can decide for themselves. We also ensure again to
            // only expand if the size is finite.
            Some(BlockBody::MultiLayouter(callback)) => {
                let expand = (pod.expand | regions.expand) & pod.size.map(Abs::is_finite);
                let pod = Regions { expand, ..pod };
                callback.call(engine, locator, styles, pod)?
            }
        };

        // Prepare fill and stroke.
        let fill = self.fill(styles);
        let stroke = self
            .stroke(styles)
            .unwrap_or_default()
            .map(|s| s.map(Stroke::unwrap_or_default));

        // Only fetch these if necessary (for clipping or filling/stroking).
        let outset = Lazy::new(|| self.outset(styles).unwrap_or_default());
        let radius = Lazy::new(|| self.radius(styles).unwrap_or_default());

        // Fetch/compute these outside of the loop.
        let clip = self.clip(styles);
        let has_fill_or_stroke = fill.is_some() || stroke.iter().any(Option::is_some);
        let has_inset = !inset.is_zero();
        let is_explicit = matches!(body, None | Some(BlockBody::Content(_)));

        // Skip filling/stroking the first frame if it is empty and a non-empty
        // one follows.
        let mut skip_first = false;
        if let [first, rest @ ..] = fragment.as_slice() {
            skip_first = has_fill_or_stroke
                && first.is_empty()
                && rest.iter().any(|frame| !frame.is_empty());
        }

        // Post-process to apply insets, clipping, fills, and strokes.
        for (i, (frame, region)) in fragment.iter_mut().zip(pod.iter()).enumerate() {
            // Explicit blocks are boundaries for gradient relativeness.
            if is_explicit {
                frame.set_kind(FrameKind::Hard);
            }

            // Enforce a correct frame size on the expanded axes. Do this before
            // applying the inset, since the pod shrunk.
            frame.set_size(pod.expand.select(region, frame.size()));

            // Apply the inset.
            if has_inset {
                crate::layout::grow(frame, &inset);
            }

            // Clip the contents, if requested.
            if clip {
                let size = frame.size() + outset.relative_to(frame.size()).sum_by_axis();
                frame.clip(clip_rect(size, &radius, &stroke));
            }

            // Add fill and/or stroke.
            if has_fill_or_stroke && (i > 0 || !skip_first) {
                frame.fill_and_stroke(
                    fill.clone(),
                    &stroke,
                    &outset,
                    &radius,
                    self.span(),
                );
            }
        }

        // Assign label to each frame in the fragment.
        if let Some(label) = self.label() {
            for frame in fragment.iter_mut() {
                frame.label(label);
            }
        }

        Ok(fragment)
    }
}

/// The contents of a block.
#[derive(Debug, Clone, PartialEq, Hash)]
pub enum BlockBody {
    /// The block contains normal content.
    Content(Content),
    /// The block contains a layout callback that needs access to just one
    /// base region.
    SingleLayouter(callbacks::BlockSingleCallback),
    /// The block contains a layout callback that needs access to the exact
    /// regions.
    MultiLayouter(callbacks::BlockMultiCallback),
}

impl Default for BlockBody {
    fn default() -> Self {
        Self::Content(Content::default())
    }
}

cast! {
    BlockBody,
    self => match self {
        Self::Content(content) => content.into_value(),
        _ => Value::Auto,
    },
    v: Content => Self::Content(v),
}

/// Defines how to size something along an axis.
#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
pub enum Sizing {
    /// A track that fits its item's contents.
    Auto,
    /// A size specified in absolute terms and relative to the parent's size.
    Rel(Rel),
    /// A size specified as a fraction of the remaining free space in the
    /// parent.
    Fr(Fr),
}

impl Sizing {
    /// Whether this is an automatic sizing.
    pub fn is_auto(self) -> bool {
        matches!(self, Self::Auto)
    }

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

impl From<Smart<Rel>> for Sizing {
    fn from(smart: Smart<Rel>) -> Self {
        match smart {
            Smart::Auto => Self::Auto,
            Smart::Custom(rel) => Self::Rel(rel),
        }
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

/// Builds the pod region for an unbreakable sized container.
fn unbreakable_pod(
    width: &Sizing,
    height: &Sizing,
    inset: &Sides<Rel<Abs>>,
    styles: StyleChain,
    base: Size,
) -> Region {
    // Resolve the size.
    let mut size = Size::new(
        match width {
            // - For auto, the whole region is available.
            // - Fr is handled outside and already factored into the `region`,
            //   so we can treat it equivalently to 100%.
            Sizing::Auto | Sizing::Fr(_) => base.x,
            // Resolve the relative sizing.
            Sizing::Rel(rel) => rel.resolve(styles).relative_to(base.x),
        },
        match height {
            Sizing::Auto | Sizing::Fr(_) => base.y,
            Sizing::Rel(rel) => rel.resolve(styles).relative_to(base.y),
        },
    );

    // Take the inset, if any, into account.
    if !inset.is_zero() {
        size = crate::layout::shrink(size, inset);
    }

    // If the child is manually, the size is forced and we should enable
    // expansion.
    let expand = Axes::new(
        *width != Sizing::Auto && size.x.is_finite(),
        *height != Sizing::Auto && size.y.is_finite(),
    );

    Region::new(size, expand)
}

/// Builds the pod regions for a breakable sized container.
fn breakable_pod<'a>(
    width: &Sizing,
    height: &Sizing,
    inset: &Sides<Rel<Abs>>,
    styles: StyleChain,
    regions: Regions,
    buf: &'a mut SmallVec<[Abs; 2]>,
) -> Regions<'a> {
    let base = regions.base();

    // The vertical region sizes we're about to build.
    let first;
    let full;
    let backlog: &mut [Abs];
    let last;

    // If the block has a fixed height, things are very different, so we
    // handle that case completely separately.
    match height {
        Sizing::Auto | Sizing::Fr(_) => {
            // If the block is automatically sized, we can just inherit the
            // regions.
            first = regions.size.y;
            full = regions.full;
            buf.extend_from_slice(regions.backlog);
            backlog = buf;
            last = regions.last;
        }

        Sizing::Rel(rel) => {
            // Resolve the sizing to a concrete size.
            let resolved = rel.resolve(styles).relative_to(base.y);

            // Since we're manually sized, the resolved size is the base height.
            full = resolved;

            // Distribute the fixed height across a start region and a backlog.
            (first, backlog) = distribute(resolved, regions, buf);

            // If the height is manually sized, we don't want a final repeatable
            // region.
            last = None;
        }
    };

    // Resolve the horizontal sizing to a concrete width and combine
    // `width` and `first` into `size`.
    let mut size = Size::new(
        match width {
            Sizing::Auto | Sizing::Fr(_) => regions.size.x,
            Sizing::Rel(rel) => rel.resolve(styles).relative_to(base.x),
        },
        first,
    );

    // Take the inset, if any, into account, applying it to the
    // individual region components.
    let (mut full, mut last) = (full, last);
    if !inset.is_zero() {
        crate::layout::shrink_multiple(&mut size, &mut full, backlog, &mut last, inset);
    }

    // If the child is manually, the size is forced and we should enable
    // expansion.
    let expand = Axes::new(
        *width != Sizing::Auto && size.x.is_finite(),
        *height != Sizing::Auto && size.y.is_finite(),
    );

    Regions { size, full, backlog, last, expand }
}

/// Distribute a fixed height spread over existing regions into a new first
/// height and a new backlog.
fn distribute<'a>(
    height: Abs,
    mut regions: Regions,
    buf: &'a mut SmallVec<[Abs; 2]>,
) -> (Abs, &'a mut [Abs]) {
    // Build new region heights from old regions.
    let mut remaining = height;
    loop {
        let limited = regions.size.y.clamp(Abs::zero(), remaining);
        buf.push(limited);
        remaining -= limited;
        if remaining.approx_empty()
            || !regions.may_break()
            || (!regions.may_progress() && limited.approx_empty())
        {
            break;
        }
        regions.next();
    }

    // If there is still something remaining, apply it to the
    // last region (it will overflow, but there's nothing else
    // we can do).
    if !remaining.approx_empty() {
        if let Some(last) = buf.last_mut() {
            *last += remaining;
        }
    }

    // Distribute the heights to the first region and the
    // backlog. There is no last region, since the height is
    // fixed.
    (buf[0], &mut buf[1..])
}

/// Manual closure implementations for layout callbacks.
///
/// Normal closures are not `Hash`, so we can't use them.
mod callbacks {
    use super::*;

    macro_rules! callback {
        ($name:ident = ($($param:ident: $param_ty:ty),* $(,)?) -> $ret:ty) => {
            #[derive(Debug, Clone, PartialEq, Hash)]
            pub struct $name {
                captured: Content,
                f: fn(&Content, $($param_ty),*) -> $ret,
            }

            impl $name {
                pub fn new<T: NativeElement>(
                    captured: Packed<T>,
                    f: fn(&Packed<T>, $($param_ty),*) -> $ret,
                ) -> Self {
                    Self {
                        // Type-erased the content.
                        captured: captured.pack(),
                        // Safety: The only difference between the two function
                        // pointer types is the type of the first parameter,
                        // which changes from `&Packed<T>` to `&Content`. This
                        // is safe because:
                        // - `Packed<T>` is a transparent wrapper around
                        //   `Content`, so for any `T` it has the same memory
                        //   representation as `Content`.
                        // - While `Packed<T>` imposes the additional constraint
                        //   that the content is of type `T`, this constraint is
                        //   upheld: It is initially the case because we store a
                        //   `Packed<T>` above. It keeps being the case over the
                        //   lifetime of the closure because `capture` is a
                        //   private field and `Content`'s `Clone` impl is
                        //   guaranteed to retain the type (if it didn't,
                        //   literally everything would break).
                        #[allow(clippy::missing_transmute_annotations)]
                        f: unsafe { std::mem::transmute(f) },
                    }
                }

                pub fn call(&self, $($param: $param_ty),*) -> $ret {
                    (self.f)(&self.captured, $($param),*)
                }
            }
        };
    }

    callback! {
        InlineCallback = (
            engine: &mut Engine,
            locator: Locator,
            styles: StyleChain,
            region: Size,
        ) -> SourceResult<Vec<InlineItem>>
    }

    callback! {
        BlockSingleCallback = (
            engine: &mut Engine,
            locator: Locator,
            styles: StyleChain,
            region: Region,
        ) -> SourceResult<Frame>
    }

    callback! {
        BlockMultiCallback = (
            engine: &mut Engine,
            locator: Locator,
            styles: StyleChain,
            regions: Regions,
        ) -> SourceResult<Fragment>
    }
}
