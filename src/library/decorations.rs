use crate::exec::{FontState, LineState};
use crate::layout::Fill;

use super::*;

/// `strike`: Enable striken-through text.
///
/// # Named parameters
/// - Color: `color`, of type `color`.
/// - Baseline offset: `position`, of type `linear`.
/// - Strength: `strength`, of type `linear`.
/// - Extent that is applied on either end of the line: `extent`, of type
///   `linear`.
///
/// # Return value
/// A template that enables striken-through text. The effect is scoped to the
/// body if present.
pub fn strike(ctx: &mut EvalContext, args: &mut FuncArgs) -> Value {
    line_impl("strike", ctx, args, |font| &mut font.strikethrough)
}

/// `underline`: Enable underlined text.
///
/// # Named parameters
/// - Color: `color`, of type `color`.
/// - Baseline offset: `position`, of type `linear`.
/// - Strength: `strength`, of type `linear`.
/// - Extent that is applied on either end of the line: `extent`, of type
///   `linear`.
///
/// # Return value
/// A template that enables underlined text. The effect is scoped to the body if
/// present.
pub fn underline(ctx: &mut EvalContext, args: &mut FuncArgs) -> Value {
    line_impl("underline", ctx, args, |font| &mut font.underline)
}

/// `overline`: Add an overline above text.
///
/// # Named parameters
/// - Color: `color`, of type `color`.
/// - Baseline offset: `position`, of type `linear`.
/// - Strength: `strength`, of type `linear`.
/// - Extent that is applied on either end of the line: `extent`, of type
///   `linear`.
///
/// # Return value
/// A template that adds an overline above text. The effect is scoped to the
/// body if present.
pub fn overline(ctx: &mut EvalContext, args: &mut FuncArgs) -> Value {
    line_impl("overline", ctx, args, |font| &mut font.overline)
}

fn line_impl(
    name: &str,
    ctx: &mut EvalContext,
    args: &mut FuncArgs,
    substate: impl Fn(&mut FontState) -> &mut Option<LineState> + 'static,
) -> Value {
    let color = args.eat_named(ctx, "color");
    let position = args.eat_named(ctx, "position");
    let strength = args.eat_named::<Linear>(ctx, "strength");
    let extent = args.eat_named(ctx, "extent").unwrap_or_default();
    let body = args.eat::<TemplateValue>(ctx);

    // Suppress any existing strikethrough if strength is explicitly zero.
    let state = strength.map_or(true, |s| !s.is_zero()).then(|| LineState {
        fill: color.map(Fill::Color),
        strength,
        position,
        extent,
    });

    Value::template(name, move |ctx| {
        let snapshot = ctx.state.clone();

        *substate(&mut ctx.state.font) = state;

        if let Some(body) = &body {
            body.exec(ctx);
            ctx.state = snapshot;
        }
    })
}
