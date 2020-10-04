use super::*;
use crate::geom::Linear;

/// `box`: Layouts its contents into a box.
///
/// # Keyword arguments
/// - `width`: The width of the box (length or relative to parent's width).
/// - `height`: The height of the box (length or relative to parent's height).
pub async fn boxed(mut args: DictValue, ctx: &mut LayoutContext) -> Value {
    let content = args.take::<SynTree>().unwrap_or_default();

    let constraints = &mut ctx.constraints;
    constraints.base = constraints.spaces[0].size;
    constraints.spaces.truncate(1);
    constraints.repeat = false;

    if let Some(width) = args.take_key::<Linear>("width", &mut ctx.f) {
        let abs = width.eval(constraints.base.width);
        constraints.base.width = abs;
        constraints.spaces[0].size.width = abs;
        constraints.spaces[0].expansion.horizontal = true;
    }

    if let Some(height) = args.take_key::<Linear>("height", &mut ctx.f) {
        let abs = height.eval(constraints.base.height);
        constraints.base.height = abs;
        constraints.spaces[0].size.height = abs;
        constraints.spaces[0].expansion.vertical = true;
    }

    args.unexpected(&mut ctx.f);

    let layouted = layout_tree(&content, ctx).await;
    let layout = layouted.into_iter().next().unwrap();

    Value::Commands(vec![Add(layout)])
}
