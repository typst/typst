use crate::length::ScaleLength;
use super::*;

/// `box`: Layouts its contents into a box.
///
/// # Keyword arguments
/// - `width`: The width of the box (length of relative to parent's width).
/// - `height`: The height of the box (length of relative to parent's height).
pub async fn boxed(mut args: TableValue, mut ctx: LayoutContext<'_>) -> Pass<Value> {
    let mut f = Feedback::new();
    let content = args.take::<SyntaxTree>().unwrap_or(SyntaxTree::new());
    let width = args.take_with_key::<_, ScaleLength>("width", &mut f);
    let height = args.take_with_key::<_, ScaleLength>("height", &mut f);
    args.unexpected(&mut f);

    ctx.spaces.truncate(1);
    ctx.repeat = false;

    width.with(|v| {
        let length = v.raw_scaled(ctx.base.x);
        ctx.base.x = length;
        ctx.spaces[0].size.x = length;
        ctx.spaces[0].expansion.horizontal = true;
    });

    height.with(|v| {
        let length = v.raw_scaled(ctx.base.y);
        ctx.base.y = length;
        ctx.spaces[0].size.y = length;
        ctx.spaces[0].expansion.vertical = true;
    });

    let layouted = layout(&content, ctx).await;
    let layout = layouted.output.into_iter().next().unwrap();
    f.extend(layouted.feedback);
    Pass::commands(vec![Add(layout)], f)
}
