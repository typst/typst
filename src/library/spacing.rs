use crate::layout::SpacingKind;
use crate::length::ScaleLength;
use super::*;

/// `h`: Add horizontal spacing.
///
/// # Positional arguments
/// - The spacing (length or relative to font size).
pub async fn h(name: Span, args: TableValue, ctx: LayoutContext<'_>) -> Pass<Value> {
    spacing(name, args, ctx, Horizontal)
}

/// `v`: Add vertical spacing.
///
/// # Positional arguments
/// - The spacing (length or relative to font size).
pub async fn v(name: Span, args: TableValue, ctx: LayoutContext<'_>) -> Pass<Value> {
    spacing(name, args, ctx, Vertical)
}

fn spacing(
    name: Span,
    mut args: TableValue,
    ctx: LayoutContext<'_>,
    axis: SpecAxis,
) -> Pass<Value> {
    let mut f = Feedback::new();

    let spacing = args.expect::<ScaleLength>("spacing", name, &mut f);
    let commands = if let Some(spacing) = spacing {
        let axis = axis.to_generic(ctx.axes);
        let spacing = spacing.raw_scaled(ctx.style.text.font_size());
        vec![AddSpacing(spacing, SpacingKind::Hard, axis)]
    } else {
        vec![]
    };

    args.unexpected(&mut f);
    Pass::commands(commands, f)
}
