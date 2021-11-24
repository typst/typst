use super::prelude::*;

/// `box`: Size content and place it into a paragraph.
pub fn box_(_: &mut EvalContext, args: &mut Args) -> TypResult<Value> {
    let width = args.named("width")?;
    let height = args.named("height")?;
    let body: Template = args.find().unwrap_or_default();
    Ok(Value::Template(Template::from_inline(move |style| {
        body.pack(style).sized(Spec::new(width, height))
    })))
}

/// `block`: Size content and place it into the flow.
pub fn block(_: &mut EvalContext, args: &mut Args) -> TypResult<Value> {
    let width = args.named("width")?;
    let height = args.named("height")?;
    let body: Template = args.find().unwrap_or_default();
    Ok(Value::Template(Template::from_block(move |style| {
        body.pack(style).sized(Spec::new(width, height))
    })))
}

/// A node that sizes its child.
#[derive(Debug, Hash)]
pub struct SizedNode {
    /// How to size the node horizontally and vertically.
    pub sizing: Spec<Option<Linear>>,
    /// The node to be sized.
    pub child: PackedNode,
}

impl Layout for SizedNode {
    fn layout(
        &self,
        ctx: &mut LayoutContext,
        regions: &Regions,
    ) -> Vec<Constrained<Rc<Frame>>> {
        // Resolve width and height relative to the region's base.
        let width = self.sizing.x.map(|w| w.resolve(regions.base.w));
        let height = self.sizing.y.map(|h| h.resolve(regions.base.h));

        // Generate constraints.
        let mut cts = Constraints::new(regions.expand);
        cts.set_base_if_linear(regions.base, self.sizing);

        // Set tight exact and base constraints if the child is
        // automatically sized since we don't know what the child might do.
        if self.sizing.x.is_none() {
            cts.exact.x = Some(regions.current.w);
            cts.base.x = Some(regions.base.w);
        }

        // Same here.
        if self.sizing.y.is_none() {
            cts.exact.y = Some(regions.current.h);
            cts.base.y = Some(regions.base.h);
        }

        // The "pod" is the region into which the child will be layouted.
        let pod = {
            let size = Size::new(
                width.unwrap_or(regions.current.w),
                height.unwrap_or(regions.current.h),
            );

            let base = Size::new(
                if width.is_some() { size.w } else { regions.base.w },
                if height.is_some() { size.h } else { regions.base.h },
            );

            let expand = Spec::new(
                width.is_some() || regions.expand.x,
                height.is_some() || regions.expand.y,
            );

            // TODO: Allow multiple regions if only width is set.
            Regions::one(size, base, expand)
        };

        let mut frames = self.child.layout(ctx, &pod);
        frames[0].cts = cts;
        frames
    }
}
