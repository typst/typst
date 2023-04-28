use super::*;

/// A base with optional attachments.
///
/// ## Example
/// ```example
/// // With syntax.
/// $ sum_(i=0)^n a_i = 2^(1+i) $
///
/// // With function call.
/// $ attach(
///   Pi, t: alpha, b: beta,
///   tl: 1, tr: 2+3, bl: 4+5, br: 6,
/// ) $
/// ```
///
/// ## Syntax
/// This function also has dedicated syntax for attachments after the base: Use
/// the underscore (`_`) to indicate a subscript i.e. bottom attachment and the
/// hat (`^`) to indicate a superscript i.e. top attachment.
///
/// Display: Attachment
/// Category: math
#[element(LayoutMath)]
pub struct AttachElem {
    /// The base to which things are attached.
    #[required]
    pub base: Content,

    /// The top attachment, smartly positioned at top-right or above the base.
    ///
    /// You can wrap the base in `{limits()}` or `{scripts()}` to override the
    /// smart positioning.
    pub t: Option<Content>,

    /// The bottom attachment, smartly positioned at the bottom-right or below
    /// the base.
    ///
    /// You can wrap the base in `{limits()}` or `{scripts()}` to override the
    /// smart positioning.
    pub b: Option<Content>,

    /// The top-left attachment (before the base).
    pub tl: Option<Content>,

    /// The bottom-left attachment (before base).
    pub bl: Option<Content>,

    /// The top-right attachment (after the base).
    pub tr: Option<Content>,

    /// The bottom-right attachment (after the base).
    pub br: Option<Content>,
}

impl LayoutMath for AttachElem {
    #[tracing::instrument(skip(ctx))]
    fn layout_math(&self, ctx: &mut MathContext) -> SourceResult<()> {
        type GetAttachment = fn(&AttachElem, styles: StyleChain) -> Option<Content>;
        let getarg = |ctx: &mut MathContext, getter: GetAttachment| {
            getter(self, ctx.styles())
                .map(|elem| ctx.layout_fragment(&elem))
                .transpose()
                .unwrap()
        };

        let base = ctx.layout_fragment(&self.base())?;

        ctx.style(ctx.style.for_superscript());
        let arg_tl = getarg(ctx, Self::tl);
        let arg_tr = getarg(ctx, Self::tr);
        let arg_t = getarg(ctx, Self::t);
        ctx.unstyle();

        ctx.style(ctx.style.for_subscript());
        let arg_bl = getarg(ctx, Self::bl);
        let arg_br = getarg(ctx, Self::br);
        let arg_b = getarg(ctx, Self::b);
        ctx.unstyle();

        let as_limits = self.base().is::<LimitsElem>()
            || (!self.base().is::<ScriptsElem>()
                && ctx.style.size == MathSize::Display
                && base.class() == Some(MathClass::Large)
                && match &base {
                    MathFragment::Variant(variant) => LIMITS.contains(&variant.c),
                    MathFragment::Frame(fragment) => fragment.limits,
                    _ => false,
                });

        let (t, tr) =
            if as_limits || arg_tr.is_some() { (arg_t, arg_tr) } else { (None, arg_t) };
        let (b, br) =
            if as_limits || arg_br.is_some() { (arg_b, arg_br) } else { (None, arg_b) };

        layout_attachments(ctx, base, [arg_tl, t, tr, arg_bl, b, br])
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
    #[tracing::instrument(skip(ctx))]
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
    #[tracing::instrument(skip(ctx))]
    fn layout_math(&self, ctx: &mut MathContext) -> SourceResult<()> {
        self.body().layout_math(ctx)
    }
}

/// Layout the attachments.
fn layout_attachments(
    ctx: &mut MathContext,
    base: MathFragment,
    [tl, t, tr, bl, b, br]: [Option<MathFragment>; 6],
) -> SourceResult<()> {
    let (shift_up, shift_down) =
        compute_shifts_up_and_down(ctx, &base, [&tl, &tr, &bl, &br]);

    let sup_delta = Abs::zero();
    let sub_delta = -base.italics_correction();
    let (base_width, base_ascent, base_descent) =
        (base.width(), base.ascent(), base.descent());
    let base_class = base.class().unwrap_or(MathClass::Normal);

    macro_rules! measure {
        ($e: ident, $attr: ident) => {
            $e.as_ref().map(|e| e.$attr()).unwrap_or_default()
        };
    }

    let ascent = base_ascent
        .max(shift_up + measure!(tr, ascent))
        .max(shift_up + measure!(tl, ascent))
        .max(shift_up + measure!(t, height));

    let descent = base_descent
        .max(shift_down + measure!(br, descent))
        .max(shift_down + measure!(bl, descent))
        .max(shift_down + measure!(b, height));

    let pre_sup_width = measure!(tl, width);
    let pre_sub_width = measure!(bl, width);
    let pre_width_dif = pre_sup_width - pre_sub_width; // Could be negative.
    let pre_width_max = pre_sup_width.max(pre_sub_width);
    let post_max_width =
        (sup_delta + measure!(tr, width)).max(sub_delta + measure!(br, width));

    let (center_frame, base_offset) = attach_top_and_bottom(ctx, base, t, b);
    let base_pos =
        Point::new(sup_delta + pre_width_max, ascent - base_ascent - base_offset);
    if [&tl, &bl, &tr, &br].iter().all(|&e| e.is_none()) {
        ctx.push(FrameFragment::new(ctx, center_frame).with_class(base_class));
        return Ok(());
    }

    let mut frame = Frame::new(Size::new(
        pre_width_max + base_width + post_max_width + scaled!(ctx, space_after_script),
        ascent + descent,
    ));
    frame.set_baseline(ascent);
    frame.push_frame(base_pos, center_frame);

    if let Some(tl) = tl {
        let pos =
            Point::new(-pre_width_dif.min(Abs::zero()), ascent - shift_up - tl.ascent());
        frame.push_frame(pos, tl.into_frame());
    }

    if let Some(bl) = bl {
        let pos =
            Point::new(pre_width_dif.max(Abs::zero()), ascent + shift_down - bl.ascent());
        frame.push_frame(pos, bl.into_frame());
    }

    if let Some(tr) = tr {
        let pos = Point::new(
            sup_delta + pre_width_max + base_width,
            ascent - shift_up - tr.ascent(),
        );
        frame.push_frame(pos, tr.into_frame());
    }

    if let Some(br) = br {
        let pos = Point::new(
            sub_delta + pre_width_max + base_width,
            ascent + shift_down - br.ascent(),
        );
        frame.push_frame(pos, br.into_frame());
    }

    ctx.push(FrameFragment::new(ctx, frame).with_class(base_class));

    Ok(())
}

fn attach_top_and_bottom(
    ctx: &mut MathContext,
    base: MathFragment,
    t: Option<MathFragment>,
    b: Option<MathFragment>,
) -> (Frame, Abs) {
    let upper_gap_min = scaled!(ctx, upper_limit_gap_min);
    let upper_rise_min = scaled!(ctx, upper_limit_baseline_rise_min);
    let lower_gap_min = scaled!(ctx, lower_limit_gap_min);
    let lower_drop_min = scaled!(ctx, lower_limit_baseline_drop_min);

    let mut base_offset = Abs::zero();
    let mut width = base.width();
    let mut height = base.height();

    if let Some(t) = &t {
        let top_gap = upper_gap_min.max(upper_rise_min - t.descent());
        width.set_max(t.width());
        height += t.height() + top_gap;
        base_offset = top_gap + t.height();
    }

    if let Some(b) = &b {
        let bottom_gap = lower_gap_min.max(lower_drop_min - b.ascent());
        width.set_max(b.width());
        height += b.height() + bottom_gap;
    }

    let base_pos = Point::new((width - base.width()) / 2.0, base_offset);
    let delta = base.italics_correction() / 2.0;

    let mut frame = Frame::new(Size::new(width, height));
    frame.set_baseline(base_pos.y + base.ascent());
    frame.push_frame(base_pos, base.into_frame());

    if let Some(t) = t {
        let top_pos = Point::with_x((width - t.width()) / 2.0 + delta);
        frame.push_frame(top_pos, t.into_frame());
    }

    if let Some(b) = b {
        let bottom_pos =
            Point::new((width - b.width()) / 2.0 - delta, height - b.height());
        frame.push_frame(bottom_pos, b.into_frame());
    }

    (frame, base_offset)
}

fn compute_shifts_up_and_down(
    ctx: &MathContext,
    base: &MathFragment,
    [tl, tr, bl, br]: [&Option<MathFragment>; 4],
) -> (Abs, Abs) {
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

    let mut shift_up = Abs::zero();
    let mut shift_down = Abs::zero();

    for e in [tl, tr].into_iter().flatten() {
        let ascent = match &base {
            MathFragment::Frame(frame) => frame.base_ascent,
            _ => base.ascent(),
        };

        shift_up = shift_up
            .max(sup_shift_up)
            .max(ascent - sup_drop_max)
            .max(sup_bottom_min + e.descent());
    }
    for e in [bl, br].into_iter().flatten() {
        shift_down = shift_down
            .max(sub_shift_down)
            .max(base.descent() + sub_drop_min)
            .max(e.ascent() - sub_top_max);
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

/// Unicode codepoints that should have sub- and superscripts attached as limits.
#[rustfmt::skip]
const LIMITS: &[char] = &[
    /* ∏ */ '\u{220F}', /* ∐ */ '\u{2210}', /* ∑ */ '\u{2211}',
    /* ⋀ */ '\u{22C0}', /* ⋁ */ '\u{22C1}',
    /* ⋂ */ '\u{22C2}', /* ⋃ */ '\u{22C3}',
    /* ⨀ */ '\u{2A00}', /* ⨁ */ '\u{2A01}', /* ⨂ */ '\u{2A02}',
    /* ⨃ */ '\u{2A03}', /* ⨄ */ '\u{2A04}',
    /* ⨅ */ '\u{2A05}', /* ⨆ */ '\u{2A06}',
];
