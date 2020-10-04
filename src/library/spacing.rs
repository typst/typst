use super::*;
use crate::geom::Linear;
use crate::layout::SpacingKind;

/// `h`: Add horizontal spacing.
///
/// # Positional arguments
/// - The spacing (length or relative to font size).
pub async fn h(args: ValueDict, ctx: &mut LayoutContext) -> Value {
    spacing(args, ctx, SpecAxis::Horizontal)
}

/// `v`: Add vertical spacing.
///
/// # Positional arguments
/// - The spacing (length or relative to font size).
pub async fn v(args: ValueDict, ctx: &mut LayoutContext) -> Value {
    spacing(args, ctx, SpecAxis::Vertical)
}

fn spacing(mut args: ValueDict, ctx: &mut LayoutContext, axis: SpecAxis) -> Value {
    let spacing = args.expect::<Linear>("spacing", Span::ZERO, &mut ctx.f);
    args.unexpected(&mut ctx.f);
    Value::Commands(if let Some(spacing) = spacing {
        let axis = axis.to_gen(ctx.state.sys);
        let spacing = spacing.eval(ctx.state.text.font_size());
        vec![AddSpacing(spacing, SpacingKind::Hard, axis)]
    } else {
        vec![]
    })
}
