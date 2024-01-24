use crate::diag::{bail, SourceResult};
use crate::engine::Engine;
use crate::foundations::{elem, Content, Packed, Resolve, StyleChain};
use crate::layout::{
    Abs, AlignElem, Axes, Fragment, Frame, LayoutMultiple, Point, Regions, Size,
};
use crate::util::Numeric;

/// Repeats content to the available space.
///
/// This can be useful when implementing a custom index, reference, or outline.
///
/// Space may be inserted between the instances of the body parameter, so be
/// sure to include negative space if you need the instances to overlap.
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
#[elem(LayoutMultiple)]
pub struct RepeatElem {
    /// The content to repeat.
    #[required]
    pub body: Content,
}

impl LayoutMultiple for Packed<RepeatElem> {
    #[typst_macros::time(name = "repeat", span = self.span())]
    fn layout(
        &self,
        engine: &mut Engine,
        styles: StyleChain,
        regions: Regions,
    ) -> SourceResult<Fragment> {
        let pod = Regions::one(regions.size, Axes::new(false, false));
        let piece = self.body().layout(engine, styles, pod)?.into_frame();
        let align = AlignElem::alignment_in(styles).resolve(styles);

        let fill = regions.size.x;
        let width = piece.width();
        let count = (fill / width).floor();
        let remaining = fill % width;
        let apart = remaining / (count - 1.0);

        let size = Size::new(regions.size.x, piece.height());

        if !size.is_finite() {
            bail!(self.span(), "repeat with no size restrictions");
        }

        let mut frame = Frame::soft(size);
        if piece.has_baseline() {
            frame.set_baseline(piece.baseline());
        }

        let mut offset = Abs::zero();
        if count == 1.0 {
            offset += align.x.position(remaining);
        }

        if width > Abs::zero() {
            for _ in 0..(count as usize).min(1000) {
                frame.push_frame(Point::with_x(offset), piece.clone());
                offset += piece.width() + apart;
            }
        }

        Ok(Fragment::frame(frame))
    }
}
