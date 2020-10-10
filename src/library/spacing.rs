use crate::geom::Linear;
use crate::layout::{Softness, Spacing};
use crate::prelude::*;

/// `h`: Add horizontal spacing.
///
/// # Positional arguments
/// - The spacing (length or relative to font size).
pub fn h(args: Args, ctx: &mut EvalContext) -> Value {
    spacing(args, ctx, SpecAxis::Horizontal)
}

/// `v`: Add vertical spacing.
///
/// # Positional arguments
/// - The spacing (length or relative to font size).
pub fn v(args: Args, ctx: &mut EvalContext) -> Value {
    spacing(args, ctx, SpecAxis::Vertical)
}

/// Apply spacing along a specific axis.
fn spacing(mut args: Args, ctx: &mut EvalContext, axis: SpecAxis) -> Value {
    let spacing = args.need::<_, Linear>(ctx, 0, "spacing");
    args.done(ctx);

    if let Some(linear) = spacing {
        let amount = linear.eval(ctx.state.font.font_size());
        let spacing = Spacing { amount, softness: Softness::Hard };
        if ctx.state.dirs.main.axis() == axis {
            ctx.end_par_group();
            ctx.push(spacing);
            ctx.start_par_group();
        } else {
            ctx.push(spacing);
        }
    }

    Value::None
}
