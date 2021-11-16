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
        PadNode { padding, child: body.pack(style) }
    })))
}

/// A node that adds padding to its child.
#[derive(Debug, Hash)]
pub struct PadNode {
    /// The amount of padding.
    pub padding: Sides<Linear>,
    /// The child node whose sides to pad.
    pub child: PackedNode,
}

impl Layout for PadNode {
    fn layout(
        &self,
        ctx: &mut LayoutContext,
        regions: &Regions,
    ) -> Vec<Constrained<Rc<Frame>>> {
        // Layout child into padded regions.
        let mut frames = self.child.layout(
            ctx,
            &regions.map(|size| size - self.padding.resolve(size).size()),
        );

        for (Constrained { item: frame, cts }, (current, base)) in
            frames.iter_mut().zip(regions.iter())
        {
            fn solve_axis(length: Length, padding: Linear) -> Length {
                (length + padding.abs).safe_div(1.0 - padding.rel.get())
            }

            // Solve for the size `padded` that satisfies (approximately):
            // `padded - padding.resolve(padded).size() == size`
            let padded = Size::new(
                solve_axis(frame.size.w, self.padding.left + self.padding.right),
                solve_axis(frame.size.h, self.padding.top + self.padding.bottom),
            );

            let padding = self.padding.resolve(padded);
            let origin = Point::new(padding.left, padding.top);

            // Create a new larger frame and place the child's frame inside it.
            let empty = Frame::new(padded, frame.baseline + origin.y);
            let prev = std::mem::replace(frame, Rc::new(empty));
            let new = Rc::make_mut(frame);
            new.push_frame(origin, prev);

            // Inflate min and max contraints by the padding.
            for spec in [&mut cts.min, &mut cts.max] {
                if let Some(x) = spec.x.as_mut() {
                    *x += padding.size().w;
                }
                if let Some(y) = spec.y.as_mut() {
                    *y += padding.size().h;
                }
            }

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
        }

        frames
    }
}
