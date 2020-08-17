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
pub async fn align(_: Span, mut args: TableValue, mut ctx: LayoutContext<'_>) -> Pass<Value> {
    let mut f = Feedback::new();

    let content = args.take::<SyntaxTree>();

    let h = args.take_key::<Spanned<SpecAlign>>("horizontal", &mut f);
    let v = args.take_key::<Spanned<SpecAlign>>("vertical", &mut f);
    let all = args
        .take_all_num_vals::<Spanned<SpecAlign>>()
        .map(|align| (align.v.axis(), align))
        .chain(h.into_iter().map(|align| (Some(Horizontal), align)))
        .chain(v.into_iter().map(|align| (Some(Vertical), align)));

    let mut had = [false; 2];
    for (axis, align) in all {
        let axis = axis.unwrap_or_else(|| align.v.axis().unwrap_or_else(|| {
            let primary = ctx.axes.primary.axis();
            if !had[primary as usize] {
                primary
            } else {
                ctx.axes.secondary.axis()
            }
        }));

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

    let commands = match content {
        Some(tree) => {
            ctx.base = ctx.spaces[0].size;
            let layouted = layout(&tree, ctx).await;
            f.extend(layouted.feedback);
            vec![AddMultiple(layouted.output)]
        }
        None => vec![SetAlignment(ctx.align)],
    };

    args.unexpected(&mut f);
    Pass::commands(commands, f)
}
