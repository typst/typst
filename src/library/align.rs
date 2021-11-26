use super::prelude::*;

/// `align`: Configure the alignment along the layouting axes.
pub fn align(_: &mut EvalContext, args: &mut Args) -> TypResult<Value> {
    let aligns = args.expect::<Spec<_>>("alignment")?;
    let body = args.expect::<Template>("body")?;
    Ok(Value::Template(Template::from_block(move |style| {
        let mut style = style.clone();
        if let Some(x) = aligns.x {
            style.par_mut().align = x;
        }

        body.pack(&style).aligned(aligns)
    })))
}

/// A node that aligns its child.
#[derive(Debug, Hash)]
pub struct AlignNode {
    /// How to align the node horizontally and vertically.
    pub aligns: Spec<Option<Align>>,
    /// The node to be aligned.
    pub child: PackedNode,
}

impl Layout for AlignNode {
    fn layout(
        &self,
        ctx: &mut LayoutContext,
        regions: &Regions,
    ) -> Vec<Constrained<Rc<Frame>>> {
        // The child only needs to expand along an axis if there's no alignment.
        let mut pod = regions.clone();
        pod.expand &= self.aligns.map_is_none();

        // Layout the child.
        let mut frames = self.child.layout(ctx, &pod);

        for (Constrained { item: frame, cts }, (current, base)) in
            frames.iter_mut().zip(regions.iter())
        {
            // Align in the target size. The target size depends on whether we
            // should expand.
            let target = regions.expand.select(current, frame.size);
            let default = Spec::new(Align::Left, Align::Top);
            let aligns = self.aligns.unwrap_or(default);
            Rc::make_mut(frame).resize(target, aligns);

            // Set constraints.
            cts.expand = regions.expand;
            cts.base = base.filter(cts.base.map_is_some());
            cts.exact = current.filter(regions.expand);
        }

        frames
    }
}
