use crate::diag::{bail, SourceResult};
use crate::engine::Engine;
use crate::foundations::{
    elem, Content, NativeElement, Packed, Resolve, Show, StyleChain,
};
use crate::introspection::Locator;
use crate::layout::{
    layout_frame, Abs, AlignElem, Axes, BlockElem, Frame, Length, Point, Region, Size,
};
use crate::utils::Numeric;

/// Repeats content to the available space.
///
/// This can be useful when implementing a custom index, reference, or outline.
///
/// Space may be inserted between the instances of the body parameter, so be
/// sure to adjust the [`justify`]($repeat.justify) parameter accordingly.
///
/// Errors if there no bounds on the available space, as it would create
/// infinite content.
///
/// # Example
/// ```example
/// Sign on the dotted line:
/// #box(width: 1fr, repeat[.])
///
/// #set text(10pt)
/// #v(8pt, weak: true)
/// #align(right)[
///   Berlin, the 22nd of December, 2022
/// ]
/// ```
#[elem(Show)]
pub struct RepeatElem {
    /// The content to repeat.
    #[required]
    pub body: Content,

    /// The gap between each instance of the body.
    #[default]
    pub gap: Length,

    /// Whether to increase the gap between instances to completely fill the
    /// available space.
    #[default(true)]
    pub justify: bool,
}

impl Show for Packed<RepeatElem> {
    fn show(&self, _: &mut Engine, _: StyleChain) -> SourceResult<Content> {
        Ok(BlockElem::single_layouter(self.clone(), layout_repeat)
            .pack()
            .spanned(self.span()))
    }
}

/// Layout the repeated content.
#[typst_macros::time(span = elem.span())]
fn layout_repeat(
    elem: &Packed<RepeatElem>,
    engine: &mut Engine,
    locator: Locator,
    styles: StyleChain,
    region: Region,
) -> SourceResult<Frame> {
    let pod = Region::new(region.size, Axes::new(false, false));
    let piece = layout_frame(engine, &elem.body, locator, styles, pod)?;
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

    // count * width + (count - 1) * gap = fill, but count is an integer so
    // we need to round down and get the remainder.
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
            offset += piece.width() + gap;
        }
    }

    Ok(frame)
}
