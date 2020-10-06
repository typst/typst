use crate::geom::Linear;
use crate::layout::SpacingKind;
use crate::prelude::*;

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
    let spacing = args.need::<_, Linear>(ctx, 0, "spacing");
    args.done(ctx);

    Value::Commands(if let Some(spacing) = spacing {
        let spacing = spacing.eval(ctx.state.text.font_size());
        let axis = axis.switch(ctx.state.dirs);
        vec![AddSpacing(spacing, SpacingKind::Hard, axis)]
    } else {
        vec![]
    })
}
