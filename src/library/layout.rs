//! Layout building blocks.

use crate::length::ScaleLength;
use super::*;

function! {
    /// `box`: Layouts content into a box.
    #[derive(Debug, Clone, PartialEq)]
    pub struct BoxFunc {
        body: SyntaxTree,
        width: Option<ScaleLength>,
        height: Option<ScaleLength>,
    }

    parse(header, body, ctx, f) {
        BoxFunc {
            body: body!(opt: body, ctx, f).unwrap_or(SyntaxTree::new()),
            width: header.args.key.get::<ScaleLength>("width", f),
            height: header.args.key.get::<ScaleLength>("height", f),
        }
    }

    layout(self, ctx, f) {
        ctx.repeat = false;
        ctx.spaces.truncate(1);

        self.width.with(|v| {
            let length = v.raw_scaled(ctx.base.x);
            ctx.base.x = length;
            ctx.spaces[0].size.x = length;
            ctx.spaces[0].expansion.horizontal = true;
        });

        self.height.with(|v| {
            let length = v.raw_scaled(ctx.base.y);
            ctx.base.y = length;
            ctx.spaces[0].size.y = length;
            ctx.spaces[0].expansion.vertical = true;
        });

        let layouted = layout(&self.body, ctx).await;
        let layout = layouted.output.into_iter().next().unwrap();
        f.extend(layouted.feedback);

        vec![Add(layout)]
    }
}

function! {
    /// `align`: Aligns content along the layouting axes.
    #[derive(Debug, Clone, PartialEq)]
    pub struct AlignFunc {
        body: Option<SyntaxTree>,
        aligns: Vec<Spanned<SpecAlign>>,
        h: Option<Spanned<SpecAlign>>,
        v: Option<Spanned<SpecAlign>>,
    }

    parse(header, body, ctx, f) {
        AlignFunc {
            body: body!(opt: body, ctx, f),
            aligns: header.args.pos.all::<Spanned<SpecAlign>>().collect(),
            h: header.args.key.get::<Spanned<SpecAlign>>("horizontal", f),
            v: header.args.key.get::<Spanned<SpecAlign>>("vertical", f),
        }
    }

    layout(self, ctx, f) {
        ctx.base = ctx.spaces[0].size;

        let axes = ctx.axes;
        let all = self.aligns.iter()
            .map(|align| {
                let spec = align.v.axis().unwrap_or(axes.primary.axis());
                (spec, align)
            })
            .chain(self.h.iter().map(|align| (Horizontal, align)))
            .chain(self.v.iter().map(|align| (Vertical, align)));

        let mut had = [false; 2];
        for (axis, align) in all {
            if align.v.axis().map(|a| a != axis).unwrap_or(false) {
                error!(
                    @f, align.span,
                    "invalid alignment {} for {} axis", align.v, axis,
                );
            } else if had[axis as usize] {
                error!(@f, align.span, "duplicate alignment for {} axis", axis);
            } else {
                had[axis as usize] = true;
                let gen_axis = axis.to_generic(ctx.axes);
                let gen_align = align.v.to_generic(ctx.axes);
                *ctx.align.get_mut(gen_axis) = gen_align;
            }
        }

        match &self.body {
            Some(body) => {
                let layouted = layout(body, ctx).await;
                f.extend(layouted.feedback);
                vec![AddMultiple(layouted.output)]
            }
            None => vec![SetAlignment(ctx.align)],
        }
    }
}
