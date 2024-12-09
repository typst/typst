use std::f64::consts::SQRT_2;

use ecow::{eco_vec, EcoString};
use typst_library::diag::SourceResult;
use typst_library::foundations::{Packed, StyleChain, StyleVec};
use typst_library::layout::{Abs, Size};
use typst_library::math::{EquationElem, MathSize, MathVariant};
use typst_library::text::{
    BottomEdge, BottomEdgeMetric, TextElem, TextSize, TopEdge, TopEdgeMetric,
};
use typst_syntax::{is_newline, Span};
use unicode_math_class::MathClass;
use unicode_segmentation::UnicodeSegmentation;

use super::{
    scaled_font_size, FrameFragment, GlyphFragment, MathContext, MathFragment, MathRun,
};

/// Lays out a [`TextElem`].
pub fn layout_text(
    elem: &Packed<TextElem>,
    ctx: &mut MathContext,
    styles: StyleChain,
) -> SourceResult<()> {
    let text = elem.text();
    let span = elem.span();
    let mut chars = text.chars();
    let math_size = EquationElem::size_in(styles);
    let mut dtls = ctx.dtls_table.is_some();
    let fragment: MathFragment = if let Some(mut glyph) = chars
        .next()
        .filter(|_| chars.next().is_none())
        .map(|c| dtls_char(c, &mut dtls))
        .map(|c| styled_char(styles, c, true))
        .and_then(|c| GlyphFragment::try_new(ctx, styles, c, span))
    {
        // A single letter that is available in the math font.
        if dtls {
            glyph.make_dotless_form(ctx);
        }

        match math_size {
            MathSize::Script => {
                glyph.make_script_size(ctx);
            }
            MathSize::ScriptScript => {
                glyph.make_script_script_size(ctx);
            }
            _ => (),
        }

        if glyph.class == MathClass::Large {
            let mut variant = if math_size == MathSize::Display {
                let height = scaled!(ctx, styles, display_operator_min_height)
                    .max(SQRT_2 * glyph.height());
                glyph.stretch_vertical(ctx, height, Abs::zero())
            } else {
                glyph.into_variant()
            };
            // TeXbook p 155. Large operators are always vertically centered on the axis.
            variant.center_on_axis(ctx);
            variant.into()
        } else {
            glyph.into()
        }
    } else if text.chars().all(|c| c.is_ascii_digit() || c == '.') {
        // Numbers aren't that difficult.
        let mut fragments = vec![];
        for c in text.chars() {
            let c = styled_char(styles, c, false);
            fragments.push(GlyphFragment::new(ctx, styles, c, span).into());
        }
        let frame = MathRun::new(fragments).into_frame(ctx, styles);
        FrameFragment::new(ctx, styles, frame).with_text_like(true).into()
    } else {
        let local = [
            TextElem::set_top_edge(TopEdge::Metric(TopEdgeMetric::Bounds)),
            TextElem::set_bottom_edge(BottomEdge::Metric(BottomEdgeMetric::Bounds)),
            TextElem::set_size(TextSize(scaled_font_size(ctx, styles).into())),
        ]
        .map(|p| p.wrap());

        // Anything else is handled by Typst's standard text layout.
        let styles = styles.chain(&local);
        let text: EcoString =
            text.chars().map(|c| styled_char(styles, c, false)).collect();
        if text.contains(is_newline) {
            let mut fragments = vec![];
            for (i, piece) in text.split(is_newline).enumerate() {
                if i != 0 {
                    fragments.push(MathFragment::Linebreak);
                }
                if !piece.is_empty() {
                    fragments.push(layout_complex_text(piece, ctx, span, styles)?.into());
                }
            }
            let mut frame = MathRun::new(fragments).into_frame(ctx, styles);
            let axis = scaled!(ctx, styles, axis_height);
            frame.set_baseline(frame.height() / 2.0 + axis);
            FrameFragment::new(ctx, styles, frame).into()
        } else {
            layout_complex_text(&text, ctx, span, styles)?.into()
        }
    };

    ctx.push(fragment);
    Ok(())
}

/// Layout the given text string into a [`FrameFragment`].
fn layout_complex_text(
    text: &str,
    ctx: &mut MathContext,
    span: Span,
    styles: StyleChain,
) -> SourceResult<FrameFragment> {
    // There isn't a natural width for a paragraph in a math environment;
    // because it will be placed somewhere probably not at the left margin
    // it will overflow. So emulate an `hbox` instead and allow the paragraph
    // to extend as far as needed.
    let spaced = text.graphemes(true).nth(1).is_some();
    let elem = TextElem::packed(text).spanned(span);
    let frame = (ctx.engine.routines.layout_inline)(
        ctx.engine,
        &StyleVec::wrap(eco_vec![elem]),
        ctx.locator.next(&span),
        styles,
        false,
        Size::splat(Abs::inf()),
        false,
    )?
    .into_frame();

    Ok(FrameFragment::new(ctx, styles, frame)
        .with_class(MathClass::Alphabetic)
        .with_text_like(true)
        .with_spaced(spaced))
}

/// Select the correct styled math letter.
///
/// <https://www.w3.org/TR/mathml-core/#new-text-transform-mappings>
/// <https://en.wikipedia.org/wiki/Mathematical_Alphanumeric_Symbols>
fn styled_char(styles: StyleChain, c: char, auto_italic: bool) -> char {
    use MathVariant::*;

    let variant = EquationElem::variant_in(styles);
    let bold = EquationElem::bold_in(styles);
    let italic = EquationElem::italic_in(styles).unwrap_or(
        auto_italic
            && matches!(
                c,
                'a'..='z' | 'ħ' | 'ı' | 'ȷ' | 'A'..='Z' |
                'α'..='ω' | '∂' | 'ϵ' | 'ϑ' | 'ϰ' | 'ϕ' | 'ϱ' | 'ϖ'
            )
            && matches!(variant, Sans | Serif),
    );

    if let Some(c) = basic_exception(c) {
        return c;
    }

    if let Some(c) = latin_exception(c, variant, bold, italic) {
        return c;
    }

    if let Some(c) = greek_exception(c, variant, bold, italic) {
        return c;
    }

    let base = match c {
        'A'..='Z' => 'A',
        'a'..='z' => 'a',
        'Α'..='Ω' => 'Α',
        'α'..='ω' => 'α',
        '0'..='9' => '0',
        // Hebrew Alef -> Dalet.
        '\u{05D0}'..='\u{05D3}' => '\u{05D0}',
        _ => return c,
    };

    let tuple = (variant, bold, italic);
    let start = match c {
        // Latin upper.
        'A'..='Z' => match tuple {
            (Serif, false, false) => 0x0041,
            (Serif, true, false) => 0x1D400,
            (Serif, false, true) => 0x1D434,
            (Serif, true, true) => 0x1D468,
            (Sans, false, false) => 0x1D5A0,
            (Sans, true, false) => 0x1D5D4,
            (Sans, false, true) => 0x1D608,
            (Sans, true, true) => 0x1D63C,
            (Cal, false, _) => 0x1D49C,
            (Cal, true, _) => 0x1D4D0,
            (Frak, false, _) => 0x1D504,
            (Frak, true, _) => 0x1D56C,
            (Mono, _, _) => 0x1D670,
            (Bb, _, _) => 0x1D538,
        },

        // Latin lower.
        'a'..='z' => match tuple {
            (Serif, false, false) => 0x0061,
            (Serif, true, false) => 0x1D41A,
            (Serif, false, true) => 0x1D44E,
            (Serif, true, true) => 0x1D482,
            (Sans, false, false) => 0x1D5BA,
            (Sans, true, false) => 0x1D5EE,
            (Sans, false, true) => 0x1D622,
            (Sans, true, true) => 0x1D656,
            (Cal, false, _) => 0x1D4B6,
            (Cal, true, _) => 0x1D4EA,
            (Frak, false, _) => 0x1D51E,
            (Frak, true, _) => 0x1D586,
            (Mono, _, _) => 0x1D68A,
            (Bb, _, _) => 0x1D552,
        },

        // Greek upper.
        'Α'..='Ω' => match tuple {
            (Serif, false, false) => 0x0391,
            (Serif, true, false) => 0x1D6A8,
            (Serif, false, true) => 0x1D6E2,
            (Serif, true, true) => 0x1D71C,
            (Sans, _, false) => 0x1D756,
            (Sans, _, true) => 0x1D790,
            (Cal | Frak | Mono | Bb, _, _) => return c,
        },

        // Greek lower.
        'α'..='ω' => match tuple {
            (Serif, false, false) => 0x03B1,
            (Serif, true, false) => 0x1D6C2,
            (Serif, false, true) => 0x1D6FC,
            (Serif, true, true) => 0x1D736,
            (Sans, _, false) => 0x1D770,
            (Sans, _, true) => 0x1D7AA,
            (Cal | Frak | Mono | Bb, _, _) => return c,
        },

        // Hebrew Alef -> Dalet.
        '\u{05D0}'..='\u{05D3}' => 0x2135,

        // Numbers.
        '0'..='9' => match tuple {
            (Serif, false, _) => 0x0030,
            (Serif, true, _) => 0x1D7CE,
            (Bb, _, _) => 0x1D7D8,
            (Sans, false, _) => 0x1D7E2,
            (Sans, true, _) => 0x1D7EC,
            (Mono, _, _) => 0x1D7F6,
            (Cal | Frak, _, _) => return c,
        },

        _ => unreachable!(),
    };

    std::char::from_u32(start + (c as u32 - base as u32)).unwrap()
}

fn basic_exception(c: char) -> Option<char> {
    Some(match c {
        '〈' => '⟨',
        '〉' => '⟩',
        '《' => '⟪',
        '》' => '⟫',
        _ => return None,
    })
}

fn latin_exception(
    c: char,
    variant: MathVariant,
    bold: bool,
    italic: bool,
) -> Option<char> {
    use MathVariant::*;
    Some(match (c, variant, bold, italic) {
        ('B', Cal, false, _) => 'ℬ',
        ('E', Cal, false, _) => 'ℰ',
        ('F', Cal, false, _) => 'ℱ',
        ('H', Cal, false, _) => 'ℋ',
        ('I', Cal, false, _) => 'ℐ',
        ('L', Cal, false, _) => 'ℒ',
        ('M', Cal, false, _) => 'ℳ',
        ('R', Cal, false, _) => 'ℛ',
        ('C', Frak, false, _) => 'ℭ',
        ('H', Frak, false, _) => 'ℌ',
        ('I', Frak, false, _) => 'ℑ',
        ('R', Frak, false, _) => 'ℜ',
        ('Z', Frak, false, _) => 'ℨ',
        ('C', Bb, ..) => 'ℂ',
        ('H', Bb, ..) => 'ℍ',
        ('N', Bb, ..) => 'ℕ',
        ('P', Bb, ..) => 'ℙ',
        ('Q', Bb, ..) => 'ℚ',
        ('R', Bb, ..) => 'ℝ',
        ('Z', Bb, ..) => 'ℤ',
        ('D', Bb, _, true) => 'ⅅ',
        ('d', Bb, _, true) => 'ⅆ',
        ('e', Bb, _, true) => 'ⅇ',
        ('i', Bb, _, true) => 'ⅈ',
        ('j', Bb, _, true) => 'ⅉ',
        ('h', Serif, false, true) => 'ℎ',
        ('e', Cal, false, _) => 'ℯ',
        ('g', Cal, false, _) => 'ℊ',
        ('o', Cal, false, _) => 'ℴ',
        ('ħ', Serif, .., true) => 'ℏ',
        ('ı', Serif, .., true) => '𝚤',
        ('ȷ', Serif, .., true) => '𝚥',
        _ => return None,
    })
}

fn greek_exception(
    c: char,
    variant: MathVariant,
    bold: bool,
    italic: bool,
) -> Option<char> {
    use MathVariant::*;
    if c == 'Ϝ' && variant == Serif && bold {
        return Some('𝟊');
    }
    if c == 'ϝ' && variant == Serif && bold {
        return Some('𝟋');
    }

    let list = match c {
        'ϴ' => ['𝚹', '𝛳', '𝜭', '𝝧', '𝞡', 'ϴ'],
        '∇' => ['𝛁', '𝛻', '𝜵', '𝝯', '𝞩', '∇'],
        '∂' => ['𝛛', '𝜕', '𝝏', '𝞉', '𝟃', '∂'],
        'ϵ' => ['𝛜', '𝜖', '𝝐', '𝞊', '𝟄', 'ϵ'],
        'ϑ' => ['𝛝', '𝜗', '𝝑', '𝞋', '𝟅', 'ϑ'],
        'ϰ' => ['𝛞', '𝜘', '𝝒', '𝞌', '𝟆', 'ϰ'],
        'ϕ' => ['𝛟', '𝜙', '𝝓', '𝞍', '𝟇', 'ϕ'],
        'ϱ' => ['𝛠', '𝜚', '𝝔', '𝞎', '𝟈', 'ϱ'],
        'ϖ' => ['𝛡', '𝜛', '𝝕', '𝞏', '𝟉', 'ϖ'],
        'Γ' => ['𝚪', '𝛤', '𝜞', '𝝘', '𝞒', 'ℾ'],
        'γ' => ['𝛄', '𝛾', '𝜸', '𝝲', '𝞬', 'ℽ'],
        'Π' => ['𝚷', '𝛱', '𝜫', '𝝥', '𝞟', 'ℿ'],
        'π' => ['𝛑', '𝜋', '𝝅', '𝝿', '𝞹', 'ℼ'],
        '∑' => ['∑', '∑', '∑', '∑', '∑', '⅀'],
        _ => return None,
    };

    Some(match (variant, bold, italic) {
        (Serif, true, false) => list[0],
        (Serif, false, true) => list[1],
        (Serif, true, true) => list[2],
        (Sans, _, false) => list[3],
        (Sans, _, true) => list[4],
        (Bb, ..) => list[5],
        _ => return None,
    })
}

/// Switch dotless character to non dotless character for use of the dtls
/// OpenType feature.
pub fn dtls_char(c: char, dtls: &mut bool) -> char {
    match (c, *dtls) {
        ('ı', true) => 'i',
        ('ȷ', true) => 'j',
        _ => {
            *dtls = false;
            c
        }
    }
}
