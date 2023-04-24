use super::*;

/// A base with optional attachments.
///
/// ## Syntax
/// This function also has dedicated syntax for attachments after the base: Use the
/// underscore (`_`) to indicate a subscript i.e. bottom attachment and the hat (`^`)
/// to indicate a superscript i.e. top attachment.
///
/// ## Example
/// ```example
/// $ sum_(i=0)^n a_i = 2^(1+i) $
/// ```
///
/// Display: Attachment
/// Category: math
#[element(LayoutMath)]
pub struct AttachElem {
    /// The base to which things are attached.
    #[required]
    pub base: Content,

    /// The top attachment, smartly placed at top-right or above the base.
    pub t: Option<Content>,

    /// The bottom attachment, smartly placed at the bottom-right or below the base.
    pub b: Option<Content>,

    /// The top-left attachment before the base.
    pub tl: Option<Content>,

    /// The bottom-left attachment before base.
    pub bl: Option<Content>,

    /// The top-right attachment after the base.
    pub tr: Option<Content>,

    /// The bottom-right attachment after the base.
    pub br: Option<Content>,
}

type GetAttachmentContent =
    fn(&AttachElem, styles: ::typst::model::StyleChain) -> Option<Content>;

impl LayoutMath for AttachElem {
    fn layout_math(&self, ctx: &mut MathContext) -> SourceResult<()> {
        let base = ctx.layout_fragment(&self.base())?;

        let get_fragment = |ctx: &mut MathContext, getter: GetAttachmentContent| {
            getter(self, ctx.styles())
                .map(|elem| ctx.layout_fragment(&elem))
                .transpose()
                .unwrap()
        };

        ctx.style(ctx.style.for_superscript());
        let arg_tl = get_fragment(ctx, Self::tl);
        let arg_tr = get_fragment(ctx, Self::tr);
        let arg_t = get_fragment(ctx, Self::t);
        ctx.unstyle();

        ctx.style(ctx.style.for_subscript());
        let arg_bl = get_fragment(ctx, Self::bl);
        let arg_br = get_fragment(ctx, Self::br);
        let arg_b = get_fragment(ctx, Self::b);
        ctx.unstyle();

        let render_as_limits =
            [&arg_tl, &arg_tr, &arg_bl, &arg_br].iter().all(|e| e.is_none())
                && (self.base().is::<LimitsElem>()
                    || (!self.base().is::<ScriptsElem>()
                        && ctx.style.size == MathSize::Display
                        && base.class() == Some(MathClass::Large)
                        && match &base {
                            MathFragment::Variant(variant) => LIMITS.contains(&variant.c),
                            MathFragment::Frame(fragment) => fragment.limits,
                            _ => false,
                        }));

        if render_as_limits {
            limits(ctx, base, arg_t, arg_b)
            // scripts(ctx, base, None, arg_t, None, None, arg_b, None)
        } else {
            scripts(
                ctx,
                base,
                arg_tl,
                if arg_tr.is_some() { arg_t.clone() } else { None },
                if arg_tr.is_some() { arg_tr } else { arg_t },
                arg_bl,
                if arg_br.is_some() { arg_b.clone() } else { None },
                if arg_br.is_some() { arg_br } else { arg_b },
            )
        }
    }
}

/// Force a base to display attachments as scripts.
///
/// ## Example
/// ```example
/// $ scripts(sum)_1^2 != sum_1^2 $
/// ```
///
/// Display: Scripts
/// Category: math
#[element(LayoutMath)]
pub struct ScriptsElem {
    /// The base to attach the scripts to.
    #[required]
    pub body: Content,
}

impl LayoutMath for ScriptsElem {
    fn layout_math(&self, ctx: &mut MathContext) -> SourceResult<()> {
        self.body().layout_math(ctx)
    }
}

/// Force a base to display attachments as limits.
///
/// ## Example
/// ```example
/// $ limits(A)_1^2 != A_1^2 $
/// ```
///
/// Display: Limits
/// Category: math
#[element(LayoutMath)]
pub struct LimitsElem {
    /// The base to attach the limits to.
    #[required]
    pub body: Content,
}

impl LayoutMath for LimitsElem {
    fn layout_math(&self, ctx: &mut MathContext) -> SourceResult<()> {
        self.body().layout_math(ctx)
    }
}

/// Layout sub- and superscripts.
#[allow(clippy::too_many_arguments)]
fn scripts(
    ctx: &mut MathContext,
    base: MathFragment,
    topleft: Option<MathFragment>,
    topabove: Option<MathFragment>,
    topright: Option<MathFragment>,
    bottomleft: Option<MathFragment>,
    bottombelow: Option<MathFragment>,
    bottomright: Option<MathFragment>,
) -> SourceResult<()> {
    let sup_shift_up = if ctx.style.cramped {
        scaled!(ctx, superscript_shift_up_cramped)
    } else {
        scaled!(ctx, superscript_shift_up)
    };
    let sup_bottom_min = scaled!(ctx, superscript_bottom_min);
    let sup_bottom_max_with_sub = scaled!(ctx, superscript_bottom_max_with_subscript);
    let sup_drop_max = scaled!(ctx, superscript_baseline_drop_max);
    let gap_min = scaled!(ctx, sub_superscript_gap_min);
    let sub_shift_down = scaled!(ctx, subscript_shift_down);
    let sub_top_max = scaled!(ctx, subscript_top_max);
    let sub_drop_min = scaled!(ctx, subscript_baseline_drop_min);
    let space_after = scaled!(ctx, space_after_script);

    let mut shift_up = Abs::zero();
    let mut shift_down = Abs::zero();

    for e in [&topleft, &topright].into_iter().flatten() {
        let ascent = match &base {
            MathFragment::Frame(frame) => frame.base_ascent,
            _ => base.ascent(),
        };

        shift_up = shift_up
            .max(sup_shift_up)
            .max(ascent - sup_drop_max)
            .max(sup_bottom_min + e.descent());
    }
    for e in [&bottomleft, &bottomright].into_iter().flatten() {
        shift_down = shift_down
            .max(sub_shift_down)
            .max(base.descent() + sub_drop_min)
            .max(e.ascent() - sub_top_max);
    }

    for (sup, sub) in [(&topleft, &bottomleft), (&topright, &bottomright)] {
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

    let italics = base.italics_correction();
    let sup_delta = Abs::zero();
    let sub_delta = -italics;

    macro_rules! measure {
        ($e: ident, $attr: ident) => {
            $e.as_ref().map(|e| e.$attr()).unwrap_or_default()
        };
    }

    let ascent = base
        .ascent()
        .max(shift_up + measure!(topright, ascent))
        .max(shift_up + measure!(topleft, ascent))
        .max(shift_up + measure!(topabove, height));

    let descent = base
        .descent()
        .max(shift_down + measure!(bottomright, descent))
        .max(shift_down + measure!(bottomleft, descent))
        .max(shift_down + measure!(bottombelow, height));

    let pre_sup_width = measure!(topleft, width);
    let pre_sub_width = measure!(bottomleft, width);
    let pre_width_dif = pre_sup_width - pre_sub_width; // Could be negative.
    let pre_width_max = pre_sup_width.max(pre_sub_width);
    let post_max_width = (sup_delta + measure!(topright, width))
        .max(sub_delta + measure!(bottomright, width));

    let mut frame = Frame::new(Size::new(
        pre_width_max + base.width() + post_max_width + space_after,
        ascent + descent,
    ));

    if let Some(topleft) = topleft {
        let pos = Point::new(
            -pre_width_dif.min(Abs::zero()),
            ascent - shift_up - topleft.ascent(),
        );
        frame.push_frame(pos, topleft.into_frame());
    }

    if let Some(bottomleft) = bottomleft {
        let pos = Point::new(
            pre_width_dif.max(Abs::zero()),
            ascent + shift_down - bottomleft.ascent(),
        );
        frame.push_frame(pos, bottomleft.into_frame());
    }

    let base_width = base.width();
    let base_ascent = base.ascent();
    let class = base.class().unwrap_or(MathClass::Normal);

    frame.set_baseline(ascent);
    let (frame_with_limits, base_offset) =
        attach_top_and_bottom(ctx, base, topabove, bottombelow);
    let base_pos =
        Point::new(sup_delta + pre_width_max, ascent - base_ascent - base_offset);
    frame.push_frame(base_pos, frame_with_limits);

    if let Some(topright) = topright {
        let pos = Point::new(
            sup_delta + pre_width_max + base_width,
            ascent - shift_up - topright.ascent(),
        );
        frame.push_frame(pos, topright.into_frame());
    }

    if let Some(bottomright) = bottomright {
        let pos = Point::new(
            sub_delta + pre_width_max + base_width,
            ascent + shift_down - bottomright.ascent(),
        );
        frame.push_frame(pos, bottomright.into_frame());
    }

    ctx.push(FrameFragment::new(ctx, frame).with_class(class));

    Ok(())
}

/// Layout limits.
fn limits(
    ctx: &mut MathContext,
    base: MathFragment,
    top: Option<MathFragment>,
    bottom: Option<MathFragment>,
) -> SourceResult<()> {
    let class = base.class().unwrap_or(MathClass::Normal);
    let (frame, _) = attach_top_and_bottom(ctx, base, top, bottom);
    ctx.push(FrameFragment::new(ctx, frame).with_class(class));

    Ok(())
}

fn attach_top_and_bottom(
    ctx: &mut MathContext,
    base: MathFragment,
    top: Option<MathFragment>,
    bottom: Option<MathFragment>,
) -> (Frame, Abs) {
    let upper_gap_min = scaled!(ctx, upper_limit_gap_min);
    let upper_rise_min = scaled!(ctx, upper_limit_baseline_rise_min);
    let lower_gap_min = scaled!(ctx, lower_limit_gap_min);
    let lower_drop_min = scaled!(ctx, lower_limit_baseline_drop_min);

    let mut base_offset = Abs::zero();
    let mut width = base.width();
    let mut height = base.height();

    if let Some(top) = &top {
        let top_gap = upper_gap_min.max(upper_rise_min - top.descent());
        width.set_max(top.width());
        height += top.height() + top_gap;
        base_offset = top_gap + top.height();
    }

    if let Some(bottom) = &bottom {
        let bottom_gap = lower_gap_min.max(lower_drop_min - bottom.ascent());
        width.set_max(bottom.width());
        height += bottom.height() + bottom_gap;
    }

    let base_pos = Point::new((width - base.width()) / 2.0, base_offset);
    let delta = base.italics_correction() / 2.0;

    let mut frame = Frame::new(Size::new(width, height));
    frame.set_baseline(base_pos.y + base.ascent());
    frame.push_frame(base_pos, base.into_frame());

    if let Some(top) = top {
        let top_pos = Point::with_x((width - top.width()) / 2.0 + delta);
        frame.push_frame(top_pos, top.into_frame());
    }

    if let Some(bottom) = bottom {
        let bottom_pos =
            Point::new((width - bottom.width()) / 2.0 - delta, height - bottom.height());
        frame.push_frame(bottom_pos, bottom.into_frame());
    }

    (frame, base_offset)
}

/// Codepoints that should have sub- and superscripts attached as limits.
const LIMITS: &[char] = &[
    '\u{2210}', '\u{22C1}', '\u{22C0}', '\u{2A04}', '\u{22C2}', '\u{22C3}', '\u{220F}',
    '\u{2211}', '\u{2A02}', '\u{2A01}', '\u{2A00}', '\u{2A06}',
];
