use crate::diag::SourceResult;
use crate::engine::Engine;
use crate::foundations::{elem, Content, Packed, Resolve, StyleChain};
use crate::layout::{
    Abs, Fragment, LayoutMultiple, Length, Point, Regions, Rel, Sides, Size,
};

/// Adds spacing around content.
///
/// The spacing can be specified for each side individually, or for all sides at
/// once by specifying a positional argument.
///
/// # Example
/// ```example
/// #set align(center)
///
/// #pad(x: 16pt, image("typing.jpg"))
/// _Typing speeds can be
///  measured in words per minute._
/// ```
#[elem(title = "Padding", LayoutMultiple)]
pub struct PadElem {
    /// The padding at the left side.
    #[parse(
        let all = args.named("rest")?.or(args.find()?);
        let x = args.named("x")?.or(all);
        let y = args.named("y")?.or(all);
        args.named("left")?.or(x)
    )]
    pub left: Rel<Length>,

    /// The padding at the top side.
    #[parse(args.named("top")?.or(y))]
    pub top: Rel<Length>,

    /// The padding at the right side.
    #[parse(args.named("right")?.or(x))]
    pub right: Rel<Length>,

    /// The padding at the bottom side.
    #[parse(args.named("bottom")?.or(y))]
    pub bottom: Rel<Length>,

    /// The horizontal padding. Both `left` and `right` take precedence over
    /// this.
    #[external]
    pub x: Rel<Length>,

    /// The vertical padding. Both `top` and `bottom` take precedence over this.
    #[external]
    pub y: Rel<Length>,

    /// The padding for all sides. All other parameters take precedence over
    /// this.
    #[external]
    pub rest: Rel<Length>,

    /// The content to pad at the sides.
    #[required]
    pub body: Content,
}

impl LayoutMultiple for Packed<PadElem> {
    #[typst_macros::time(name = "pad", span = self.span())]
    fn layout(
        &self,
        engine: &mut Engine,
        styles: StyleChain,
        regions: Regions,
    ) -> SourceResult<Fragment> {
        let sides = Sides::new(
            self.left(styles),
            self.top(styles),
            self.right(styles),
            self.bottom(styles),
        );

        // Layout child into padded regions.
        let mut backlog = vec![];
        let padding = sides.resolve(styles);
        let pod = regions.map(&mut backlog, |size| shrink(size, padding));
        let mut fragment = self.body().layout(engine, styles, pod)?;

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
    size.zip_map(padding.sum_by_axis(), |s, p| (s + p.abs) / (1.0 - p.rel.get()))
}
