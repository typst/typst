use crate::diag::{bail, At, Hint, SourceResult};
use crate::engine::Engine;
use crate::foundations::{elem, scope, Content, Packed, Smart, StyleChain, Unlabellable};
use crate::introspection::Locator;
use crate::layout::{
    Alignment, Axes, Em, Fragment, Length, Regions, Rel, Size, VAlignment,
};
use crate::realize::{Behave, Behaviour};

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
#[elem(scope, Behave)]
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

    /// Whether the placed element has floating layout.
    ///
    /// Floating elements are positioned at the top or bottom of the page,
    /// displacing in-flow content. They are always placed in the in-flow
    /// order relative to each other, as well as before any content following
    /// a later [`place.flush`] element.
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

impl Packed<PlaceElem> {
    #[typst_macros::time(name = "place", span = self.span())]
    pub fn layout(
        &self,
        engine: &mut Engine,
        locator: Locator,
        styles: StyleChain,
        base: Size,
    ) -> SourceResult<Fragment> {
        // The pod is the base area of the region because for absolute
        // placement we don't really care about the already used area.
        let float = self.float(styles);
        let alignment = self.alignment(styles);

        if float
            && alignment.is_custom_and(|align| {
                matches!(align.y(), None | Some(VAlignment::Horizon))
            })
        {
            bail!(self.span(), "floating placement must be `auto`, `top`, or `bottom`");
        } else if !float && alignment.is_auto() {
            return Err("automatic positioning is only available for floating placement")
                .hint("you can enable floating placement with `place(float: true, ..)`")
                .at(self.span());
        }

        let child = self
            .body()
            .clone()
            .aligned(alignment.unwrap_or_else(|| Alignment::CENTER));

        let pod = Regions::one(base, Axes::splat(false));
        let frame = child.layout(engine, locator, styles, pod)?.into_frame();
        Ok(Fragment::frame(frame))
    }
}

impl Behave for Packed<PlaceElem> {
    fn behaviour(&self) -> Behaviour {
        Behaviour::Ignorant
    }
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
#[elem(Behave, Unlabellable)]
pub struct FlushElem {}

impl Behave for Packed<FlushElem> {
    fn behaviour(&self) -> Behaviour {
        Behaviour::Invisible
    }
}

impl Unlabellable for Packed<FlushElem> {}
