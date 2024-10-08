use crate::diag::{bail, SourceResult};
use crate::foundations::{
    cast, elem, func, Content, NativeElement, Packed, Smart, StyleChain, Value,
};
use crate::layout::{Em, Frame, Length, Point, Rel, Size};
use crate::math::{
    scaled_font_size, style_cramped, FrameFragment, GlyphFragment, LayoutMath,
    MathContext, MathFragment, Scaled,
};
use crate::text::TextElem;

/// How much the accent can be shorter than the base.
const ACCENT_SHORT_FALL: Em = Em::new(0.5);

/// This macro generates accent-related functions.
///
/// ```ignore
/// accents! {
///     '\u{0300}' | '`' => grave,
/// //  ^^^^^^^^^    ^^^    ^^^^^
/// //  |            |      |
/// //  |            |      +-- The name of the function.
/// //  |            +--------- The alternative characters that represent the accent.
/// //  +---------------------- The primary character that represents the accent.
/// }
/// ```
///
/// When combined with the `Accent::combine` function, accent characters can be normalized
/// to the primary character.
macro_rules! accents {
    ($($primary:literal $(| $alt:literal)* => $name:ident),* $(,)?) => {
        impl Accent {
            /// Normalize an accent to a combining one.
            pub fn combine(c: char) -> Option<char> {
                Some(match c {
                    $($primary $(| $alt)* => $primary,)*
                    _ => return None,
                })
            }
        }

        $(
            /// The accent function for callable symbol definitions.
            #[func]
            pub fn $name(
                /// The base to which the accent is applied.
                base: Content,
                /// The size of the accent, relative to the width of the base.
                #[named]
                size: Option<Smart<Rel<Length>>>,
            ) -> Content {
                let mut accent = AccentElem::new(base, Accent::new($primary));
                if let Some(size) = size {
                    accent = accent.with_size(size);
                }
                accent.pack()
            }
        )+
    };
}

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
    /// | Dash          | `dash`          | `‾`       |
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

impl LayoutMath for Packed<AccentElem> {
    #[typst_macros::time(name = "math.accent", span = self.span())]
    fn layout_math(&self, ctx: &mut MathContext, styles: StyleChain) -> SourceResult<()> {
        let cramped = style_cramped();
        let base = ctx.layout_into_fragment(self.base(), styles.chain(&cramped))?;

        // Preserve class to preserve automatic spacing.
        let base_class = base.class();
        let base_attach = base.accent_attach();

        let width = self
            .size(styles)
            .unwrap_or(Rel::one())
            .map(|length| length.at(scaled_font_size(ctx, styles)))
            .relative_to(base.width());

        // Forcing the accent to be at least as large as the base makes it too
        // wide in many case.
        let Accent(c) = self.accent();
        let glyph = GlyphFragment::new(ctx, styles, *c, self.span());
        let short_fall = ACCENT_SHORT_FALL.at(glyph.font_size);
        let variant = glyph.stretch_horizontal(ctx, width, short_fall);
        let accent = variant.frame;
        let accent_attach = variant.accent_attach;

        // Descent is negative because the accent's ink bottom is above the
        // baseline. Therefore, the default gap is the accent's negated descent
        // minus the accent base height. Only if the base is very small, we need
        // a larger gap so that the accent doesn't move too low.
        let accent_base_height = scaled!(ctx, styles, accent_base_height);
        let gap = -accent.descent() - base.height().min(accent_base_height);
        let size = Size::new(base.width(), accent.height() + gap + base.height());
        let accent_pos = Point::with_x(base_attach - accent_attach);
        let base_pos = Point::with_y(accent.height() + gap);
        let baseline = base_pos.y + base.ascent();
        let base_italics_correction = base.italics_correction();
        let base_text_like = base.is_text_like();

        let base_ascent = match &base {
            MathFragment::Frame(frame) => frame.base_ascent,
            _ => base.ascent(),
        };

        let mut frame = Frame::soft(size);
        frame.set_baseline(baseline);
        frame.push_frame(accent_pos, accent);
        frame.push_frame(base_pos, base.into_frame());
        ctx.push(
            FrameFragment::new(ctx, styles, frame)
                .with_class(base_class)
                .with_base_ascent(base_ascent)
                .with_italics_correction(base_italics_correction)
                .with_accent_attach(base_attach)
                .with_text_like(base_text_like),
        );

        Ok(())
    }
}

/// An accent character.
#[derive(Debug, Default, Copy, Clone, Eq, PartialEq, Hash)]
pub struct Accent(char);

impl Accent {
    /// Normalize a character into an accent.
    pub fn new(c: char) -> Self {
        Self(Self::combine(c).unwrap_or(c))
    }
}

// Keep it synced with the documenting table above.
accents! {
    '\u{0300}' | '`' => grave,
    '\u{0301}' | '´' => acute,
    '\u{0302}' | '^' | 'ˆ' => hat,
    '\u{0303}' | '~' | '∼' | '˜' => tilde,
    '\u{0304}' | '¯' => macron,
    '\u{0305}' | '-' | '‾' | '−' => dash,
    '\u{0306}' | '˘' => breve,
    '\u{0307}' | '.' | '˙' | '⋅' => dot,
    '\u{0308}' | '¨' => dot_double,
    '\u{20db}' => dot_triple,
    '\u{20dc}' => dot_quad,
    '\u{030a}' | '∘' | '○' => circle,
    '\u{030b}' | '˝' => acute_double,
    '\u{030c}' | 'ˇ' => caron,
    '\u{20d6}' | '←' => arrow_l,
    '\u{20d7}' | '→' | '⟶' => arrow,
    '\u{20e1}' | '↔' | '⟷' => arrow_l_r,
    '\u{20d0}' | '↼' => harpoon_lt,
    '\u{20d1}' | '⇀' => harpoon,
}

cast! {
    Accent,
    self => self.0.into_value(),
    v: char => Self::new(v),
    v: Content => match v.to_packed::<TextElem>() {
        Some(elem) => Value::Str(elem.text().clone().into()).cast()?,
        None => bail!("expected text"),
    },
}
