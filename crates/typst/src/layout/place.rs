use crate::diag::{bail, At, Hint, SourceResult};
use crate::engine::Engine;
use crate::foundations::{elem, Content, Packed, Smart, StyleChain};
use crate::layout::{
    Alignment, Axes, Em, Fragment, LayoutMultiple, Length, Regions, Rel, Size, VAlignment,
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
#[elem(Behave)]
pub struct PlaceElem {
    /// Relative to which position in the parent container to place the content.
    ///
    /// Cannot be `{auto}` if `float` is `{false}` and must be either
    /// `{auto}`, `{top}`, or `{bottom}` if `float` is `{true}`.
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
    /// displacing in-flow content.
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
    pub dx: Rel<Length>,

    /// The vertical displacement of the placed content.
    pub dy: Rel<Length>,

    /// The content to place.
    #[required]
    pub body: Content,
}

impl Packed<PlaceElem> {
    #[typst_macros::time(name = "place", span = self.span())]
    pub fn layout(
        &self,
        engine: &mut Engine,
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
        let frame = child.layout(engine, styles, pod)?.into_frame();
        Ok(Fragment::frame(frame))
    }
}

impl Behave for Packed<PlaceElem> {
    fn behaviour(&self) -> Behaviour {
        Behaviour::Ignorant
    }
}
