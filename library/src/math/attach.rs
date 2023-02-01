use super::*;

/// # Attachment
/// A base with optional attachments.
///
/// ## Syntax
/// This function also has dedicated syntax: Use the underscore (`_`) to
/// indicate a bottom attachment and the hat (`^`) to indicate a top attachment.
///
/// ## Example
/// ```
/// $ sum_(i=0)^n a_i = 2^(1+i) $
/// ```
///
/// ## Parameters
/// - base: Content (positional, required)
///   The base to which things are attached.
///
/// - top: Content (named)
///   The top attachment.
///
/// - bottom: Content (named)
///   The bottom attachment.
///
/// ## Category
/// math
#[func]
#[capable(LayoutMath)]
#[derive(Debug, Hash)]
pub struct AttachNode {
    /// The base.
    pub base: Content,
    /// The top attachment.
    pub top: Option<Content>,
    /// The bottom attachment.
    pub bottom: Option<Content>,
}

#[node]
impl AttachNode {
    fn construct(_: &Vm, args: &mut Args) -> SourceResult<Content> {
        let base = args.expect("base")?;
        let top = args.named("top")?;
        let bottom = args.named("bottom")?;
        Ok(Self { base, top, bottom }.pack())
    }
}

impl LayoutMath for AttachNode {
    fn layout_math(&self, ctx: &mut MathContext) -> SourceResult<()> {
        let base = ctx.layout_fragment(&self.base)?;

        ctx.style(ctx.style.for_subscript());
        let top = self.top.as_ref().map(|node| ctx.layout_frame(node)).transpose()?;
        ctx.unstyle();

        ctx.style(ctx.style.for_superscript());
        let bottom =
            self.bottom.as_ref().map(|node| ctx.layout_frame(node)).transpose()?;
        ctx.unstyle();

        let render_limits = self.base.is::<LimitsNode>()
            || (!self.base.is::<ScriptsNode>()
                && ctx.style.size == MathSize::Display
                && base.class() == Some(MathClass::Large)
                && match &base {
                    MathFragment::Variant(variant) => LIMITS.contains(&variant.c),
                    MathFragment::Frame(fragment) => fragment.limits,
                    _ => false,
                });

        if render_limits {
            limits(ctx, base, top, bottom)
        } else {
            scripts(ctx, base, top, bottom)
        }
    }
}

/// # Scripts
/// Force a base to display attachments as scripts.
///
/// ## Example
/// ```
/// $ scripts(sum)_1^2 != sum_1^2 $
/// ```
///
/// ## Parameters
/// - base: Content (positional, required)
///   The base to attach the scripts to.
///
/// ## Category
/// math
#[func]
#[capable(LayoutMath)]
#[derive(Debug, Hash)]
pub struct ScriptsNode(Content);

#[node]
impl ScriptsNode {
    fn construct(_: &Vm, args: &mut Args) -> SourceResult<Content> {
        Ok(Self(args.expect("base")?).pack())
    }
}

impl LayoutMath for ScriptsNode {
    fn layout_math(&self, ctx: &mut MathContext) -> SourceResult<()> {
        self.0.layout_math(ctx)
    }
}

/// # Limits
/// Force a base to display attachments as limits.
///
/// ## Example
/// ```
/// $ limits(A)_1^2 != A_1^2 $
/// ```
///
/// ## Parameters
/// - base: Content (positional, required)
///   The base to attach the limits to.
///
/// ## Category
/// math
#[func]
#[capable(LayoutMath)]
#[derive(Debug, Hash)]
pub struct LimitsNode(Content);

#[node]
impl LimitsNode {
    fn construct(_: &Vm, args: &mut Args) -> SourceResult<Content> {
        Ok(Self(args.expect("base")?).pack())
    }
}

impl LayoutMath for LimitsNode {
    fn layout_math(&self, ctx: &mut MathContext) -> SourceResult<()> {
        self.0.layout_math(ctx)
    }
}

/// Layout sub- and superscripts.
fn scripts(
    ctx: &mut MathContext,
    base: MathFragment,
    sup: Option<Frame>,
    sub: Option<Frame>,
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
        shift_up = sup_shift_up
            .max(base.ascent() - sup_drop_max)
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
    let sub_delta = -italics;
    let sup_delta = match base.class() {
        Some(MathClass::Large) => Abs::zero(),
        _ => italics,
    };

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
    frame.push_frame(base_pos, base.to_frame(ctx));

    if let Some(sup) = sup {
        let sup_pos =
            Point::new(sup_delta + base_width, ascent - shift_up - sup.ascent());
        frame.push_frame(sup_pos, sup);
    }

    if let Some(sub) = sub {
        let sub_pos =
            Point::new(sub_delta + base_width, ascent + shift_down - sub.ascent());
        frame.push_frame(sub_pos, sub);
    }

    ctx.push(FrameFragment::new(frame).with_class(class));

    Ok(())
}

/// Layout limits.
fn limits(
    ctx: &mut MathContext,
    base: MathFragment,
    top: Option<Frame>,
    bottom: Option<Frame>,
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
    frame.push_frame(base_pos, base.to_frame(ctx));

    if let Some(top) = top {
        let top_pos = Point::with_x((width - top.width()) / 2.0 + delta);
        frame.push_frame(top_pos, top);
    }

    if let Some(bottom) = bottom {
        let bottom_pos =
            Point::new((width - bottom.width()) / 2.0 - delta, height - bottom.height());
        frame.push_frame(bottom_pos, bottom);
    }

    ctx.push(FrameFragment::new(frame).with_class(class));

    Ok(())
}

/// Codepoints that should have sub- and superscripts attached as limits.
const LIMITS: &[char] = &[
    '\u{2210}', '\u{22C1}', '\u{22C0}', '\u{2A04}', '\u{22C2}', '\u{22C3}', '\u{220F}',
    '\u{2211}', '\u{2A02}', '\u{2A01}', '\u{2A00}', '\u{2A06}',
];
