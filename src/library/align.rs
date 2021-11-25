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
        // Along axes with specified alignment, the child doesn't need to expand.
        let mut pod = regions.clone();
        pod.expand.x &= self.aligns.x.is_none();
        pod.expand.y &= self.aligns.y.is_none();

        // Layout the child.
        let mut frames = self.child.layout(ctx, &pod);

        for (Constrained { item: frame, cts }, (current, base)) in
            frames.iter_mut().zip(regions.iter())
        {
            // The possibly larger size in which we align the frame.
            let new = Size::new(
                if regions.expand.x { current.w } else { frame.size.w },
                if regions.expand.y { current.h } else { frame.size.h },
            );

            let aligns = self.aligns.unwrap_or(Spec::new(Align::Left, Align::Top));
            Rc::make_mut(frame).resize(new, aligns);

            // Set constraints.
            cts.expand = regions.expand;
            cts.base.x.and_set(Some(base.w));
            cts.base.y.and_set(Some(base.h));
            cts.exact = Spec::new(
                regions.expand.x.then(|| current.w),
                regions.expand.y.then(|| current.h),
            );
        }

        frames
    }
}
