use typst_library::diag::SourceResult;
use typst_library::engine::Engine;
use typst_library::foundations::{Packed, StyleChain};
use typst_library::introspection::Locator;
use typst_library::layout::{
    Abs, Fragment, Frame, PadElem, Point, Regions, Rel, Sides, Size,
};

/// Layout the padded content.
#[typst_macros::time(span = elem.span())]
pub fn layout_pad(
    elem: &Packed<PadElem>,
    engine: &mut Engine,
    locator: Locator,
    styles: StyleChain,
    regions: Regions,
) -> SourceResult<Fragment> {
    let padding = Sides::new(
        elem.left.resolve(styles),
        elem.top.resolve(styles),
        elem.right.resolve(styles),
        elem.bottom.resolve(styles),
    );

    let mut backlog = vec![];
    let pod = regions.map(&mut backlog, |size| shrink(size, &padding));

    // Layout child into padded regions.
    let mut fragment = crate::layout_fragment(engine, &elem.body, locator, styles, pod)?;

    for frame in &mut fragment {
        grow(frame, &padding);
    }

    Ok(fragment)
}

/// Shrink a region size by an inset relative to the size itself.
pub fn shrink(size: Size, inset: &Sides<Rel<Abs>>) -> Size {
    size - inset.sum_by_axis().relative_to(size)
}

/// Shrink the components of possibly multiple `Regions` by an inset relative to
/// the regions themselves.
pub fn shrink_multiple(
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
pub fn grow(frame: &mut Frame, inset: &Sides<Rel<Abs>>) {
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
