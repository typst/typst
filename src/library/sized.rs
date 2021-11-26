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
        let is_auto = self.sizing.map_is_none();
        let is_rel = self.sizing.map(|s| s.map_or(false, Linear::is_relative));

        // The "pod" is the region into which the child will be layouted.
        let pod = {
            // Resolve the sizing to a concrete size.
            let size = self
                .sizing
                .zip(regions.base)
                .map(|(s, b)| s.map(|v| v.resolve(b)))
                .unwrap_or(regions.current);

            // Select the appropriate base and expansion for the child depending
            // on whether it is automatically or linearly sized.
            let base = is_auto.select(regions.base, size);
            let expand = regions.expand | !is_auto;

            Regions::one(size, base, expand)
        };

        let mut frames = self.child.layout(ctx, &pod);

        // Set base & exact constraints if the child is automatically sized
        // since we don't know what the child might do. Also set base if our
        // sizing is relative.
        let frame = &mut frames[0];
        frame.cts = Constraints::new(regions.expand);
        frame.cts.exact = regions.current.filter(is_auto);
        frame.cts.base = regions.base.filter(is_auto | is_rel);

        frames
    }
}
