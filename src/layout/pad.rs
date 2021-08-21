use super::*;

/// A node that adds padding to its child.
#[cfg_attr(feature = "layout-cache", derive(Hash))]
pub struct PadNode {
    /// The amount of padding.
    pub padding: Sides<Linear>,
    /// The child node whose sides to pad.
    pub child: LayoutNode,
}

impl Layout for PadNode {
    fn layout(
        &self,
        ctx: &mut LayoutContext,
        regions: &Regions,
    ) -> Vec<Constrained<Rc<Frame>>> {
        let mut frames = self.child.layout(
            ctx,
            &regions.map(|size| size - self.padding.resolve(size).size()),
        );

        for (Constrained { item: frame, constraints }, (current, base)) in
            frames.iter_mut().zip(regions.iter())
        {
            fn solve_axis(length: Length, padding: Linear) -> Length {
                (length + padding.abs) / (1.0 - padding.rel.get())
            }

            // Solve for the size `padded` that satisfies (approximately):
            // `padded - padding.resolve(padded).size() == size`
            let padded = Size::new(
                solve_axis(frame.size.w, self.padding.left + self.padding.right),
                solve_axis(frame.size.h, self.padding.top + self.padding.bottom),
            );

            let padding = self.padding.resolve(padded);
            let origin = Point::new(padding.left, padding.top);

            // Inflate min and max contraints by the padding.
            for spec in [&mut constraints.min, &mut constraints.max] {
                if let Some(x) = spec.x.as_mut() {
                    *x += padding.size().w;
                }
                if let Some(y) = spec.y.as_mut() {
                    *y += padding.size().h;
                }
            }

            // Set exact and base constraints if the child had them.
            constraints.exact.x.and_set(Some(current.w));
            constraints.exact.y.and_set(Some(current.h));
            constraints.base.x.and_set(Some(base.w));
            constraints.base.y.and_set(Some(base.h));

            // Also set base constraints if the padding is relative.
            if self.padding.left.is_relative() || self.padding.right.is_relative() {
                constraints.base.x = Some(base.w);
            }

            if self.padding.top.is_relative() || self.padding.bottom.is_relative() {
                constraints.base.y = Some(base.h);
            }

            // Create a new larger frame and place the child's frame inside it.
            let empty = Frame::new(padded, frame.baseline + origin.y);
            let prev = std::mem::replace(frame, Rc::new(empty));
            let new = Rc::make_mut(frame);
            new.push_frame(origin, prev);
        }

        frames
    }
}

impl From<PadNode> for LayoutNode {
    fn from(pad: PadNode) -> Self {
        Self::new(pad)
    }
}
