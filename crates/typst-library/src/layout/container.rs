use crate::diag::{SourceResult, bail};
use crate::engine::Engine;
use crate::foundations::{
    Args, AutoValue, Construct, Content, NativeElement, Packed, Smart, StyleChain, Value,
    cast, elem,
};
use crate::introspection::Locator;
use crate::layout::{
    Abs, Corners, Em, Fr, Fragment, Frame, Length, Region, Regions, Rel, Sides, Size,
    Spacing,
};
use crate::visualize::{Paint, Stroke};

/// An inline-level container that sizes content.
///
/// All elements except inline math, text, and boxes are block-level and cannot
/// occur inside of a [paragraph]($par). The box function can be used to
/// integrate such elements into a paragraph. Boxes take the size of their
/// contents by default but can also be sized explicitly.
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
    pub baseline: Rel<Length>,

    /// The box's background color. See the
    /// [rectangle's documentation]($rect.fill) for more details.
    pub fill: Option<Paint>,

    /// The box's border color. See the
    /// [rectangle's documentation]($rect.stroke) for more details.
    #[fold]
    pub stroke: Sides<Option<Option<Stroke>>>,

    /// How much to round the box's corners. See the
    /// [rectangle's documentation]($rect.radius) for more details.
    #[fold]
    pub radius: Corners<Option<Rel<Length>>>,

    /// How much to pad the box's content.
    ///
    /// This can be a single length for all sides or a dictionary of lengths
    /// for individual sides. When passing a dictionary, it can contain the
    /// following keys in order of precedence: `top`, `right`, `bottom`, `left`
    /// (controlling the respective cell sides), `x`, `y` (controlling vertical
    /// and horizontal insets), and `rest` (covers all insets not styled by
    /// other dictionary entries). All keys are optional; omitted keys will use
    /// their previously set value, or the default value if never set.
    ///
    /// [Relative lengths]($relative) for this parameter are relative to the box
    /// size excluding [outset]($box.outset). Note that relative insets and
    /// outsets are different from relative [widths]($box.width) and
    /// [heights]($box.height), which are relative to the container.
    ///
    /// _Note:_ When the box contains text, its exact size depends on the
    /// current [text edges]($text.top-edge).
    ///
    /// ```example
    /// #rect(inset: 0pt)[Tight]
    /// ```
    #[fold]
    pub inset: Sides<Option<Rel<Length>>>,

    /// How much to expand the box's size without affecting the layout.
    ///
    /// This can be a single length for all sides or a dictionary of lengths for
    /// individual sides. [Relative lengths]($relative) for this parameter are
    /// relative to the box size excluding outset. See the documentation for
    /// [inset]($box.inset) above for further details.
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
    pub body: Option<Content>,
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
        self.body.call(engine, locator, styles, region)
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
/// Blocks are also the primary way to control whether text becomes part of a
/// paragraph or not. See [the paragraph documentation]($par/#what-becomes-a-paragraph)
/// for more details.
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
    #[fold]
    pub stroke: Sides<Option<Option<Stroke>>>,

    /// How much to round the block's corners. See the
    /// [rectangle's documentation]($rect.radius) for more details.
    #[fold]
    pub radius: Corners<Option<Rel<Length>>>,

    /// How much to pad the block's content. See the
    /// [box's documentation]($box.inset) for more details.
    #[fold]
    pub inset: Sides<Option<Rel<Length>>>,

    /// How much to expand the block's size without affecting the layout. See
    /// the [box's documentation]($box.outset) for more details.
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
#[derive(Debug, Default, Copy, Clone, Eq, PartialEq, Hash)]
pub enum Sizing {
    /// A track that fits its item's contents.
    #[default]
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

/// Manual closure implementations for layout callbacks.
///
/// Normal closures are not `Hash`, so we can't use them.
mod callbacks {
    use super::*;

    macro_rules! callback {
        ($name:ident = ($($param:ident: $param_ty:ty),* $(,)?) -> $ret:ty) => {
            #[derive(Debug, Clone, Hash)]
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

            impl PartialEq for $name {
                fn eq(&self, other: &Self) -> bool {
                    // Comparing function pointers is problematic. Since for
                    // each type of content, there is typically just one
                    // callback, we skip it. It barely matters anyway since
                    // getting into a comparison codepath for inline & block
                    // elements containing callback bodies is close to
                    // impossible (as these are generally generated in show
                    // rules).
                    self.captured.eq(&other.captured)
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
