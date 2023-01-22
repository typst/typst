use crate::prelude::*;

/// # Place
/// Place content at an absolute position.
///
/// Placed content will not affect the position of other content. Place is
/// always relative to its parent container and will be in the foreground of all
/// other content in the container. Page margins will be respected.
///
///
/// ## Example
/// ```
/// #set page(height: 60pt)
/// Hello, world!
///
/// #place(
///   top + right,
///   square(
///     width: 10pt,
///     stroke: 1pt + blue
///   ),
/// )
/// ```
///
/// ## Parameters
/// - alignment: Axes<Option<GenAlign>> (positional)
///   Relative to which position in the parent container to place the content.
///
///   When an axis of the page is `{auto}` sized, all alignments relative to that
///   axis will be ignored, instead, the item will be placed in the origin of the
///   axis.
///
/// - body: Content (positional, required)
///   The content to place.
///
/// - dx: Rel<Length> (named)
///   The horizontal displacement of the placed content.
///
///   ### Example
///   ```
///   #set align(center)
///
///   #box(
///     width: 80pt,
///     height: 80pt,
///     {
///       for i in range(18) {
///         let amount = i * 4pt
///         place(dx: amount, dy: amount)[A]
///       }
///     }
///   )
///   ```
///
/// - dy: Rel<Length> (named)
///   The vertical displacement of the placed content.
///
/// ## Category
/// layout
#[func]
#[capable(Layout, Behave)]
#[derive(Debug, Hash)]
pub struct PlaceNode(pub Content, bool);

#[node]
impl PlaceNode {
    fn construct(_: &Vm, args: &mut Args) -> SourceResult<Content> {
        let aligns = args.find()?.unwrap_or(Axes::with_x(Some(GenAlign::Start)));
        let dx = args.named("dx")?.unwrap_or_default();
        let dy = args.named("dy")?.unwrap_or_default();
        let body = args.expect::<Content>("body")?;
        let out_of_flow = aligns.y.is_some();
        Ok(Self(body.moved(Axes::new(dx, dy)).aligned(aligns), out_of_flow).pack())
    }

    fn field(&self, name: &str) -> Option<Value> {
        match name {
            "body" => Some(Value::Content(self.0.clone())),
            _ => None,
        }
    }
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
            let finite = regions.base.map(Abs::is_finite);
            let expand = finite & (regions.expand | out_of_flow);
            Regions::one(regions.base, regions.base, expand)
        };

        let mut frame = self.0.layout(vt, styles, pod)?.into_frame();

        // If expansion is off, zero all sizes so that we don't take up any
        // space in our parent. Otherwise, respect the expand settings.
        let target = regions.expand.select(regions.first, Size::zero());
        frame.resize(target, Align::LEFT_TOP);

        Ok(Fragment::frame(frame))
    }
}

impl PlaceNode {
    /// Whether this node wants to be placed relative to its its parent's base
    /// origin. Instead of relative to the parent's current flow/cursor
    /// position.
    pub fn out_of_flow(&self) -> bool {
        self.1
    }
}

impl Behave for PlaceNode {
    fn behaviour(&self) -> Behaviour {
        Behaviour::Ignorant
    }
}
