use crate::diag::SourceResult;
use crate::engine::Engine;
use crate::foundations::{elem, Content, NativeElement, Packed, Show, StyleChain};
use crate::layout::{Abs, Angle, Axes, BlockElem, Length, Rel};
use crate::visualize::Stroke;

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
    pub start: Axes<Rel<Length>>,

    /// The point where the line ends.
    pub end: Option<Axes<Rel<Length>>>,

    /// The line's length. This is only respected if `end` is `{none}`.
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
    #[fold]
    pub stroke: Stroke,
}

impl Show for Packed<LineElem> {
    fn show(&self, engine: &mut Engine, _: StyleChain) -> SourceResult<Content> {
        Ok(BlockElem::single_layouter(self.clone(), engine.routines.layout_line)
            .pack()
            .spanned(self.span()))
    }
}
