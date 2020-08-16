use crate::layout::SpacingKind;
use crate::length::ScaleLength;
use super::*;

/// `h`: Add horizontal spacing.
///
/// # Positional arguments
/// - The spacing (length or relative to font size).
pub async fn h(args: TableValue, ctx: LayoutContext<'_>) -> Pass<Value> {
    spacing(args, ctx, Horizontal).await
}

/// `v`: Add vertical spacing.
///
/// # Positional arguments
/// - The spacing (length or relative to font size).
pub async fn v(args: TableValue, ctx: LayoutContext<'_>) -> Pass<Value> {
    spacing(args, ctx, Vertical).await
}

async fn spacing(
    mut args: TableValue,
    ctx: LayoutContext<'_>,
    axis: SpecAxis,
) -> Pass<Value> {
    let mut f = Feedback::new();
    let spacing = args.expect::<ScaleLength>(&mut f).map(|s| (axis, s));
    args.unexpected(&mut f);

    Pass::commands(if let Some((axis, spacing)) = spacing {
        let axis = axis.to_generic(ctx.axes);
        let spacing = spacing.raw_scaled(ctx.style.text.font_size());
        vec![AddSpacing(spacing, SpacingKind::Hard, axis)]
    } else {
        vec![]
    }, f)
}
