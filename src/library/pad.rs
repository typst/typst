use super::*;
use crate::layout::PadNode;

/// `pad`: Pad content at the sides.
///
/// # Positional parameters
/// - Padding for all sides: `padding`, of type `linear` relative to sides.
/// - Body: of type `template`.
///
/// # Named parameters
/// - Left padding: `left`, of type `linear` relative to parent width.
/// - Right padding: `right`, of type `linear` relative to parent width.
/// - Top padding: `top`, of type `linear` relative to parent height.
/// - Bottom padding: `bottom`, of type `linear` relative to parent height.
///
/// # Return value
/// A template that pads its region and sets the body into it.
pub fn pad(ctx: &mut EvalContext, args: &mut FuncArgs) -> Value {
    let all = args.eat(ctx);
    let left = args.eat_named(ctx, "left");
    let top = args.eat_named(ctx, "top");
    let right = args.eat_named(ctx, "right");
    let bottom = args.eat_named(ctx, "bottom");
    let body = args.eat_expect::<TemplateValue>(ctx, "body").unwrap_or_default();

    let padding = Sides::new(
        left.or(all).unwrap_or_default(),
        top.or(all).unwrap_or_default(),
        right.or(all).unwrap_or_default(),
        bottom.or(all).unwrap_or_default(),
    );

    Value::template("pad", move |ctx| {
        let child = ctx.exec_template_stack(&body).into();
        ctx.push_into_stack(PadNode { padding, child });
    })
}
