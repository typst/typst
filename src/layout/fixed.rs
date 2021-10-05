use decorum::N64;

use super::*;

/// A node that can fix its child's width and height.
#[derive(Debug)]
#[cfg_attr(feature = "layout-cache", derive(Hash))]
pub struct FixedNode {
    /// The fixed width, if any.
    pub width: Option<Linear>,
    /// The fixed height, if any.
    pub height: Option<Linear>,
    /// The fixed aspect ratio between width and height.
    ///
    /// The resulting frame will satisfy `width = aspect * height`.
    pub aspect: Option<N64>,
    /// The child node whose size to fix.
    pub child: LayoutNode,
}

impl Layout for FixedNode {
    fn layout(
        &self,
        ctx: &mut LayoutContext,
        regions: &Regions,
    ) -> Vec<Constrained<Rc<Frame>>> {
        // Fill in width or height if aspect ratio and the other is given.
        let aspect = self.aspect.map(N64::into_inner);
        let width = self.width.or(self.height.zip(aspect).map(|(h, a)| a * h));
        let height = self.height.or(self.width.zip(aspect).map(|(w, a)| w / a));

        // Resolve the linears based on the current width and height.
        let mut child_size = Size::new(
            width.map_or(regions.current.w, |w| w.resolve(regions.base.w)),
            height.map_or(regions.current.h, |h| h.resolve(regions.base.h)),
        );

        // If width or height aren't set for an axis, the base should be
        // inherited from the parent for that axis.
        let child_base = Size::new(
            if width.is_some() { child_size.w } else { regions.base.w },
            if height.is_some() { child_size.h } else { regions.base.h },
        );

        // Prepare constraints.
        let mut constraints = Constraints::new(regions.expand);
        constraints.set_base_if_linear(regions.base, Spec::new(width, height));

        // If the size for one axis isn't specified, the `current` size along
        // that axis needs to remain the same for the result to be reusable.
        if width.is_none() {
            constraints.exact.x = Some(regions.current.w);
        }
        if height.is_none() {
            constraints.exact.y = Some(regions.current.h);
        }

        // Apply the aspect ratio.
        if let Some(aspect) = aspect {
            let width = child_size.w.min(aspect * child_size.h);
            child_size = Size::new(width, width / aspect);
            constraints.exact = regions.current.to_spec().map(Some);
            constraints.min = Spec::splat(None);
            constraints.max = Spec::splat(None);
        }

        // If width or height are fixed, the child should fill the available
        // space along that axis.
        let child_expand = Spec::new(width.is_some(), height.is_some());

        // Layout the child.
        let mut regions = Regions::one(child_size, child_base, child_expand);
        let mut frames = self.child.layout(ctx, &regions);

        // If we have an aspect ratio and the child is content-sized, we need to
        // relayout with expansion.
        if let Some(aspect) = aspect {
            if width.is_none() && height.is_none() {
                let needed = frames[0].item.size.cap(child_size);
                let width = needed.w.max(aspect * needed.h);
                regions.current = Size::new(width, width / aspect);
                regions.expand = Spec::splat(true);
                frames = self.child.layout(ctx, &regions);
            }
        }

        // Overwrite the child's constraints with ours.
        assert_eq!(frames.len(), 1);
        frames[0].constraints = constraints;

        frames
    }
}

impl From<FixedNode> for LayoutNode {
    fn from(fixed: FixedNode) -> Self {
        Self::new(fixed)
    }
}
