use typst_library::diag::SourceResult;
use typst_library::foundations::{Packed, StyleChain, SymbolElem};
use typst_library::layout::{Abs, Axis, Corner, Frame, Point, Size};
use typst_library::math::{
    AttachElem, EquationElem, LimitsElem, PrimesElem, ScriptsElem, StretchElem,
    StretchSize,
};
use typst_utils::OptionExt;

use super::{
    FrameFragment, Limits, MathContext, MathFragment, stretch_fragment,
    style_for_subscript, style_for_superscript,
};

macro_rules! measure {
    ($e: ident, $attr: ident) => {
        $e.as_ref().map(|e| e.$attr()).unwrap_or_default()
    };
}

/// Lays out an [`AttachElem`].
#[typst_macros::time(name = "math.attach", span = elem.span())]
pub fn layout_attach(
    elem: &Packed<AttachElem>,
    ctx: &mut MathContext,
    styles: StyleChain,
) -> SourceResult<()> {
    let merged = elem.merge_base();
    let elem = merged.as_ref().unwrap_or(elem);
    let stretch = stretch_size(styles, elem);

    let mut base = ctx.layout_into_fragment(&elem.base, styles)?;
    let sup_style = style_for_superscript(styles);
    let sup_style_chain = styles.chain(&sup_style);
    let tl = elem.tl.get_cloned(sup_style_chain);
    let tr = elem.tr.get_cloned(sup_style_chain);
    let primed = tr.as_ref().is_some_and(|content| content.is::<PrimesElem>());
    let t = elem.t.get_cloned(sup_style_chain);

    let sub_style = style_for_subscript(styles);
    let sub_style_chain = styles.chain(&sub_style);
    let bl = elem.bl.get_cloned(sub_style_chain);
    let br = elem.br.get_cloned(sub_style_chain);
    let b = elem.b.get_cloned(sub_style_chain);

    let limits = base.limits().active(styles);
    let (t, tr) = match (t, tr) {
        (Some(t), Some(tr)) if primed && !limits => (None, Some(tr + t)),
        (Some(t), None) if !limits => (None, Some(t)),
        (t, tr) => (t, tr),
    };
    let (b, br) = if limits || br.is_some() { (b, br) } else { (None, b) };

    macro_rules! layout {
        ($content:ident, $style_chain:ident) => {
            $content
                .map(|elem| ctx.layout_into_fragment(&elem, $style_chain))
                .transpose()
        };
    }

    // Layout the top and bottom attachments early so we can measure their
    // widths, in order to calculate what the stretch size is relative to.
    let t = layout!(t, sup_style_chain)?;
    let b = layout!(b, sub_style_chain)?;
    if let Some(stretch) = stretch {
        let relative_to_width = measure!(t, width).max(measure!(b, width));
        stretch_fragment(
            ctx,
            styles,
            &mut base,
            Some(Axis::X),
            Some(relative_to_width),
            &stretch,
        )?;
    }

    let fragments = [
        layout!(tl, sup_style_chain)?,
        t,
        layout!(tr, sup_style_chain)?,
        layout!(bl, sub_style_chain)?,
        b,
        layout!(br, sub_style_chain)?,
    ];

    layout_attachments(ctx, styles, base, fragments)
}

/// Lays out a [`PrimeElem`].
#[typst_macros::time(name = "math.primes", span = elem.span())]
pub fn layout_primes(
    elem: &Packed<PrimesElem>,
    ctx: &mut MathContext,
    styles: StyleChain,
) -> SourceResult<()> {
    match elem.count {
        count @ 1..=4 => {
            let c = match count {
                1 => '′',
                2 => '″',
                3 => '‴',
                4 => '⁗',
                _ => unreachable!(),
            };
            let f = ctx.layout_into_fragment(&SymbolElem::packed(c), styles)?;
            ctx.push(f);
        }
        count => {
            // Custom amount of primes
            let prime = ctx
                .layout_into_fragment(&SymbolElem::packed('′'), styles)?
                .into_frame();
            let width = prime.width() * (count + 1) as f64 / 2.0;
            let mut frame = Frame::soft(Size::new(width, prime.height()));
            frame.set_baseline(prime.ascent());

            for i in 0..count {
                frame.push_frame(
                    Point::new(prime.width() * (i as f64 / 2.0), Abs::zero()),
                    prime.clone(),
                )
            }
            ctx.push(FrameFragment::new(styles, frame).with_text_like(true));
        }
    }
    Ok(())
}

/// Lays out a [`ScriptsElem`].
#[typst_macros::time(name = "math.scripts", span = elem.span())]
pub fn layout_scripts(
    elem: &Packed<ScriptsElem>,
    ctx: &mut MathContext,
    styles: StyleChain,
) -> SourceResult<()> {
    let mut fragment = ctx.layout_into_fragment(&elem.body, styles)?;
    fragment.set_limits(Limits::Never);
    ctx.push(fragment);
    Ok(())
}

/// Lays out a [`LimitsElem`].
#[typst_macros::time(name = "math.limits", span = elem.span())]
pub fn layout_limits(
    elem: &Packed<LimitsElem>,
    ctx: &mut MathContext,
    styles: StyleChain,
) -> SourceResult<()> {
    let limits = if elem.inline.get(styles) { Limits::Always } else { Limits::Display };
    let mut fragment = ctx.layout_into_fragment(&elem.body, styles)?;
    fragment.set_limits(limits);
    ctx.push(fragment);
    Ok(())
}

/// Get the size to stretch the base to.
fn stretch_size(styles: StyleChain, elem: &Packed<AttachElem>) -> Option<StretchSize> {
    // Extract from an EquationElem.
    let mut base = &elem.base;
    while let Some(equation) = base.to_packed::<EquationElem>() {
        base = &equation.body;
    }

    base.to_packed::<StretchElem>()
        .map(|stretch| stretch.size.get_cloned(styles))
}

/// Lay out the attachments.
fn layout_attachments(
    ctx: &mut MathContext,
    styles: StyleChain,
    base: MathFragment,
    [tl, t, tr, bl, b, br]: [Option<MathFragment>; 6],
) -> SourceResult<()> {
    let base_class = base.class();

    // Calculate the distance from the base's baseline to the superscripts' and
    // subscripts' baseline.
    let (tx_shift, bx_shift) = if [&tl, &tr, &bl, &br].iter().all(|e| e.is_none()) {
        (Abs::zero(), Abs::zero())
    } else {
        compute_script_shifts(ctx, styles, &base, [&tl, &tr, &bl, &br])
    };

    // Calculate the distance from the base's baseline to the top attachment's
    // and bottom attachment's baseline.
    let (t_shift, b_shift) =
        compute_limit_shifts(ctx, styles, &base, [t.as_ref(), b.as_ref()]);

    // Calculate the final frame height.
    let ascent = base
        .ascent()
        .max(tx_shift + measure!(tr, ascent))
        .max(tx_shift + measure!(tl, ascent))
        .max(t_shift + measure!(t, ascent));
    let descent = base
        .descent()
        .max(bx_shift + measure!(br, descent))
        .max(bx_shift + measure!(bl, descent))
        .max(b_shift + measure!(b, descent));
    let height = ascent + descent;

    // Calculate the vertical position of each element in the final frame.
    let base_y = ascent - base.ascent();
    let tx_y = |tx: &MathFragment| ascent - tx_shift - tx.ascent();
    let bx_y = |bx: &MathFragment| ascent + bx_shift - bx.ascent();
    let t_y = |t: &MathFragment| ascent - t_shift - t.ascent();
    let b_y = |b: &MathFragment| ascent + b_shift - b.ascent();

    // Calculate the distance each limit extends to the left and right of the
    // base's width.
    let ((t_pre_width, t_post_width), (b_pre_width, b_post_width)) =
        compute_limit_widths(&base, [t.as_ref(), b.as_ref()]);

    // `space_after_script` is extra spacing that is at the start before each
    // pre-script, and at the end after each post-script (see the MathConstants
    // table in the OpenType MATH spec).
    let space_after_script = scaled!(ctx, styles, space_after_script);

    // Calculate the distance each pre-script extends to the left of the base's
    // width.
    let (tl_pre_width, bl_pre_width) = compute_pre_script_widths(
        &base,
        [tl.as_ref(), bl.as_ref()],
        (tx_shift, bx_shift),
        space_after_script,
    );

    // Calculate the distance each post-script extends to the right of the
    // base's width. Also calculate each post-script's kerning (we need this for
    // its position later).
    let ((tr_post_width, tr_kern), (br_post_width, br_kern)) = compute_post_script_widths(
        &base,
        [tr.as_ref(), br.as_ref()],
        (tx_shift, bx_shift),
        space_after_script,
    );

    // Calculate the final frame width.
    let pre_width = t_pre_width.max(b_pre_width).max(tl_pre_width).max(bl_pre_width);
    let base_width = base.width();
    let post_width = t_post_width.max(b_post_width).max(tr_post_width).max(br_post_width);
    let width = pre_width + base_width + post_width;

    // Calculate the horizontal position of each element in the final frame.
    let base_x = pre_width;
    let tl_x = pre_width - tl_pre_width + space_after_script;
    let bl_x = pre_width - bl_pre_width + space_after_script;
    let tr_x = pre_width + base_width + tr_kern;
    let br_x = pre_width + base_width + br_kern;
    let t_x = pre_width - t_pre_width;
    let b_x = pre_width - b_pre_width;

    // Create the final frame.
    let mut frame = Frame::soft(Size::new(width, height));
    frame.set_baseline(ascent);
    frame.push_frame(Point::new(base_x, base_y), base.into_frame());

    macro_rules! layout {
        ($e: ident, $x: ident, $y: ident) => {
            if let Some($e) = $e {
                frame.push_frame(Point::new($x, $y(&$e)), $e.into_frame());
            }
        };
    }

    layout!(tl, tl_x, tx_y); // pre-superscript
    layout!(bl, bl_x, bx_y); // pre-subscript
    layout!(tr, tr_x, tx_y); // post-superscript
    layout!(br, br_x, bx_y); // post-subscript
    layout!(t, t_x, t_y); // upper-limit
    layout!(b, b_x, b_y); // lower-limit

    // Done! Note that we retain the class of the base.
    ctx.push(FrameFragment::new(styles, frame).with_class(base_class));

    Ok(())
}

/// Calculate the distance each post-script extends to the right of the base's
/// width, as well as its kerning value. Requires the distance from the base's
/// baseline to each post-script's baseline to obtain the correct kerning value.
/// Returns 2 tuples of two lengths, each first containing the distance the
/// post-script extends left of the base's width and second containing the
/// post-script's kerning value. The first tuple is for the post-superscript,
/// and the second is for the post-subscript.
fn compute_post_script_widths(
    base: &MathFragment,
    [tr, br]: [Option<&MathFragment>; 2],
    (tr_shift, br_shift): (Abs, Abs),
    space_after_post_script: Abs,
) -> ((Abs, Abs), (Abs, Abs)) {
    let tr_values = tr.map_or_default(|tr| {
        let kern = math_kern(base, tr, tr_shift, Corner::TopRight);
        (space_after_post_script + tr.width() + kern, kern)
    });

    // The base's bounding box already accounts for its italic correction, so we
    // need to shift the post-subscript left by the base's italic correction
    // (see the kerning algorithm as described in the OpenType MATH spec).
    let br_values = br.map_or_default(|br| {
        let kern = math_kern(base, br, br_shift, Corner::BottomRight)
            - base.italics_correction();
        (space_after_post_script + br.width() + kern, kern)
    });

    (tr_values, br_values)
}

/// Calculate the distance each pre-script extends to the left of the base's
/// width. Requires the distance from the base's baseline to each pre-script's
/// baseline to obtain the correct kerning value.
/// Returns two lengths, the first being the distance the pre-superscript
/// extends left of the base's width and the second being the distance the
/// pre-subscript extends left of the base's width.
fn compute_pre_script_widths(
    base: &MathFragment,
    [tl, bl]: [Option<&MathFragment>; 2],
    (tl_shift, bl_shift): (Abs, Abs),
    space_before_pre_script: Abs,
) -> (Abs, Abs) {
    let tl_pre_width = tl.map_or_default(|tl| {
        let kern = math_kern(base, tl, tl_shift, Corner::TopLeft);
        space_before_pre_script + tl.width() + kern
    });

    let bl_pre_width = bl.map_or_default(|bl| {
        let kern = math_kern(base, bl, bl_shift, Corner::BottomLeft);
        space_before_pre_script + bl.width() + kern
    });

    (tl_pre_width, bl_pre_width)
}

/// Calculate the distance each limit extends beyond the base's width, in each
/// direction. Can be a negative value if the limit does not extend beyond the
/// base's width, indicating how far into the base's width the limit extends.
/// Returns 2 tuples of two lengths, each first containing the distance the
/// limit extends leftward beyond the base's width and second containing the
/// distance the limit extends rightward beyond the base's width. The first
/// tuple is for the upper-limit, and the second is for the lower-limit.
fn compute_limit_widths(
    base: &MathFragment,
    [t, b]: [Option<&MathFragment>; 2],
) -> ((Abs, Abs), (Abs, Abs)) {
    // The upper- (lower-) limit is shifted to the right (left) of the base's
    // center by half the base's italic correction.
    let delta = base.italics_correction() / 2.0;

    let t_widths = t.map_or_default(|t| {
        let half = (t.width() - base.width()) / 2.0;
        (half - delta, half + delta)
    });

    let b_widths = b.map_or_default(|b| {
        let half = (b.width() - base.width()) / 2.0;
        (half + delta, half - delta)
    });

    (t_widths, b_widths)
}

/// Calculate the distance from the base's baseline to each limit's baseline.
/// Returns two lengths, the first being the distance to the upper-limit's
/// baseline and the second being the distance to the lower-limit's baseline.
fn compute_limit_shifts(
    ctx: &MathContext,
    styles: StyleChain,
    base: &MathFragment,
    [t, b]: [Option<&MathFragment>; 2],
) -> (Abs, Abs) {
    // `upper_gap_min` and `lower_gap_min` give gaps to the descender and
    // ascender of the limits respectively, whereas `upper_rise_min` and
    // `lower_drop_min` give gaps to each limit's baseline (see the
    // MathConstants table in the OpenType MATH spec).

    let t_shift = t.map_or_default(|t| {
        let upper_gap_min = scaled!(ctx, styles, upper_limit_gap_min);
        let upper_rise_min = scaled!(ctx, styles, upper_limit_baseline_rise_min);
        base.ascent() + upper_rise_min.max(upper_gap_min + t.descent())
    });

    let b_shift = b.map_or_default(|b| {
        let lower_gap_min = scaled!(ctx, styles, lower_limit_gap_min);
        let lower_drop_min = scaled!(ctx, styles, lower_limit_baseline_drop_min);
        base.descent() + lower_drop_min.max(lower_gap_min + b.ascent())
    });

    (t_shift, b_shift)
}

/// Calculate the distance from the base's baseline to each script's baseline.
/// Returns two lengths, the first being the distance to the superscripts'
/// baseline and the second being the distance to the subscripts' baseline.
fn compute_script_shifts(
    ctx: &MathContext,
    styles: StyleChain,
    base: &MathFragment,
    [tl, tr, bl, br]: [&Option<MathFragment>; 4],
) -> (Abs, Abs) {
    let sup_shift_up = if styles.get(EquationElem::cramped) {
        scaled!(ctx, styles, superscript_shift_up_cramped)
    } else {
        scaled!(ctx, styles, superscript_shift_up)
    };

    let sup_bottom_min = scaled!(ctx, styles, superscript_bottom_min);
    let sup_bottom_max_with_sub =
        scaled!(ctx, styles, superscript_bottom_max_with_subscript);
    let sup_drop_max = scaled!(ctx, styles, superscript_baseline_drop_max);
    let gap_min = scaled!(ctx, styles, sub_superscript_gap_min);
    let sub_shift_down = scaled!(ctx, styles, subscript_shift_down);
    let sub_top_max = scaled!(ctx, styles, subscript_top_max);
    let sub_drop_min = scaled!(ctx, styles, subscript_baseline_drop_min);

    let mut shift_up = Abs::zero();
    let mut shift_down = Abs::zero();
    let is_text_like = base.is_text_like();

    if tl.is_some() || tr.is_some() {
        let ascent = match &base {
            MathFragment::Frame(frame) => frame.base_ascent,
            _ => base.ascent(),
        };
        shift_up = shift_up
            .max(sup_shift_up)
            .max(if is_text_like { Abs::zero() } else { ascent - sup_drop_max })
            .max(sup_bottom_min + measure!(tl, descent))
            .max(sup_bottom_min + measure!(tr, descent));
    }

    if bl.is_some() || br.is_some() {
        let descent = match &base {
            MathFragment::Frame(frame) => frame.base_descent,
            _ => base.descent(),
        };
        shift_down = shift_down
            .max(sub_shift_down)
            .max(if is_text_like { Abs::zero() } else { descent + sub_drop_min })
            .max(measure!(bl, ascent) - sub_top_max)
            .max(measure!(br, ascent) - sub_top_max);
    }

    for (sup, sub) in [(tl, bl), (tr, br)] {
        if let (Some(sup), Some(sub)) = (&sup, &sub) {
            let sup_bottom = shift_up - sup.descent();
            let sub_top = sub.ascent() - shift_down;
            let gap = sup_bottom - sub_top;
            if gap >= gap_min {
                continue;
            }

            let increase = gap_min - gap;
            let sup_only =
                (sup_bottom_max_with_sub - sup_bottom).clamp(Abs::zero(), increase);
            let rest = (increase - sup_only) / 2.0;
            shift_up += sup_only + rest;
            shift_down += rest;
        }
    }

    (shift_up, shift_down)
}

/// Calculate the kerning value for a script with respect to the base. A
/// positive value means shifting the script further away from the base, whereas
/// a negative value means shifting the script closer to the base. Requires the
/// distance from the base's baseline to the script's baseline, as well as the
/// script's corner (tl, tr, bl, br).
fn math_kern(base: &MathFragment, script: &MathFragment, shift: Abs, pos: Corner) -> Abs {
    // This process is described under the MathKernInfo table in the OpenType
    // MATH spec.

    let (corr_height_top, corr_height_bot) = match pos {
        // Calculate two correction heights for superscripts:
        // - The distance from the superscript's baseline to the top of the
        //   base's bounding box.
        // - The distance from the base's baseline to the bottom of the
        //   superscript's bounding box.
        Corner::TopLeft | Corner::TopRight => {
            (base.ascent() - shift, shift - script.descent())
        }
        // Calculate two correction heights for subscripts:
        // - The distance from the base's baseline to the top of the
        //   subscript's bounding box.
        // - The distance from the subscript's baseline to the bottom of the
        //   base's bounding box.
        Corner::BottomLeft | Corner::BottomRight => {
            (script.ascent() - shift, shift - base.descent())
        }
    };

    // Calculate the sum of kerning values for each correction height.
    let summed_kern = |height| {
        let base_kern = base.kern_at_height(pos, height);
        let attach_kern = script.kern_at_height(pos.inv(), height);
        base_kern + attach_kern
    };

    // Take the smaller kerning amount (and so the larger value). Note that
    // there is a bug in the spec (as of 2024-08-15): it says to take the
    // minimum of the two sums, but as the kerning value is usually negative it
    // really means the smaller kern. The current wording of the spec could
    // result in glyphs colliding.
    summed_kern(corr_height_top).max(summed_kern(corr_height_bot))
}
