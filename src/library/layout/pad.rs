use crate::library::prelude::*;

/// Pad a node at the sides.
#[derive(Debug, Hash)]
pub struct PadNode {
    /// The amount of padding.
    pub padding: Sides<Linear>,
    /// The child node whose sides to pad.
    pub child: LayoutNode,
}

#[class]
impl PadNode {
    fn construct(_: &mut Context, args: &mut Args) -> TypResult<Template> {
        let all = args.find()?;
        let hor = args.named("horizontal")?;
        let ver = args.named("vertical")?;
        let left = args.named("left")?.or(hor).or(all).unwrap_or_default();
        let top = args.named("top")?.or(ver).or(all).unwrap_or_default();
        let right = args.named("right")?.or(hor).or(all).unwrap_or_default();
        let bottom = args.named("bottom")?.or(ver).or(all).unwrap_or_default();
        let body: LayoutNode = args.expect("body")?;
        let padding = Sides::new(left, top, right, bottom);
        Ok(Template::block(body.padded(padding)))
    }
}

impl Layout for PadNode {
    fn layout(
        &self,
        ctx: &mut Context,
        regions: &Regions,
        styles: StyleChain,
    ) -> TypResult<Vec<Arc<Frame>>> {
        // Layout child into padded regions.
        let pod = regions.map(|size| shrink(size, self.padding));
        let mut frames = self.child.layout(ctx, &pod, styles)?;

        for frame in &mut frames {
            // Apply the padding inversely such that the grown size padded
            // yields the frame's size.
            let padded = grow(frame.size, self.padding);
            let padding = self.padding.resolve(padded);
            let offset = Point::new(padding.left, padding.top);

            // Grow the frame and translate everything in the frame inwards.
            let frame = Arc::make_mut(frame);
            frame.size = padded;
            frame.translate(offset);
        }

        Ok(frames)
    }
}

/// Shrink a size by padding relative to the size itself.
fn shrink(size: Size, padding: Sides<Linear>) -> Size {
    size - padding.resolve(size).sum_by_axis()
}

/// Grow a size by padding relative to the grown size.
/// This is the inverse operation to `shrink()`.
///
/// For the horizontal axis the derivation looks as follows.
/// (Vertical axis is analogous.)
///
/// Let w be the grown target width,
///     s be given width,
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
fn grow(size: Size, padding: Sides<Linear>) -> Size {
    size.zip(padding.sum_by_axis())
        .map(|(s, p)| (s + p.abs).safe_div(1.0 - p.rel.get()))
}
