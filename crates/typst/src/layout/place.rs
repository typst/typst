use crate::foundations::{elem, scope, Cast, Content, Smart};
use crate::layout::{Alignment, Em, Length, Rel};

/// Places content at an absolute position.
///
/// Placed content will not affect the position of other content. Place is
/// always relative to its parent container and will be in the foreground of all
/// other content in the container. Page margins will be respected.
///
///
/// # Example
/// ```example
/// #set page(height: 60pt)
/// Hello, world!
///
/// #place(
///   top + right,
///   square(
///     width: 20pt,
///     stroke: 2pt + blue
///   ),
/// )
/// ```
#[elem(scope)]
pub struct PlaceElem {
    /// Relative to which position in the parent container to place the content.
    ///
    /// - If `float` is `{false}`, then this can be any alignment other than `{auto}`.
    /// - If `float` is `{true}`, then this must be `{auto}`, `{top}`, or `{bottom}`.
    ///
    /// When an axis of the page is `{auto}` sized, all alignments relative to
    /// that axis will be ignored, instead, the item will be placed in the
    /// origin of the axis.
    #[positional]
    #[default(Smart::Custom(Alignment::START))]
    pub alignment: Smart<Alignment>,

    /// Relative to which containing scope something is placed.
    ///
    /// The parent scope is primarily used with figures and, for
    /// this reason, the figure function has a mirrored [`scope`
    /// parameter]($figure.scope). Nonetheless, it can also be more generally
    /// useful to break out of the columns. A typical example would be to
    /// [create a single-column title section]($guides/page-setup/#columns) in a
    /// two-column document.
    ///
    /// Note that parent-scoped placement is currently only supported if `float`
    /// is `{true}`. This may change in the future.
    pub scope: PlacementScope,

    /// Whether the placed element has floating layout.
    ///
    /// Floating elements are positioned at the top or bottom of the parent
    /// container, displacing in-flow content. They are always placed in the
    /// in-flow order relative to each other, as well as before any content
    /// following a later [`place.flush`] element.
    ///
    /// ```example
    /// #set page(height: 150pt)
    /// #let note(where, body) = place(
    ///   center + where,
    ///   float: true,
    ///   clearance: 6pt,
    ///   rect(body),
    /// )
    ///
    /// #lorem(10)
    /// #note(bottom)[Bottom 1]
    /// #note(bottom)[Bottom 2]
    /// #lorem(40)
    /// #note(top)[Top]
    /// #lorem(10)
    /// ```
    pub float: bool,

    /// The amount of clearance the placed element has in a floating layout.
    ///
    /// Has no effect if `float` is `{false}`.
    #[default(Em::new(1.5).into())]
    #[resolve]
    pub clearance: Length,

    /// The horizontal displacement of the placed content.
    ///
    /// ```example
    /// #set page(height: 100pt)
    /// #for i in range(16) {
    ///   let amount = i * 4pt
    ///   place(center, dx: amount - 32pt, dy: amount)[A]
    /// }
    /// ```
    ///
    /// This does not affect the layout of in-flow content.
    /// In other words, the placed content is treated as if it
    /// were wrapped in a [`move`] element.
    pub dx: Rel<Length>,

    /// The vertical displacement of the placed content.
    ///
    /// This does not affect the layout of in-flow content.
    /// In other words, the placed content is treated as if it
    /// were wrapped in a [`move`] element.
    pub dy: Rel<Length>,

    /// The content to place.
    #[required]
    pub body: Content,
}

#[scope]
impl PlaceElem {
    #[elem]
    type FlushElem;
}

/// Relative to which containing scope something shall be placed.
#[derive(Debug, Default, Copy, Clone, Eq, PartialEq, Hash, Cast)]
pub enum PlacementScope {
    /// Place into the current column.
    #[default]
    Column,
    /// Place relative to the parent, letting the content span over all columns.
    Parent,
}

/// Asks the layout algorithm to place pending floating elements before
/// continuing with the content.
///
/// This is useful for preventing floating figures from spilling
/// into the next section.
///
/// ```example
/// #set page(height: 165pt, width: 150pt)
///
/// Some introductory text: #lorem(15)
///
/// #figure(
///   rect(
///     width: 100%,
///     height: 64pt,
///     [I float with a caption!],
///   ),
///   placement: auto,
///   caption: [A self-describing figure],
/// )
///
/// #place.flush()
///
/// Some conclusive text that must occur
/// after the figure.
/// ```
#[elem]
pub struct FlushElem {}
