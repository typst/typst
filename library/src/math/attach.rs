use super::*;

/// A base with optional attachments.
///
/// ## Syntax
/// This function also has dedicated syntax: Use the underscore (`_`) to
/// indicate a bottom attachment and the hat (`^`) to indicate a top attachment.
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

    /// The top attachment.
    pub top: Option<Content>,

    /// The bottom attachment.
    pub bottom: Option<Content>,
}

impl LayoutMath for AttachElem {
    #[tracing::instrument(skip(ctx))]
    fn layout_math(&self, ctx: &mut MathContext) -> SourceResult<()> {
        let base = self.base();
        let display_limits = base.is::<LimitsElem>();
        let display_scripts = base.is::<ScriptsElem>();

        let base = ctx.layout_fragment(&base)?;

        ctx.style(ctx.style.for_superscript());
        let top = self
            .top(ctx.styles())
            .map(|elem| ctx.layout_fragment(&elem))
            .transpose()?;
        ctx.unstyle();

        ctx.style(ctx.style.for_subscript());
        let bottom = self
            .bottom(ctx.styles())
            .map(|elem| ctx.layout_fragment(&elem))
            .transpose()?;
        ctx.unstyle();

        let display_limits = display_limits
            || (!display_scripts
                && ctx.style.size == MathSize::Display
                && base.class() == Some(MathClass::Large)
                && match &base {
                    MathFragment::Variant(variant) => LIMITS.contains(&variant.c),
                    MathFragment::Frame(fragment) => fragment.limits,
                    _ => false,
                });

        if display_limits {
            limits(ctx, base, top, bottom)
        } else {
            scripts(ctx, base, top, bottom)
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

/// Layout sub- and superscripts.
fn scripts(
    ctx: &mut MathContext,
    base: MathFragment,
    sup: Option<MathFragment>,
    sub: Option<MathFragment>,
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

    if let Some(sup) = &sup {
        let ascent = match &base {
            MathFragment::Frame(frame) => frame.base_ascent,
            _ => base.ascent(),
        };

        shift_up = sup_shift_up
            .max(ascent - sup_drop_max)
            .max(sup_bottom_min + sup.descent());
    }

    if let Some(sub) = &sub {
        shift_down = sub_shift_down
            .max(base.descent() + sub_drop_min)
            .max(sub.ascent() - sub_top_max);
    }

    if let (Some(sup), Some(sub)) = (&sup, &sub) {
        let sup_bottom = shift_up - sup.descent();
        let sub_top = sub.ascent() - shift_down;
        let gap = sup_bottom - sub_top;
        if gap < gap_min {
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

    let mut width = Abs::zero();
    let mut ascent = base.ascent();
    let mut descent = base.descent();

    if let Some(sup) = &sup {
        ascent.set_max(shift_up + sup.ascent());
        width.set_max(sup_delta + sup.width());
    }

    if let Some(sub) = &sub {
        descent.set_max(shift_down + sub.descent());
        width.set_max(sub_delta + sub.width());
    }

    width += base.width() + space_after;

    let base_pos = Point::with_y(ascent - base.ascent());
    let base_width = base.width();
    let class = base.class().unwrap_or(MathClass::Normal);

    let mut frame = Frame::new(Size::new(width, ascent + descent));
    frame.set_baseline(ascent);
    frame.push_frame(base_pos, base.into_frame());

    if let Some(sup) = sup {
        let sup_pos =
            Point::new(sup_delta + base_width, ascent - shift_up - sup.ascent());
        frame.push_frame(sup_pos, sup.into_frame());
    }

    if let Some(sub) = sub {
        let sub_pos =
            Point::new(sub_delta + base_width, ascent + shift_down - sub.ascent());
        frame.push_frame(sub_pos, sub.into_frame());
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
    let class = base.class().unwrap_or(MathClass::Normal);
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

    ctx.push(FrameFragment::new(ctx, frame).with_class(class));

    Ok(())
}

/// Codepoints that should have sub- and superscripts attached as limits.
const LIMITS: &[char] = &[
    '\u{2210}', '\u{22C1}', '\u{22C0}', '\u{2A04}', '\u{22C2}', '\u{22C3}', '\u{220F}',
    '\u{2211}', '\u{2A02}', '\u{2A01}', '\u{2A00}', '\u{2A06}',
];
