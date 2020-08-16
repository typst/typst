use super::*;

/// `align`: Align content along the layouting axes.
///
/// # Positional arguments
/// - At most two of `left`, `right`, `top`, `bottom`, `center`.
///
/// # Keyword arguments
/// - `horizontal`: Any of `left`, `right` or `center`.
/// - `vertical`: Any of `top`, `bottom` or `center`.
///
/// There may not be two alignment specifications for the same axis.
pub async fn align(mut args: TableValue, mut ctx: LayoutContext<'_>) -> Pass<Value> {
    let mut f = Feedback::new();

    let content = args.take::<SyntaxTree>();
    let aligns: Vec<_> = args.take_all_num_vals::<Spanned<SpecAlign>>().collect();
    let h = args.take_with_key::<_, Spanned<SpecAlign>>("horizontal", &mut f);
    let v = args.take_with_key::<_, Spanned<SpecAlign>>("vertical", &mut f);
    args.unexpected(&mut f);

    ctx.base = ctx.spaces[0].size;

    let axes = ctx.axes;
    let all = aligns.iter()
        .map(|align| {
            let spec = align.v.axis().unwrap_or(axes.primary.axis());
            (spec, align)
        })
        .chain(h.iter().map(|align| (Horizontal, align)))
        .chain(v.iter().map(|align| (Vertical, align)));

    let mut had = [false; 2];
    for (axis, align) in all {
        if align.v.axis().map(|a| a != axis).unwrap_or(false) {
            error!(
                @f, align.span,
                "invalid alignment {} for {} axis", align.v, axis,
            );
        } else if had[axis as usize] {
            error!(@f, align.span, "duplicate alignment for {} axis", axis);
        } else {
            had[axis as usize] = true;
            let gen_axis = axis.to_generic(ctx.axes);
            let gen_align = align.v.to_generic(ctx.axes);
            *ctx.align.get_mut(gen_axis) = gen_align;
        }
    }

    Pass::commands(match content {
        Some(tree) => {
            let layouted = layout(&tree, ctx).await;
            f.extend(layouted.feedback);
            vec![AddMultiple(layouted.output)]
        }
        None => vec![SetAlignment(ctx.align)],
    }, f)
}
