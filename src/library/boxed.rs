use crate::geom::Linear;
use crate::prelude::*;

/// `box`: Layouts its contents into a box.
///
/// # Keyword arguments
/// - `width`: The width of the box (length or relative to parent's width).
/// - `height`: The height of the box (length or relative to parent's height).
pub async fn boxed(mut args: Args, ctx: &mut LayoutContext) -> Value {
    let body = args.find::<SynTree>().unwrap_or_default();
    let width = args.get::<_, Linear>(ctx, "width");
    let height = args.get::<_, Linear>(ctx, "height");
    args.done(ctx);

    let align = ctx.state.align;
    let constraints = &mut ctx.constraints;
    constraints.base = constraints.spaces[0].size;
    constraints.spaces.truncate(1);
    constraints.repeat = false;

    if let Some(width) = width {
        let abs = width.eval(constraints.base.width);
        constraints.base.width = abs;
        constraints.spaces[0].size.width = abs;
        constraints.spaces[0].expansion.horizontal = true;
    }

    if let Some(height) = height {
        let abs = height.eval(constraints.base.height);
        constraints.base.height = abs;
        constraints.spaces[0].size.height = abs;
        constraints.spaces[0].expansion.vertical = true;
    }

    let layouted = layout_tree(&body, ctx).await;
    let layout = layouted.into_iter().next().unwrap();

    Value::Commands(vec![Add(layout, align)])
}
