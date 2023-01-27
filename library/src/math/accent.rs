use super::*;

/// How much the accent can be shorter than the base.
const ACCENT_SHORT_FALL: Em = Em::new(0.5);

/// # Accent
/// An accented node.
///
/// ## Example
/// ```
/// $accent(a, ->) != accent(a, ~)$ \
/// $accent(a, `) = accent(a, grave)$
/// ```
///
/// ## Parameters
/// - base: Content (positional, required)
///   The base to which the accent is applied.
///   May consist of multiple letters.
///
///   ### Example
///   ```
///   $accent(A B C, ->)$
///   ```
///
/// - accent: Content (positional, required)
///   The accent to apply to the base.
///
///   Supported accents include:
///   - Plus: `` + ``
///   - Overline: `` - ``, `‾`
///   - Dot: `.`
///   - Circumflex: `^`
///   - Acute: `´`
///   - Low Line: `_`
///   - Grave: `` ` ``
///   - Tilde: `~`
///   - Diaeresis: `¨`
///   - Macron: `¯`
///   - Acute: `´`
///   - Cedilla: `¸`
///   - Caron: `ˇ`
///   - Breve: `˘`
///   - Double acute: `˝`
///   - Left arrow: `<-`
///   - Right arrow: `->`
///
/// ## Category
/// math
#[func]
#[capable(LayoutMath)]
#[derive(Debug, Hash)]
pub struct AccentNode {
    /// The accent base.
    pub base: Content,
    /// The accent.
    pub accent: Content,
}

#[node]
impl AccentNode {
    fn construct(_: &Vm, args: &mut Args) -> SourceResult<Content> {
        let base = args.expect("base")?;
        let accent = args.expect("accent")?;
        Ok(Self { base, accent }.pack())
    }
}

impl LayoutMath for AccentNode {
    fn layout_math(&self, ctx: &mut MathContext) -> SourceResult<()> {
        ctx.style(ctx.style.with_cramped(true));
        let base = ctx.layout_fragment(&self.base)?;
        ctx.unstyle();

        let base_attach = match base {
            MathFragment::Glyph(base) => {
                attachment(ctx, base.id, base.italics_correction)
            }
            _ => (base.width() + base.italics_correction()) / 2.0,
        };

        let Some(c) = extract(&self.accent) else {
            ctx.push(base);
            if let Some(span) = self.accent.span() {
                bail!(span, "not an accent");
            }
            return Ok(());
        };

        // Forcing the accent to be at least as large as the base makes it too
        // wide in many case.
        let glyph = GlyphFragment::new(ctx, c);
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
        let baseline = base_pos.y + base.ascent();

        let mut frame = Frame::new(size);
        frame.set_baseline(baseline);
        frame.push_frame(accent_pos, accent);
        frame.push_frame(base_pos, base.to_frame(ctx));
        ctx.push(frame);

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

/// Extract a single character from content.
fn extract(accent: &Content) -> Option<char> {
    let atom = accent.to::<AtomNode>()?;
    let mut chars = atom.0.chars();
    let c = chars.next().filter(|_| chars.next().is_none())?;
    Some(combining(c))
}

/// Convert from a non-combining accent to a combining one.
///
/// https://www.w3.org/TR/mathml-core/#combining-character-equivalences
fn combining(c: char) -> char {
    match c {
        '\u{002b}' => '\u{031f}',
        '\u{002d}' => '\u{0305}',
        '\u{002e}' => '\u{0307}',
        '\u{005e}' => '\u{0302}',
        '\u{005f}' => '\u{0332}',
        '\u{0060}' => '\u{0300}',
        '\u{007e}' => '\u{0303}',
        '\u{00a8}' => '\u{0308}',
        '\u{00af}' => '\u{0304}',
        '\u{00b4}' => '\u{0301}',
        '\u{00b8}' => '\u{0327}',
        '\u{02c6}' => '\u{0302}',
        '\u{02c7}' => '\u{030c}',
        '\u{02d8}' => '\u{0306}',
        '\u{02d9}' => '\u{0307}',
        '\u{02db}' => '\u{0328}',
        '\u{02dc}' => '\u{0303}',
        '\u{02dd}' => '\u{030b}',
        '\u{203e}' => '\u{0305}',
        '\u{2190}' => '\u{20d6}',
        '\u{2192}' => '\u{20d7}',
        '\u{2212}' => '\u{0305}',
        '\u{223C}' => '\u{0303}',
        '\u{22C5}' => '\u{0307}',
        '\u{27f6}' => '\u{20d7}',
        _ => c,
    }
}
