use crate::layout::SpacingKind;
use crate::length::ScaleLength;
use super::*;

function! {
    /// `h` and `v`: Add horizontal or vertical spacing.
    #[derive(Debug, Clone, PartialEq)]
    pub struct SpacingFunc {
        spacing: Option<(SpecAxis, ScaleLength)>,
    }

    type Meta = SpecAxis;

    parse(header, body, state, f, meta) {
        expect_no_body(body, f);
        Self {
            spacing: header.args.pos.expect::<ScaleLength>(f)
                .map(|s| (meta, s))
                .or_missing(header.name.span, "spacing", f),
        }
    }

    layout(self, ctx, f) {
        if let Some((axis, spacing)) = self.spacing {
            let axis = axis.to_generic(ctx.axes);
            let spacing = spacing.raw_scaled(ctx.style.text.font_size());
            vec![AddSpacing(spacing, SpacingKind::Hard, axis)]
        } else {
            vec![]
        }
    }
}
