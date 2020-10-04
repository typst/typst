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
pub async fn align(mut args: Args, ctx: &mut LayoutContext) -> Value {
    let body = args.find::<SynTree>();

    let h = args.get::<_, Spanned<SpecAlign>>(ctx, "horizontal");
    let v = args.get::<_, Spanned<SpecAlign>>(ctx, "vertical");
    let pos = args.find_all::<Spanned<SpecAlign>>();

    let iter = pos
        .map(|align| (align.v.axis(), align))
        .chain(h.into_iter().map(|align| (Some(SpecAxis::Horizontal), align)))
        .chain(v.into_iter().map(|align| (Some(SpecAxis::Vertical), align)));

    let aligns = parse_aligns(ctx, iter);

    args.done(ctx);
    Value::Commands(match body {
        Some(tree) => vec![
            SetAlignment(aligns),
            LayoutSyntaxTree(tree),
            SetAlignment(ctx.state.align),
        ],
        None => vec![SetAlignment(aligns)],
    })
}

/// Deduplicate alignments and deduce to which axes they apply.
fn parse_aligns(
    ctx: &mut LayoutContext,
    iter: impl Iterator<Item = (Option<SpecAxis>, Spanned<SpecAlign>)>,
) -> LayoutAlign {
    let mut aligns = ctx.state.align;
    let mut had = [false; 2];
    let mut deferred_center = false;

    for (axis, align) in iter {
        // Check whether we know which axis this alignment belongs to. We don't
        // if the alignment is `center` for a positional argument. Then we set
        // `deferred_center` to true and handle the situation once we know more.
        if let Some(axis) = axis {
            if align.v.axis().map_or(false, |a| a != axis) {
                error!(
                    @ctx.f, align.span,
                    "invalid alignment {} for {} axis", align.v, axis,
                );
            } else if had[axis as usize] {
                error!(@ctx.f, align.span, "duplicate alignment for {} axis", axis);
            } else {
                let gen_align = align.v.to_gen(ctx.state.sys);
                *aligns.get_mut(axis.to_gen(ctx.state.sys)) = gen_align;
                had[axis as usize] = true;
            }
        } else {
            if had == [true, true] {
                error!(@ctx.f, align.span, "duplicate alignment");
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

            *aligns.get_mut(axis.to_gen(ctx.state.sys)) = GenAlign::Center;

            had[axis as usize] = true;
            deferred_center = false;
        }
    }

    // If center has not been flushed by known, it is the only argument and then
    // we default to applying it to the primary axis.
    if deferred_center {
        aligns.primary = GenAlign::Center;
    }

    aligns
}
