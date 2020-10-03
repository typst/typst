use super::*;
use crate::geom::Linear;

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

    if let Some(width) = args.take_key::<Linear>("width", &mut f) {
        let abs = width.eval(ctx.base.width);
        ctx.base.width = abs;
        ctx.spaces[0].size.width = abs;
        ctx.spaces[0].expansion.horizontal = true;
    }

    if let Some(height) = args.take_key::<Linear>("height", &mut f) {
        let abs = height.eval(ctx.base.height);
        ctx.base.height = abs;
        ctx.spaces[0].size.height = abs;
        ctx.spaces[0].expansion.vertical = true;
    }

    let layouted = layout(&content, ctx).await;
    let layout = layouted.output.into_iter().next().unwrap();
    f.extend(layouted.feedback);

    args.unexpected(&mut f);
    Pass::commands(vec![Add(layout)], f)
}
