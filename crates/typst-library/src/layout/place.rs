use crate::prelude::*;

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
#[elem(Layout, Behave)]
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
    #[default(Smart::Custom(Align::START))]
    pub alignment: Smart<Align>,

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

impl Layout for PlaceElem {
    #[tracing::instrument(name = "PlaceElem::layout", skip_all)]
    fn layout(
        &self,
        vt: &mut Vt,
        styles: StyleChain,
        regions: Regions,
    ) -> SourceResult<Fragment> {
        // The pod is the base area of the region because for absolute
        // placement we don't really care about the already used area.
        let base = regions.base();
        let float = self.float(styles);
        let alignment = self.alignment(styles);

        if float && alignment.map_or(false, |align| align.y() == Some(VAlign::Horizon)) {
            bail!(self.span(), "floating placement must be `auto`, `top`, or `bottom`");
        } else if !float && alignment.is_auto() {
            return Err("automatic positioning is only available for floating placement")
                .hint("you can enable floating placement with `place(float: true, ..)`")
                .at(self.span());
        }

        let child = self.body().aligned(alignment.unwrap_or_else(|| Align::CENTER));

        let pod = Regions::one(base, Axes::splat(false));
        let frame = child.layout(vt, styles, pod)?.into_frame();
        Ok(Fragment::frame(frame))
    }
}

impl Behave for PlaceElem {
    fn behaviour(&self) -> Behaviour {
        Behaviour::Ignorant
    }
}
