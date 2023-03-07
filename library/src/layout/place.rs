use crate::prelude::*;

/// Place content at an absolute position.
///
/// Placed content will not affect the position of other content. Place is
/// always relative to its parent container and will be in the foreground of all
/// other content in the container. Page margins will be respected.
///
///
/// ## Example
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
///
/// Display: Place
/// Category: layout
#[node(Layout, Behave)]
pub struct PlaceNode {
    /// Relative to which position in the parent container to place the content.
    ///
    /// When an axis of the page is `{auto}` sized, all alignments relative to that
    /// axis will be ignored, instead, the item will be placed in the origin of the
    /// axis.
    #[positional]
    #[default(Axes::with_x(Some(GenAlign::Start)))]
    pub alignment: Axes<Option<GenAlign>>,

    /// The content to place.
    #[positional]
    #[required]
    pub body: Content,

    /// The horizontal displacement of the placed content.
    ///
    /// ```example
    /// #set page(height: 100pt)
    /// #for i in range(16) {
    ///   let amount = i * 4pt
    ///   place(center, dx: amount - 32pt, dy: amount)[A]
    /// }
    /// ```
    #[named]
    #[default]
    pub dx: Rel<Length>,

    /// The vertical displacement of the placed content.
    #[named]
    #[default]
    pub dy: Rel<Length>,
}

impl Layout for PlaceNode {
    fn layout(
        &self,
        vt: &mut Vt,
        styles: StyleChain,
        regions: Regions,
    ) -> SourceResult<Fragment> {
        let out_of_flow = self.out_of_flow();

        // The pod is the base area of the region because for absolute
        // placement we don't really care about the already used area.
        let pod = {
            let finite = regions.base().map(Abs::is_finite);
            let expand = finite & (regions.expand | out_of_flow);
            Regions::one(regions.base(), expand)
        };

        let child = self
            .body()
            .moved(Axes::new(self.dx(), self.dy()))
            .aligned(self.alignment());

        let mut frame = child.layout(vt, styles, pod)?.into_frame();

        // If expansion is off, zero all sizes so that we don't take up any
        // space in our parent. Otherwise, respect the expand settings.
        let target = regions.expand.select(regions.size, Size::zero());
        frame.resize(target, Align::LEFT_TOP);

        Ok(Fragment::frame(frame))
    }
}

impl PlaceNode {
    /// Whether this node wants to be placed relative to its its parent's base
    /// origin. Instead of relative to the parent's current flow/cursor
    /// position.
    pub fn out_of_flow(&self) -> bool {
        self.alignment().y.is_some()
    }
}

impl Behave for PlaceNode {
    fn behaviour(&self) -> Behaviour {
        Behaviour::Ignorant
    }
}
