use super::*;
use crate::geom::Linear;
use crate::layout::SpacingKind;

/// `h`: Add horizontal spacing.
///
/// # Positional arguments
/// - The spacing (length or relative to font size).
pub async fn h(args: Args, ctx: &mut LayoutContext) -> Value {
    spacing(args, ctx, SpecAxis::Horizontal)
}

/// `v`: Add vertical spacing.
///
/// # Positional arguments
/// - The spacing (length or relative to font size).
pub async fn v(args: Args, ctx: &mut LayoutContext) -> Value {
    spacing(args, ctx, SpecAxis::Vertical)
}

fn spacing(mut args: Args, ctx: &mut LayoutContext, axis: SpecAxis) -> Value {
    let spacing = args.get::<_, Linear>(ctx, 0);
    args.done(ctx);

    Value::Commands(if let Some(spacing) = spacing {
        let spacing = spacing.eval(ctx.state.text.font_size());
        let axis = axis.to_gen(ctx.state.sys);
        vec![AddSpacing(spacing, SpacingKind::Hard, axis)]
    } else {
        vec![]
    })
}
