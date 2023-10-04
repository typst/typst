use super::*;

/// A base with optional attachments.
///
/// ```example
/// $ attach(
///   Pi, t: alpha, b: beta,
///   tl: 1, tr: 2+3, bl: 4+5, br: 6,
/// ) $
/// ```
#[elem(LayoutMath)]
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
        let layout_attachment = |ctx: &mut MathContext, getter: GetAttachment| {
            getter(self, ctx.styles())
                .map(|elem| ctx.layout_fragment(&elem))
                .transpose()
        };

        let base = ctx.layout_fragment(&self.base())?;

        ctx.style(ctx.style.for_superscript());
        let tl = layout_attachment(ctx, Self::tl)?;
        let tr = layout_attachment(ctx, Self::tr)?;
        let t = layout_attachment(ctx, Self::t)?;
        ctx.unstyle();

        ctx.style(ctx.style.for_subscript());
        let bl = layout_attachment(ctx, Self::bl)?;
        let br = layout_attachment(ctx, Self::br)?;
        let b = layout_attachment(ctx, Self::b)?;
        ctx.unstyle();

        let limits = base.limits().active(ctx);
        let (t, tr) = if limits || tr.is_some() { (t, tr) } else { (None, t) };
        let (b, br) = if limits || br.is_some() { (b, br) } else { (None, b) };
        layout_attachments(ctx, base, [tl, t, tr, bl, b, br])
    }
}

/// Grouped primes.
///
/// ```example
/// $ a'''_b = a^'''_b $
/// ```
///
/// # Syntax
/// This function has dedicated syntax: use apostrophes instead of primes. They
/// will automatically attach to the previous element, moving superscripts to
/// the next level.
#[elem(LayoutMath)]
pub struct PrimesElem {
    /// The number of grouped primes.
    #[required]
    pub count: usize,
}

impl LayoutMath for PrimesElem {
    #[tracing::instrument(skip(ctx))]
    fn layout_math(&self, ctx: &mut MathContext) -> SourceResult<()> {
        match self.count() {
            count @ 1..=4 => {
                let f = ctx.layout_fragment(&TextElem::packed(match count {
                    1 => '′',
                    2 => '″',
                    3 => '‴',
                    4 => '⁗',
                    _ => unreachable!(),
                }))?;
                ctx.push(f);
            }
            count => {
                // Custom amount of primes
                let prime = ctx.layout_fragment(&TextElem::packed('′'))?.into_frame();
                let width = prime.width() * (count + 1) as f64 / 2.0;
                let mut frame = Frame::soft(Size::new(width, prime.height()));
                frame.set_baseline(prime.ascent());

                for i in 0..count {
                    frame.push_frame(
                        Point::new(prime.width() * (i as f64 / 2.0), Abs::zero()),
                        prime.clone(),
                    )
                }
                ctx.push(FrameFragment::new(ctx, frame));
            }
        }
        Ok(())
    }
}

/// Forces a base to display attachments as scripts.
///
/// ```example
/// $ scripts(sum)_1^2 != sum_1^2 $
/// ```
#[elem(LayoutMath)]
pub struct ScriptsElem {
    /// The base to attach the scripts to.
    #[required]
    pub body: Content,
}

impl LayoutMath for ScriptsElem {
    #[tracing::instrument(skip(ctx))]
    fn layout_math(&self, ctx: &mut MathContext) -> SourceResult<()> {
        let mut fragment = ctx.layout_fragment(&self.body())?;
        fragment.set_limits(Limits::Never);
        ctx.push(fragment);
        Ok(())
    }
}

/// Forces a base to display attachments as limits.
///
/// ```example
/// $ limits(A)_1^2 != A_1^2 $
/// ```
#[elem(LayoutMath)]
pub struct LimitsElem {
    /// The base to attach the limits to.
    #[required]
    pub body: Content,

    /// Whether to also force limits in inline equations.
    ///
    /// When applying limits globally (e.g., through a show rule), it is
    /// typically a good idea to disable this.
    #[default(true)]
    pub inline: bool,
}

impl LayoutMath for LimitsElem {
    #[tracing::instrument(skip(ctx))]
    fn layout_math(&self, ctx: &mut MathContext) -> SourceResult<()> {
        let mut fragment = ctx.layout_fragment(&self.body())?;
        fragment.set_limits(if self.inline(ctx.styles()) {
            Limits::Always
        } else {
            Limits::Display
        });
        ctx.push(fragment);
        Ok(())
    }
}

/// Describes in which situation a frame should use limits for attachments.
#[derive(Debug, Copy, Clone)]
pub enum Limits {
    /// Always scripts.
    Never,
    /// Display limits only in `display` math.
    Display,
    /// Always limits.
    Always,
}

impl Limits {
    /// The default limit configuration if the given character is the base.
    pub fn for_char(c: char) -> Self {
        match unicode_math_class::class(c) {
            Some(MathClass::Large) => {
                if is_integral_char(c) {
                    Limits::Never
                } else {
                    Limits::Display
                }
            }
            Some(MathClass::Relation) => Limits::Always,
            _ => Limits::Never,
        }
    }

    /// Whether limits should be displayed in this context
    pub fn active(&self, ctx: &MathContext) -> bool {
        match self {
            Self::Always => true,
            Self::Display => ctx.style.size == MathSize::Display,
            Self::Never => false,
        }
    }
}

macro_rules! measure {
    ($e: ident, $attr: ident) => {
        $e.as_ref().map(|e| e.$attr()).unwrap_or_default()
    };
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

    let mut frame = Frame::soft(Size::new(
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

    let mut frame = Frame::soft(Size::new(width, height));
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
    let is_char_box = is_character_box(base);

    if tl.is_some() || tr.is_some() {
        let ascent = match &base {
            MathFragment::Frame(frame) => frame.base_ascent,
            _ => base.ascent(),
        };
        shift_up = shift_up
            .max(sup_shift_up)
            .max(if is_char_box { Abs::zero() } else { ascent - sup_drop_max })
            .max(sup_bottom_min + measure!(tl, descent))
            .max(sup_bottom_min + measure!(tr, descent));
    }

    if bl.is_some() || br.is_some() {
        shift_down = shift_down
            .max(sub_shift_down)
            .max(if is_char_box { Abs::zero() } else { base.descent() + sub_drop_min })
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

/// Determines if the character is one of a variety of integral signs
fn is_integral_char(c: char) -> bool {
    ('∫'..='∳').contains(&c) || ('⨋'..='⨜').contains(&c)
}

/// Whether the fragment consists of a single character or atomic piece of text.
fn is_character_box(fragment: &MathFragment) -> bool {
    match fragment {
        MathFragment::Glyph(_) | MathFragment::Variant(_) => {
            fragment.class() != Some(MathClass::Large)
        }
        MathFragment::Frame(fragment) => is_atomic_text_frame(&fragment.frame),
        _ => false,
    }
}

/// Handles e.g. "sin", "log", "exp", "CustomOperator".
fn is_atomic_text_frame(frame: &Frame) -> bool {
    // Meta information isn't visible or renderable, so we exclude it.
    let mut iter = frame
        .items()
        .map(|(_, item)| item)
        .filter(|item| !matches!(item, FrameItem::Meta(_, _)));
    matches!(iter.next(), Some(FrameItem::Text(_))) && iter.next().is_none()
}
