use crate::size::PSize;
use super::*;


function! {
    /// `align`: Aligns content along the layouting axes.
    #[derive(Debug, Clone, PartialEq)]
    pub struct AlignFunc {
        body: Option<SyntaxModel>,
        map: PosAxisMap<AlignmentValue>,
    }

    parse(header, body, ctx, f) {
        AlignFunc {
            body: body!(opt: body, ctx, f),
            map: PosAxisMap::parse::<AxisKey>(&mut f.problems, &mut header.args),
        }
    }

    layout(self, ctx, f) {
        ctx.base = ctx.spaces[0].dimensions;

        let map = self.map.dedup(&mut f.problems, ctx.axes, |alignment| {
            alignment.axis().map(|s| s.to_generic(ctx.axes))
        });

        for &axis in &[Primary, Secondary] {
            if let Some(Spanned { v: alignment, span }) = map.get_spanned(axis) {
                if let Some(generic) = alignment.to_generic(ctx.axes, axis) {
                    *ctx.alignment.get_mut(axis) = generic;
                } else {
                    error!(
                        @f, span,
                        "invalid alignment `{}` for {} axis", alignment, axis,
                    );
                }
            }
        }

        match &self.body {
            Some(body) => {
                let layouted = layout(body, ctx).await;
                f.extend(layouted.feedback);
                vec![AddMultiple(layouted.output)]
            }
            None => vec![SetAlignment(ctx.alignment)],
        }
    }
}

function! {
    /// `direction`: Sets the directions of the layouting axes.
    #[derive(Debug, Clone, PartialEq)]
    pub struct DirectionFunc {
        name_span: Span,
        body: Option<SyntaxModel>,
        map: PosAxisMap<Direction>,
    }

    parse(header, body, ctx, f) {
        DirectionFunc {
            name_span: header.name.span,
            body: body!(opt: body, ctx, f),
            map: PosAxisMap::parse::<AxisKey>(&mut f.problems, &mut header.args),
        }
    }

    layout(self, ctx, f) {
        ctx.base = ctx.spaces[0].dimensions;

        let map = self.map.dedup(&mut f.problems, ctx.axes, |direction| {
            Some(direction.axis().to_generic(ctx.axes))
        });

        let mut axes = ctx.axes;

        map.with(Primary, |&dir| axes.primary = dir);
        map.with(Secondary, |&dir| axes.secondary = dir);

        if axes.primary.axis() == axes.secondary.axis() {
            error!(
                @f, self.name_span,
                "invalid aligned primary and secondary axes: `{}`, `{}`",
                ctx.axes.primary, ctx.axes.secondary,
            );
        } else {
            ctx.axes = axes;
        }

        match &self.body {
            Some(body) => {
                let layouted = layout(body, ctx).await;
                f.extend(layouted.feedback);
                vec![AddMultiple(layouted.output)]
            }
            None => vec![SetAxes(ctx.axes)],
        }
    }
}

function! {
    /// `box`: Layouts content into a box.
    #[derive(Debug, Clone, PartialEq)]
    pub struct BoxFunc {
        body: SyntaxModel,
        extents: AxisMap<PSize>,
        debug: Option<bool>,
    }

    parse(header, body, ctx, f) {
        BoxFunc {
            body: body!(opt: body, ctx, f).unwrap_or(SyntaxModel::new()),
            extents: AxisMap::parse::<ExtentKey>(&mut f.problems, &mut header.args.key),
            debug: header.args.key.get::<bool>(&mut f.problems, "debug"),
        }
    }

    layout(self, ctx, f) {
        ctx.repeat = false;
        ctx.spaces.truncate(1);

        if let Some(debug) = self.debug {
            ctx.debug = debug;
        }

        let map = self.extents.dedup(&mut f.problems, ctx.axes);
        for &axis in &[Horizontal, Vertical] {
            if let Some(psize) = map.get(axis) {
                let size = psize.scaled(ctx.base.get(axis));
                *ctx.base.get_mut(axis) = size;
                *ctx.spaces[0].dimensions.get_mut(axis) = size;
                *ctx.spaces[0].expansion.get_mut(axis) = true;
            }
        }

        let layouted = layout(&self.body, ctx).await;
        let layout = layouted.output.into_iter().next().unwrap();
        f.extend(layouted.feedback);

        vec![Add(layout)]
    }
}
