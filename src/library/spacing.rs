use super::*;

/// `h`: Horizontal spacing.
///
/// # Positional parameters
/// - Amount of spacing: of type `linear` relative to current font size.
///
/// # Return value
/// A template that inserts horizontal spacing.
pub fn h(ctx: &mut EvalContext, args: &mut FuncArgs) -> Value {
    spacing_impl("h", ctx, args, GenAxis::Cross)
}

/// `v`: Vertical spacing.
///
/// # Positional parameters
/// - Amount of spacing: of type `linear` relative to current font size.
///
/// # Return value
/// A template that inserts vertical spacing.
pub fn v(ctx: &mut EvalContext, args: &mut FuncArgs) -> Value {
    spacing_impl("v", ctx, args, GenAxis::Main)
}

fn spacing_impl(
    name: &str,
    ctx: &mut EvalContext,
    args: &mut FuncArgs,
    axis: GenAxis,
) -> Value {
    let spacing: Option<Linear> = args.expect(ctx, "spacing");
    Value::template(name, move |ctx| {
        if let Some(linear) = spacing {
            // TODO: Should this really always be font-size relative?
            let amount = linear.resolve(ctx.state.font.size);
            ctx.push_spacing(axis, amount);
        }
    })
}
