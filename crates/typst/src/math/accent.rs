use ttf_parser::GlyphId;
use unicode_math_class::MathClass;

use crate::diag::{bail, SourceResult};
use crate::foundations::{cast, elem, Content, NativeElement, Resolve, Smart, Value};
use crate::layout::{Abs, Em, Frame, Length, Point, Rel, Size};
use crate::math::{
    FrameFragment, GlyphFragment, LayoutMath, MathContext, MathFragment, Scaled,
};
use crate::symbols::Symbol;
use crate::text::TextElem;

/// How much the accent can be shorter than the base.
const ACCENT_SHORT_FALL: Em = Em::new(0.5);

/// Attaches an accent to a base.
///
/// # Example
/// ```example
/// $grave(a) = accent(a, `)$ \
/// $arrow(a) = accent(a, arrow)$ \
/// $tilde(a) = accent(a, \u{0303})$
/// ```
#[elem(LayoutMath)]
pub struct AccentElem {
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
    /// | Accent        | Name            | Codepoint |
    /// | ------------- | --------------- | --------- |
    /// | Grave         | `grave`         | <code>&DiacriticalGrave;</code> |
    /// | Acute         | `acute`         | `´`       |
    /// | Circumflex    | `hat`           | `^`       |
    /// | Tilde         | `tilde`         | `~`       |
    /// | Macron        | `macron`        | `¯`       |
    /// | Breve         | `breve`         | `˘`       |
    /// | Dot           | `dot`           | `.`       |
    /// | Double dot, Diaeresis | `dot.double`, `diaer` | `¨` |
    /// | Triple dot    | `dot.triple`    | <code>&tdot;</code> |
    /// | Quadruple dot | `dot.quad`      | <code>&DotDot;</code> |
    /// | Circle        | `circle`        | `∘`       |
    /// | Double acute  | `acute.double`  | `˝`       |
    /// | Caron         | `caron`         | `ˇ`       |
    /// | Right arrow   | `arrow`, `->`   | `→`       |
    /// | Left arrow    | `arrow.l`, `<-` | `←`       |
    /// | Left/Right arrow | `arrow.l.r`  | `↔`       |
    /// | Right harpoon | `harpoon`       | `⇀`       |
    /// | Left harpoon  | `harpoon.lt`    | `↼`       |
    #[required]
    pub accent: Accent,

    /// The size of the accent, relative to the width of the base.
    pub size: Smart<Rel<Length>>,
}

impl LayoutMath for AccentElem {
    #[typst_macros::time(name = "math.accent", span = self.span())]
    fn layout_math(&self, ctx: &mut MathContext) -> SourceResult<()> {
        ctx.style(ctx.style.with_cramped(true));
        let base = ctx.layout_fragment(self.base())?;
        ctx.unstyle();

        // Preserve class to preserve automatic spacing.
        let base_class = base.class().unwrap_or(MathClass::Normal);
        let base_attach = match &base {
            MathFragment::Glyph(base) => {
                attachment(ctx, base.id, base.italics_correction)
            }
            _ => (base.width() + base.italics_correction()) / 2.0,
        };

        let width = self
            .size(ctx.styles())
            .unwrap_or(Rel::one())
            .resolve(ctx.styles())
            .relative_to(base.width());

        // Forcing the accent to be at least as large as the base makes it too
        // wide in many case.
        let Accent(c) = self.accent();
        let glyph = GlyphFragment::new(ctx, *c, self.span());
        let short_fall = ACCENT_SHORT_FALL.scaled(ctx);
        let variant = glyph.stretch_horizontal(ctx, width, short_fall);
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

        let mut frame = Frame::soft(size);
        frame.set_baseline(baseline);
        frame.push_frame(accent_pos, accent);
        frame.push_frame(base_pos, base.into_frame());
        ctx.push(
            FrameFragment::new(ctx, frame)
                .with_class(base_class)
                .with_base_ascent(base_ascent),
        );

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
#[derive(Debug, Default, Copy, Clone, Eq, PartialEq, Hash)]
pub struct Accent(char);

impl Accent {
    /// Normalize a character into an accent.
    pub fn new(c: char) -> Self {
        Self(Symbol::combining_accent(c).unwrap_or(c))
    }
}

cast! {
    Accent,
    self => self.0.into_value(),
    v: char => Self::new(v),
    v: Content => match v.to::<TextElem>() {
        Some(elem) => Value::Str(elem.text().clone().into()).cast()?,
        None => bail!("expected text"),
    },
}
