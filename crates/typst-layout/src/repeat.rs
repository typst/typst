use typst_library::diag::{SourceResult, bail};
use typst_library::engine::Engine;
use typst_library::foundations::{Packed, Resolve, StyleChain};
use typst_library::introspection::Locator;
use typst_library::layout::{
    Abs, AlignElem, Axes, Frame, Point, Region, RepeatElem, Size,
};
use typst_utils::Numeric;

/// Layout the repeated content.
#[typst_macros::time(span = elem.span())]
pub fn layout_repeat(
    elem: &Packed<RepeatElem>,
    engine: &mut Engine,
    locator: Locator,
    styles: StyleChain,
    region: Region,
) -> SourceResult<Frame> {
    let pod = Region::new(region.size, Axes::new(false, false));
    let piece = crate::layout_frame(engine, &elem.body, locator, styles, pod)?;
    let size = Size::new(region.size.x, piece.height());

    if !size.is_finite() {
        bail!(elem.span(), "repeat with no size restrictions");
    }

    let mut frame = Frame::soft(size);
    if piece.has_baseline() {
        frame.set_baseline(piece.baseline());
    }

    let mut gap = elem.gap(styles).resolve(styles);
    let fill = region.size.x;
    let width = piece.width();

    // We need to fit the body N times, but the number of gaps is (N - 1):
    // N * w + (N - 1) * g ≤ F
    // where N - body count (count)
    //       w - body width (width)
    //       g - gap width (gap)
    //       F - available space to fill (fill)
    //
    // N * w + N * g - g ≤ F
    // N * (w + g) ≤ F + g
    // N ≤ (F + g) / (w + g)
    // N = ⌊(F + g) / (w + g)⌋
    let count = ((fill + gap) / (width + gap)).floor();
    let remaining = (fill + gap) % (width + gap);

    let justify = elem.justify(styles);
    if justify {
        gap += remaining / (count - 1.0);
    }

    let align = AlignElem::alignment_in(styles).resolve(styles);
    let mut offset = Abs::zero();
    if count == 1.0 || !justify {
        offset += align.x.position(remaining);
    }

    if width > Abs::zero() {
        for _ in 0..(count as usize).min(1000) {
            frame.push_frame(Point::with_x(offset), piece.clone());
            offset += width + gap;
        }
    }

    Ok(frame)
}
