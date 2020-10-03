use super::*;
use crate::length::ScaleLength;

/// `box`: Layouts its contents into a box.
///
/// # Keyword arguments
/// - `width`: The width of the box (length or relative to parent's width).
/// - `height`: The height of the box (length or relative to parent's height).
pub async fn boxed(
    _: Span,
    mut args: DictValue,
    mut ctx: LayoutContext<'_>,
) -> Pass<Value> {
    let mut f = Feedback::new();

    let content = args.take::<SynTree>().unwrap_or_default();

    ctx.base = ctx.spaces[0].size;
    ctx.spaces.truncate(1);
    ctx.repeat = false;

    if let Some(width) = args.take_key::<ScaleLength>("width", &mut f) {
        let length = width.raw_scaled(ctx.base.width);
        ctx.base.width = length;
        ctx.spaces[0].size.width = length;
        ctx.spaces[0].expansion.horizontal = true;
    }

    if let Some(height) = args.take_key::<ScaleLength>("height", &mut f) {
        let length = height.raw_scaled(ctx.base.height);
        ctx.base.height = length;
        ctx.spaces[0].size.height = length;
        ctx.spaces[0].expansion.vertical = true;
    }

    let layouted = layout(&content, ctx).await;
    let layout = layouted.output.into_iter().next().unwrap();
    f.extend(layouted.feedback);

    args.unexpected(&mut f);
    Pass::commands(vec![Add(layout)], f)
}
