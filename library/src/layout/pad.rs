use crate::prelude::*;

/// Add spacing around content.
///
/// The `pad` function adds spacing around content. The spacing can be specified
/// for each side individually, or for all sides at once by specifying a
/// positional argument.
///
/// ## Example
/// ```example
/// #set align(center)
///
/// #pad(x: 16pt, image("typing.jpg"))
/// _Typing speeds can be
///  measured in words per minute._
/// ```
///
/// ## Parameters
/// - x: `Rel<Length>` (named)
///   The horizontal padding. Both `left` and `right` take precedence over this.
///
/// - y: `Rel<Length>` (named)
///   The vertical padding. Both `top` and `bottom` take precedence over this.
///
/// - rest: `Rel<Length>` (named)
///   The padding for all sides. All other parameters take precedence over this.
///
/// Display: Padding
/// Category: layout
#[node(Construct, Layout)]
pub struct PadNode {
    /// The content to pad at the sides.
    #[positional]
    #[required]
    pub body: Content,

    /// The padding at the left side.
    #[named]
    #[default]
    pub left: Rel<Length>,

    /// The padding at the right side.
    #[named]
    #[default]
    pub right: Rel<Length>,

    /// The padding at the top side.
    #[named]
    #[default]
    pub top: Rel<Length>,

    /// The padding at the bottom side.
    #[named]
    #[default]
    pub bottom: Rel<Length>,
}

impl Construct for PadNode {
    fn construct(_: &Vm, args: &mut Args) -> SourceResult<Content> {
        let all = args.named("rest")?.or(args.find()?);
        let x = args.named("x")?;
        let y = args.named("y")?;
        let left = args.named("left")?.or(x).or(all).unwrap_or_default();
        let top = args.named("top")?.or(y).or(all).unwrap_or_default();
        let right = args.named("right")?.or(x).or(all).unwrap_or_default();
        let bottom = args.named("bottom")?.or(y).or(all).unwrap_or_default();
        let body = args.expect::<Content>("body")?;
        Ok(Self::new(body)
            .with_left(left)
            .with_top(top)
            .with_bottom(bottom)
            .with_right(right)
            .pack())
    }
}

impl Layout for PadNode {
    fn layout(
        &self,
        vt: &mut Vt,
        styles: StyleChain,
        regions: Regions,
    ) -> SourceResult<Fragment> {
        let mut backlog = vec![];

        // Layout child into padded regions.
        let sides = Sides::new(self.left(), self.top(), self.right(), self.bottom());
        let padding = sides.resolve(styles);
        let pod = regions.map(&mut backlog, |size| shrink(size, padding));
        let mut fragment = self.body().layout(vt, styles, pod)?;

        for frame in &mut fragment {
            // Apply the padding inversely such that the grown size padded
            // yields the frame's size.
            let padded = grow(frame.size(), padding);
            let padding = padding.relative_to(padded);
            let offset = Point::new(padding.left, padding.top);

            // Grow the frame and translate everything in the frame inwards.
            frame.set_size(padded);
            frame.translate(offset);
        }

        Ok(fragment)
    }
}

/// Shrink a size by padding relative to the size itself.
fn shrink(size: Size, padding: Sides<Rel<Abs>>) -> Size {
    size - padding.relative_to(size).sum_by_axis()
}

/// Grow a size by padding relative to the grown size.
/// This is the inverse operation to `shrink()`.
///
/// For the horizontal axis the derivation looks as follows.
/// (Vertical axis is analogous.)
///
/// Let w be the grown target width,
///     s be the given width,
///     l be the left padding,
///     r be the right padding,
///     p = l + r.
///
/// We want that: w - l.resolve(w) - r.resolve(w) = s
///
/// Thus: w - l.resolve(w) - r.resolve(w) = s
///   <=> w - p.resolve(w) = s
///   <=> w - p.rel * w - p.abs = s
///   <=> (1 - p.rel) * w = s + p.abs
///   <=> w = (s + p.abs) / (1 - p.rel)
fn grow(size: Size, padding: Sides<Rel<Abs>>) -> Size {
    size.zip(padding.sum_by_axis())
        .map(|(s, p)| (s + p.abs).safe_div(1.0 - p.rel.get()))
}
