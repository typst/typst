use super::*;

/// `align`: Align content along the layouting axes.
///
/// # Positional arguments
/// - At most two of `left`, `right`, `top`, `bottom`, `center`.
///
/// When `center` is used as a positional argument, it is automatically inferred
/// which axis it should apply to depending on further arguments, defaulting
/// to the axis, text is set along.
///
/// # Keyword arguments
/// - `horizontal`: Any of `left`, `right` or `center`.
/// - `vertical`: Any of `top`, `bottom` or `center`.
///
/// There may not be two alignment specifications for the same axis.
pub async fn align(_: Span, mut args: DictValue, ctx: LayoutContext<'_>) -> Pass<Value> {
    let mut f = Feedback::new();

    let content = args.take::<SynTree>();
    let h = args.take_key::<Spanned<SpecAlign>>("horizontal", &mut f);
    let v = args.take_key::<Spanned<SpecAlign>>("vertical", &mut f);
    let all = args
        .take_all_num_vals::<Spanned<SpecAlign>>()
        .map(|align| (align.v.axis(), align))
        .chain(h.into_iter().map(|align| (Some(SpecAxis::Horizontal), align)))
        .chain(v.into_iter().map(|align| (Some(SpecAxis::Vertical), align)));

    let mut aligns = ctx.align;
    let mut had = [false; 2];
    let mut deferred_center = false;

    for (axis, align) in all {
        // Check whether we know which axis this alignment belongs to. We don't
        // if the alignment is `center` for a positional argument. Then we set
        // `deferred_center` to true and handle the situation once we know more.
        if let Some(axis) = axis {
            if align.v.axis().map(|a| a != axis).unwrap_or(false) {
                error!(
                    @f, align.span,
                    "invalid alignment {} for {} axis", align.v, axis,
                );
            } else if had[axis as usize] {
                error!(@f, align.span, "duplicate alignment for {} axis", axis);
            } else {
                let gen_align = align.v.to_gen(ctx.sys);
                *aligns.get_mut(axis.to_gen(ctx.sys)) = gen_align;
                had[axis as usize] = true;
            }
        } else {
            if had == [true, true] {
                error!(@f, align.span, "duplicate alignment");
            } else if deferred_center {
                // We have two unflushed centers, meaning we know that both axes
                // are to be centered.
                had = [true, true];
                aligns = LayoutAlign::new(GenAlign::Center, GenAlign::Center);
            } else {
                deferred_center = true;
            }
        }

        // Flush a deferred center alignment if we know have had at least one
        // known alignment.
        if deferred_center && had != [false, false] {
            let axis = if !had[SpecAxis::Horizontal as usize] {
                SpecAxis::Horizontal
            } else {
                SpecAxis::Vertical
            };

            *aligns.get_mut(axis.to_gen(ctx.sys)) = GenAlign::Center;

            had[axis as usize] = true;
            deferred_center = false;
        }
    }

    // If center has not been flushed by known, it is the only argument and then
    // we default to applying it to the primary axis.
    if deferred_center {
        aligns.primary = GenAlign::Center;
    }

    let commands = match content {
        Some(tree) => vec![
            SetAlignment(aligns),
            LayoutSyntaxTree(tree),
            SetAlignment(ctx.align),
        ],
        None => vec![SetAlignment(aligns)],
    };

    args.unexpected(&mut f);
    Pass::commands(commands, f)
}
