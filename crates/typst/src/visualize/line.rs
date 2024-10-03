use crate::diag::{bail, SourceResult};
use crate::engine::Engine;
use crate::foundations::{elem, Content, NativeElement, Packed, Show, StyleChain};
use crate::introspection::Locator;
use crate::layout::{
    Abs, Angle, Axes, BlockElem, Frame, FrameItem, Length, Region, Rel, Size,
};
use crate::utils::Numeric;
use crate::visualize::{Geometry, Stroke};

/// A line from one point to another.
///
/// # Example
/// ```example
/// #set page(height: 100pt)
///
/// #line(length: 100%)
/// #line(end: (50%, 50%))
/// #line(
///   length: 4cm,
///   stroke: 2pt + maroon,
/// )
/// ```
#[elem(Show)]
pub struct LineElem {
    /// The start point of the line.
    ///
    /// Must be an array of exactly two relative lengths.
    #[resolve]
    pub start: Axes<Rel<Length>>,

    /// The offset from `start` where the line ends.
    #[resolve]
    pub end: Option<Axes<Rel<Length>>>,

    /// The line's length. This is only respected if `end` is `{none}`.
    #[resolve]
    #[default(Abs::pt(30.0).into())]
    pub length: Rel<Length>,

    /// The angle at which the line points away from the origin. This is only
    /// respected if `end` is `{none}`.
    pub angle: Angle,

    /// How to [stroke] the line.
    ///
    /// ```example
    /// #set line(length: 100%)
    /// #stack(
    ///   spacing: 1em,
    ///   line(stroke: 2pt + red),
    ///   line(stroke: (paint: blue, thickness: 4pt, cap: "round")),
    ///   line(stroke: (paint: blue, thickness: 1pt, dash: "dashed")),
    ///   line(stroke: (paint: blue, thickness: 1pt, dash: ("dot", 2pt, 4pt, 2pt))),
    /// )
    /// ```
    #[resolve]
    #[fold]
    pub stroke: Stroke,
}

impl Show for Packed<LineElem> {
    fn show(&self, _: &mut Engine, _: StyleChain) -> SourceResult<Content> {
        Ok(BlockElem::single_layouter(self.clone(), layout_line)
            .pack()
            .spanned(self.span()))
    }
}

/// Layout the line.
#[typst_macros::time(span = elem.span())]
fn layout_line(
    elem: &Packed<LineElem>,
    _: &mut Engine,
    _: Locator,
    styles: StyleChain,
    region: Region,
) -> SourceResult<Frame> {
    let resolve = |axes: Axes<Rel<Abs>>| axes.zip_map(region.size, Rel::relative_to);
    let start = resolve(elem.start(styles));
    let delta = elem.end(styles).map(|end| resolve(end) - start).unwrap_or_else(|| {
        let length = elem.length(styles);
        let angle = elem.angle(styles);
        let x = angle.cos() * length;
        let y = angle.sin() * length;
        resolve(Axes::new(x, y))
    });

    let stroke = elem.stroke(styles).unwrap_or_default();
    let size = start.max(start + delta).max(Size::zero());

    if !size.is_finite() {
        bail!(elem.span(), "cannot create line with infinite length");
    }

    let mut frame = Frame::soft(size);
    let shape = Geometry::Line(delta.to_point()).stroked(stroke);
    frame.push(start.to_point(), FrameItem::Shape(shape, elem.span()));
    Ok(frame)
}
