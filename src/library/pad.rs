//! Surrounding nodes with extra space.

use super::prelude::*;

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
    fn construct(_: &mut Vm, args: &mut Args) -> TypResult<Template> {
        let all = args.find()?;
        let left = args.named("left")?;
        let top = args.named("top")?;
        let right = args.named("right")?;
        let bottom = args.named("bottom")?;
        let body: LayoutNode = args.expect("body")?;
        let padding = Sides::new(
            left.or(all).unwrap_or_default(),
            top.or(all).unwrap_or_default(),
            right.or(all).unwrap_or_default(),
            bottom.or(all).unwrap_or_default(),
        );

        Ok(Template::block(body.padded(padding)))
    }
}

impl Layout for PadNode {
    fn layout(
        &self,
        vm: &mut Vm,
        regions: &Regions,
        styles: StyleChain,
    ) -> TypResult<Vec<Constrained<Arc<Frame>>>> {
        // Layout child into padded regions.
        let pod = regions.map(|size| shrink(size, self.padding));
        let mut frames = self.child.layout(vm, &pod, styles)?;

        for ((current, base), Constrained { item: frame, cts }) in
            regions.iter().zip(&mut frames)
        {
            // Apply the padding inversely such that the grown size padded
            // yields the frame's size.
            let padded = grow(frame.size, self.padding);
            let padding = self.padding.resolve(padded);
            let offset = Point::new(padding.left, padding.top);

            // Grow the frame and translate everything in the frame inwards.
            let frame = Arc::make_mut(frame);
            frame.size = padded;
            frame.translate(offset);

            // Set exact and base constraints if the child had them. Also set
            // base if our padding is relative.
            let is_rel = self.padding.sum_by_axis().map(Linear::is_relative);
            cts.exact = current.filter(cts.exact.map_is_some());
            cts.base = base.filter(is_rel | cts.base.map_is_some());

            // Inflate min and max contraints by the padding.
            for spec in [&mut cts.min, &mut cts.max] {
                spec.as_mut()
                    .zip(padding.sum_by_axis())
                    .map(|(s, p)| s.as_mut().map(|v| *v += p));
            }
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
