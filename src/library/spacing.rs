use super::*;
use crate::geom::Linear;
use crate::layout::SpacingKind;

/// `h`: Add horizontal spacing.
///
/// # Positional arguments
/// - The spacing (length or relative to font size).
pub async fn h(name: Span, args: DictValue, ctx: LayoutContext<'_>) -> Pass<Value> {
    spacing(name, args, ctx, SpecAxis::Horizontal)
}

/// `v`: Add vertical spacing.
///
/// # Positional arguments
/// - The spacing (length or relative to font size).
pub async fn v(name: Span, args: DictValue, ctx: LayoutContext<'_>) -> Pass<Value> {
    spacing(name, args, ctx, SpecAxis::Vertical)
}

fn spacing(
    name: Span,
    mut args: DictValue,
    ctx: LayoutContext<'_>,
    axis: SpecAxis,
) -> Pass<Value> {
    let mut f = Feedback::new();

    let spacing = args.expect::<Linear>("spacing", name, &mut f);
    let commands = if let Some(spacing) = spacing {
        let axis = axis.to_gen(ctx.sys);
        let spacing = spacing.eval(ctx.style.text.font_size());
        vec![AddSpacing(spacing, SpacingKind::Hard, axis)]
    } else {
        vec![]
    };

    args.unexpected(&mut f);
    Pass::commands(commands, f)
}
