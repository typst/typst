use super::prelude::*;

/// `align`: Configure the alignment along the layouting axes.
pub fn align(_: &mut EvalContext, args: &mut Args) -> TypResult<Value> {
    let Spec { x, y } = parse_aligns(args)?;
    let body = args.expect::<Template>("body")?;
    Ok(Value::Template(Template::from_block(move |style| {
        let mut style = style.clone();
        if let Some(x) = x {
            style.par_mut().align = x;
        }

        body.pack(&style).aligned(x, y)
    })))
}

/// Parse alignment arguments with shorthand.
pub(super) fn parse_aligns(args: &mut Args) -> TypResult<Spec<Option<Align>>> {
    let mut x = args.named("horizontal")?;
    let mut y = args.named("vertical")?;
    for Spanned { v, span } in args.all::<Spanned<Align>>() {
        match v.axis() {
            None | Some(SpecAxis::Horizontal) if x.is_none() => x = Some(v),
            None | Some(SpecAxis::Vertical) if y.is_none() => y = Some(v),
            _ => bail!(span, "unexpected argument"),
        }
    }
    Ok(Spec::new(x, y))
}

/// A node that aligns its child.
#[derive(Debug, Hash)]
pub struct AlignNode {
    /// The node to be aligned.
    pub child: PackedNode,
    /// How to align the node horizontally and vertically.
    pub aligns: Spec<Option<Align>>,
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

        let mut frames = self.child.layout(ctx, &pod);
        for (Constrained { item: frame, cts }, (current, _)) in
            frames.iter_mut().zip(regions.iter())
        {
            let canvas = Size::new(
                if regions.expand.x { current.w } else { frame.size.w },
                if regions.expand.y { current.h } else { frame.size.h },
            );

            let aligns = self.aligns.unwrap_or(Spec::new(Align::Left, Align::Top));
            let offset = Point::new(
                aligns.x.resolve(Length::zero() .. canvas.w - frame.size.w),
                aligns.y.resolve(Length::zero() .. canvas.h - frame.size.h),
            );

            let frame = Rc::make_mut(frame);
            frame.size = canvas;
            frame.baseline += offset.y;
            frame.translate(offset);

            cts.expand = regions.expand;
            cts.exact = current.to_spec().map(Some);
        }

        frames
    }
}
