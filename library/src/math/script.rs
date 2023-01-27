use super::*;

/// # Script
/// A mathematical sub- and/or superscript.
///
/// ## Syntax
/// This function also has dedicated syntax: Use the underscore (`_`) to
/// indicate a subscript and the circumflex (`^`) to indicate a superscript.
///
/// ## Example
/// ```
/// $ a_i = 2^(1+i) $
/// ```
///
/// ## Parameters
/// - base: Content (positional, required)
///   The base to which the applies the sub- and/or superscript.
///
/// - sub: Content (named)
///   The subscript.
///
/// - sup: Content (named)
///   The superscript.
///
/// ## Category
/// math
#[func]
#[capable(LayoutMath)]
#[derive(Debug, Hash)]
pub struct ScriptNode {
    /// The base.
    pub base: Content,
    /// The subscript.
    pub sub: Option<Content>,
    /// The superscript.
    pub sup: Option<Content>,
}

#[node]
impl ScriptNode {
    fn construct(_: &Vm, args: &mut Args) -> SourceResult<Content> {
        let base = args.expect("base")?;
        let sub = args.named("sub")?;
        let sup = args.named("sup")?;
        Ok(Self { base, sub, sup }.pack())
    }
}

impl LayoutMath for ScriptNode {
    fn layout_math(&self, ctx: &mut MathContext) -> SourceResult<()> {
        let base = ctx.layout_fragment(&self.base)?;

        let mut sub = Frame::new(Size::zero());
        if let Some(node) = &self.sub {
            ctx.style(ctx.style.for_subscript());
            sub = ctx.layout_frame(node)?;
            ctx.unstyle();
        }

        let mut sup = Frame::new(Size::zero());
        if let Some(node) = &self.sup {
            ctx.style(ctx.style.for_superscript());
            sup = ctx.layout_frame(node)?;
            ctx.unstyle();
        }

        let render_limits = ctx.style.size == MathSize::Display
            && base.class() == Some(MathClass::Large)
            && match &base {
                MathFragment::Variant(variant) => LIMITS.contains(&variant.c),
                MathFragment::Frame(fragment) => fragment.limits,
                _ => false,
            };

        if render_limits {
            limits(ctx, base, sub, sup)
        } else {
            scripts(ctx, base, sub, sup, self.sub.is_some() && self.sup.is_some())
        }
    }
}

/// Layout normal sub- and superscripts.
fn scripts(
    ctx: &mut MathContext,
    base: MathFragment,
    sub: Frame,
    sup: Frame,
    both: bool,
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

    let mut shift_up = sup_shift_up
        .max(base.ascent() - sup_drop_max)
        .max(sup_bottom_min + sup.descent());

    let mut shift_down = sub_shift_down
        .max(base.descent() + sub_drop_min)
        .max(sub.ascent() - sub_top_max);

    if both {
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

    let delta = base.italics_correction();
    let ascent = shift_up + sup.ascent();
    let descent = shift_down + sub.descent();
    let height = ascent + descent;
    let width = base.width() + sup.width().max(sub.width() - delta) + space_after;
    let base_pos = Point::with_y(ascent - base.ascent());
    let sup_pos = Point::with_x(base.width());
    let sub_pos = Point::new(base.width() - delta, height - sub.height());
    let class = base.class().unwrap_or(MathClass::Normal);

    let mut frame = Frame::new(Size::new(width, height));
    frame.set_baseline(ascent);
    frame.push_frame(base_pos, base.to_frame(ctx));
    frame.push_frame(sub_pos, sub);
    frame.push_frame(sup_pos, sup);
    ctx.push(FrameFragment::new(frame).with_class(class));

    Ok(())
}

/// Layout limits.
fn limits(
    ctx: &mut MathContext,
    base: MathFragment,
    sub: Frame,
    sup: Frame,
) -> SourceResult<()> {
    let upper_gap_min = scaled!(ctx, upper_limit_gap_min);
    let upper_rise_min = scaled!(ctx, upper_limit_baseline_rise_min);
    let lower_gap_min = scaled!(ctx, lower_limit_gap_min);
    let lower_drop_min = scaled!(ctx, lower_limit_baseline_drop_min);

    let sup_gap = upper_gap_min.max(upper_rise_min - sup.descent());
    let sub_gap = lower_gap_min.max(lower_drop_min - sub.ascent());

    let delta = base.italics_correction() / 2.0;
    let width = base.width().max(sup.width()).max(sub.width());
    let height = sup.height() + sup_gap + base.height() + sub_gap + sub.height();
    let base_pos = Point::new((width - base.width()) / 2.0, sup.height() + sup_gap);
    let sup_pos = Point::with_x((width - sup.width()) / 2.0 + delta);
    let sub_pos = Point::new((width - sub.width()) / 2.0 - delta, height - sub.height());
    let class = base.class().unwrap_or(MathClass::Normal);

    let mut frame = Frame::new(Size::new(width, height));
    frame.set_baseline(base_pos.y + base.ascent());
    frame.push_frame(base_pos, base.to_frame(ctx));
    frame.push_frame(sub_pos, sub);
    frame.push_frame(sup_pos, sup);
    ctx.push(FrameFragment::new(frame).with_class(class));

    Ok(())
}

/// Codepoints that should have sub- and superscripts attached as limits.
const LIMITS: &[char] = &[
    '\u{2210}', '\u{22C1}', '\u{22C0}', '\u{2A04}', '\u{22C2}', '\u{22C3}', '\u{220F}',
    '\u{2211}', '\u{2A02}', '\u{2A01}', '\u{2A00}', '\u{2A06}',
];
