use typst::eval::combining_accent;

use super::*;

/// How much the accent can be shorter than the base.
const ACCENT_SHORT_FALL: Em = Em::new(0.5);

/// Attach an accent to a base.
///
/// ## Example
/// ```example
/// $grave(a) = accent(a, `)$ \
/// $arrow(a) = accent(a, arrow)$ \
/// $tilde(a) = accent(a, \u{0303})$
/// ```
///
/// Display: Accent
/// Category: math
#[node(LayoutMath)]
pub struct AccentNode {
    /// The base to which the accent is applied.
    /// May consist of multiple letters.
    ///
    /// ```example
    /// $arrow(A B C)$
    /// ```
    #[required]
    pub base: Content,

    /// The accent to apply to the base.
    ///
    /// Supported accents include:
    ///
    /// | Accent       | Name            | Codepoint |
    /// | ------------ | --------------- | --------- |
    /// | Grave        | `grave`         | <code>&DiacriticalGrave;</code> |
    /// | Acute        | `acute`         | `´`       |
    /// | Circumflex   | `hat`           | `^`       |
    /// | Tilde        | `tilde`         | `~`       |
    /// | Macron       | `macron`        | `¯`       |
    /// | Breve        | `breve`         | `˘`       |
    /// | Dot          | `dot`           | `.`       |
    /// | Diaeresis    | `diaer`         | `¨`       |
    /// | Circle       | `circle`        | `∘`       |
    /// | Double acute | `acute.double`  | `˝`       |
    /// | Caron        | `caron`         | `ˇ`       |
    /// | Right arrow  | `arrow`, `->`   | `→`       |
    /// | Left arrow   | `arrow.l`, `<-` | `←`       |
    #[required]
    pub accent: Accent,
}

impl LayoutMath for AccentNode {
    fn layout_math(&self, ctx: &mut MathContext) -> SourceResult<()> {
        ctx.style(ctx.style.with_cramped(true));
        let base = ctx.layout_fragment(&self.base())?;
        ctx.unstyle();

        let base_attach = match &base {
            MathFragment::Glyph(base) => {
                attachment(ctx, base.id, base.italics_correction)
            }
            _ => (base.width() + base.italics_correction()) / 2.0,
        };

        // Forcing the accent to be at least as large as the base makes it too
        // wide in many case.
        let Accent(c) = self.accent();
        let glyph = GlyphFragment::new(ctx, c, self.span());
        let short_fall = ACCENT_SHORT_FALL.scaled(ctx);
        let variant = glyph.stretch_horizontal(ctx, base.width(), short_fall);
        let accent = variant.frame;
        let accent_attach = match variant.id {
            Some(id) => attachment(ctx, id, variant.italics_correction),
            None => accent.width() / 2.0,
        };

        // Descent is negative because the accent's ink bottom is above the
        // baseline. Therefore, the default gap is the accent's negated descent
        // minus the accent base height. Only if the base is very small, we need
        // a larger gap so that the accent doesn't move too low.
        let accent_base_height = scaled!(ctx, accent_base_height);
        let gap = -accent.descent() - base.height().min(accent_base_height);
        let size = Size::new(base.width(), accent.height() + gap + base.height());
        let accent_pos = Point::with_x(base_attach - accent_attach);
        let base_pos = Point::with_y(accent.height() + gap);
        let base_ascent = base.ascent();
        let baseline = base_pos.y + base.ascent();

        let mut frame = Frame::new(size);
        frame.set_baseline(baseline);
        frame.push_frame(accent_pos, accent);
        frame.push_frame(base_pos, base.to_frame());
        ctx.push(FrameFragment::new(ctx, frame).with_base_ascent(base_ascent));

        Ok(())
    }
}

/// The horizontal attachment position for the given glyph.
fn attachment(ctx: &MathContext, id: GlyphId, italics_correction: Abs) -> Abs {
    ctx.table
        .glyph_info
        .and_then(|info| info.top_accent_attachments)
        .and_then(|attachments| attachments.get(id))
        .map(|record| record.value.scaled(ctx))
        .unwrap_or_else(|| {
            let advance = ctx.ttf.glyph_hor_advance(id).unwrap_or_default();
            (advance.scaled(ctx) + italics_correction) / 2.0
        })
}

/// An accent character.
pub struct Accent(char);

impl Accent {
    /// Normalize a character into an accent.
    pub fn new(c: char) -> Self {
        Self(combining_accent(c).unwrap_or(c))
    }
}

cast_from_value! {
    Accent,
    v: char => Self::new(v),
    v: Content => match v.to::<TextNode>() {
        Some(node) => Value::Str(node.text().into()).cast()?,
        None => Err("expected text")?,
    },
}

cast_to_value! {
    v: Accent => v.0.into()
}
