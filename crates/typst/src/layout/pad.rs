use crate::diag::SourceResult;
use crate::engine::Engine;
use crate::foundations::{
    elem, Content, NativeElement, Packed, Resolve, Show, StyleChain,
};
use crate::introspection::Locator;
use crate::layout::{
    layout_fragment, Abs, BlockElem, Fragment, Frame, Length, Point, Regions, Rel, Sides,
    Size,
};

/// Adds spacing around content.
///
/// The spacing can be specified for each side individually, or for all sides at
/// once by specifying a positional argument.
///
/// # Example
/// ```example
/// #set align(center)
///
/// #pad(x: 16pt, image("typing.jpg"))
/// _Typing speeds can be
///  measured in words per minute._
/// ```
#[elem(title = "Padding", Show)]
pub struct PadElem {
    /// The padding at the left side.
    #[parse(
        let all = args.named("rest")?.or(args.find()?);
        let x = args.named("x")?.or(all);
        let y = args.named("y")?.or(all);
        args.named("left")?.or(x)
    )]
    pub left: Rel<Length>,

    /// The padding at the top side.
    #[parse(args.named("top")?.or(y))]
    pub top: Rel<Length>,

    /// The padding at the right side.
    #[parse(args.named("right")?.or(x))]
    pub right: Rel<Length>,

    /// The padding at the bottom side.
    #[parse(args.named("bottom")?.or(y))]
    pub bottom: Rel<Length>,

    /// A shorthand to set `left` and `right` to the same value.
    #[external]
    pub x: Rel<Length>,

    /// A shorthand to set `top` and `bottom` to the same value.
    #[external]
    pub y: Rel<Length>,

    /// A shorthand to set all four sides to the same value.
    #[external]
    pub rest: Rel<Length>,

    /// The content to pad at the sides.
    #[required]
    pub body: Content,
}

impl Show for Packed<PadElem> {
    fn show(&self, _: &mut Engine, _: StyleChain) -> SourceResult<Content> {
        Ok(BlockElem::multi_layouter(self.clone(), layout_pad)
            .pack()
            .spanned(self.span()))
    }
}

/// Layout the padded content.
#[typst_macros::time(span = elem.span())]
fn layout_pad(
    elem: &Packed<PadElem>,
    engine: &mut Engine,
    locator: Locator,
    styles: StyleChain,
    regions: Regions,
) -> SourceResult<Fragment> {
    let padding = Sides::new(
        elem.left(styles).resolve(styles),
        elem.top(styles).resolve(styles),
        elem.right(styles).resolve(styles),
        elem.bottom(styles).resolve(styles),
    );

    let mut backlog = vec![];
    let pod = regions.map(&mut backlog, |size| shrink(size, &padding));

    // Layout child into padded regions.
    let mut fragment = layout_fragment(engine, &elem.body, locator, styles, pod)?;

    for frame in &mut fragment {
        grow(frame, &padding);
    }

    Ok(fragment)
}

/// Shrink a region size by an inset relative to the size itself.
pub(crate) fn shrink(size: Size, inset: &Sides<Rel<Abs>>) -> Size {
    size - inset.sum_by_axis().relative_to(size)
}

/// Shrink the components of possibly multiple `Regions` by an inset relative to
/// the regions themselves.
pub(crate) fn shrink_multiple(
    size: &mut Size,
    full: &mut Abs,
    backlog: &mut [Abs],
    last: &mut Option<Abs>,
    inset: &Sides<Rel<Abs>>,
) {
    let summed = inset.sum_by_axis();
    *size -= summed.relative_to(*size);
    *full -= summed.y.relative_to(*full);
    for item in backlog {
        *item -= summed.y.relative_to(*item);
    }
    *last = last.map(|v| v - summed.y.relative_to(v));
}

/// Grow a frame's size by an inset relative to the grown size.
/// This is the inverse operation to `shrink()`.
///
/// For the horizontal axis the derivation looks as follows.
/// (Vertical axis is analogous.)
///
/// Let w be the grown target width,
///     s be the given width,
///     l be the left inset,
///     r be the right inset,
///     p = l + r.
///
/// We want that: w - l.resolve(w) - r.resolve(w) = s
///
/// Thus: w - l.resolve(w) - r.resolve(w) = s
///   <=> w - p.resolve(w) = s
///   <=> w - p.rel * w - p.abs = s
///   <=> (1 - p.rel) * w = s + p.abs
///   <=> w = (s + p.abs) / (1 - p.rel)
pub(crate) fn grow(frame: &mut Frame, inset: &Sides<Rel<Abs>>) {
    // Apply the padding inversely such that the grown size padded
    // yields the frame's size.
    let padded = frame
        .size()
        .zip_map(inset.sum_by_axis(), |s, p| (s + p.abs) / (1.0 - p.rel.get()));

    let inset = inset.relative_to(padded);
    let offset = Point::new(inset.left, inset.top);

    // Grow the frame and translate everything in the frame inwards.
    frame.set_size(padded);
    frame.translate(offset);
}
