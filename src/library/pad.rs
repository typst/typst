use super::prelude::*;

/// `pad`: Pad content at the sides.
pub fn pad(_: &mut EvalContext, args: &mut Args) -> TypResult<Value> {
    let all = args.find();
    let left = args.named("left")?;
    let top = args.named("top")?;
    let right = args.named("right")?;
    let bottom = args.named("bottom")?;
    let body: Template = args.expect("body")?;
    let padding = Sides::new(
        left.or(all).unwrap_or_default(),
        top.or(all).unwrap_or_default(),
        right.or(all).unwrap_or_default(),
        bottom.or(all).unwrap_or_default(),
    );

    Ok(Value::Template(Template::from_inline(move |style| {
        body.pack(style).padded(padding)
    })))
}

/// A node that adds padding to its child.
#[derive(Debug, Hash)]
pub struct PadNode {
    /// The child node whose sides to pad.
    pub child: PackedNode,
    /// The amount of padding.
    pub padding: Sides<Linear>,
}

impl Layout for PadNode {
    fn layout(
        &self,
        ctx: &mut LayoutContext,
        regions: &Regions,
    ) -> Vec<Constrained<Rc<Frame>>> {
        // Layout child into padded regions.
        let pod = regions.map(|size| shrink(size, self.padding));
        let mut frames = self.child.layout(ctx, &pod);

        for (Constrained { item: frame, cts }, (current, base)) in
            frames.iter_mut().zip(regions.iter())
        {
            // Apply the padding inversely such that the grown size padded
            // yields the frame's size.
            let padded = grow(frame.size, self.padding);
            let padding = self.padding.resolve(padded);
            let offset = Point::new(padding.left, padding.top);

            // Grow the frame and translate everything in the frame inwards.
            let frame = Rc::make_mut(frame);
            frame.size = padded;
            frame.baseline += offset.y;
            frame.translate(offset);

            // Set exact and base constraints if the child had them.
            cts.exact.x.and_set(Some(current.w));
            cts.exact.y.and_set(Some(current.h));
            cts.base.x.and_set(Some(base.w));
            cts.base.y.and_set(Some(base.h));

            // Also set base constraints if the padding is relative.
            if self.padding.left.is_relative() || self.padding.right.is_relative() {
                cts.base.x = Some(base.w);
            }

            if self.padding.top.is_relative() || self.padding.bottom.is_relative() {
                cts.base.y = Some(base.h);
            }

            // Inflate min and max contraints by the padding.
            for spec in [&mut cts.min, &mut cts.max] {
                if let Some(x) = spec.x.as_mut() {
                    *x += padding.size().w;
                }
                if let Some(y) = spec.y.as_mut() {
                    *y += padding.size().h;
                }
            }
        }

        frames
    }
}

/// Shrink a size by padding relative to the size itself.
fn shrink(size: Size, padding: Sides<Linear>) -> Size {
    size - padding.resolve(size).size()
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
    fn solve_axis(length: Length, padding: Linear) -> Length {
        (length + padding.abs).safe_div(1.0 - padding.rel.get())
    }

    Size::new(
        solve_axis(size.w, padding.left + padding.right),
        solve_axis(size.h, padding.top + padding.bottom),
    )
}
