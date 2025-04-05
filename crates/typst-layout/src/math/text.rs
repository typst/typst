use std::f64::consts::SQRT_2;

use ecow::EcoString;
use typst_library::diag::SourceResult;
use typst_library::foundations::{Packed, StyleChain, SymbolElem};
use typst_library::layout::{Abs, Size};
use typst_library::math::{EquationElem, MathSize, MathStyle, MathVariant};
use typst_library::text::{
    BottomEdge, BottomEdgeMetric, TextElem, TopEdge, TopEdgeMetric,
};
use typst_syntax::{is_newline, Span};
use unicode_math_class::MathClass;
use unicode_segmentation::UnicodeSegmentation;

use super::{FrameFragment, GlyphFragment, MathContext, MathFragment, MathRun};

/// Lays out a [`TextElem`].
pub fn layout_text(
    elem: &Packed<TextElem>,
    ctx: &mut MathContext,
    styles: StyleChain,
) -> SourceResult<()> {
    let text = &elem.text;
    let span = elem.span();
    let fragment = if text.contains(is_newline) {
        layout_text_lines(text.split(is_newline), span, ctx, styles)?
    } else {
        layout_inline_text(text, span, ctx, styles)?
    };
    ctx.push(fragment);
    Ok(())
}

/// Layout multiple lines of text.
fn layout_text_lines<'a>(
    lines: impl Iterator<Item = &'a str>,
    span: Span,
    ctx: &mut MathContext,
    styles: StyleChain,
) -> SourceResult<FrameFragment> {
    let mut fragments = vec![];
    for (i, line) in lines.enumerate() {
        if i != 0 {
            fragments.push(MathFragment::Linebreak);
        }
        if !line.is_empty() {
            fragments.push(layout_inline_text(line, span, ctx, styles)?.into());
        }
    }
    let mut frame = MathRun::new(fragments).into_frame(styles);
    let axis = scaled!(ctx, styles, axis_height);
    frame.set_baseline(frame.height() / 2.0 + axis);
    Ok(FrameFragment::new(styles, frame))
}

/// Layout the given text string into a [`FrameFragment`] after styling all
/// characters for the math font (without auto-italics).
fn layout_inline_text(
    text: &str,
    span: Span,
    ctx: &mut MathContext,
    styles: StyleChain,
) -> SourceResult<FrameFragment> {
    if text.chars().all(|c| c.is_ascii_digit() || c == '.') {
        // Small optimization for numbers. Note that this lays out slightly
        // differently to normal text and is worth re-evaluating in the future.
        let mut fragments = vec![];
        let is_single = text.chars().count() == 1;
        for unstyled_c in text.chars() {
            let c = styled_char(styles, unstyled_c, false);
            let mut glyph = GlyphFragment::new(ctx, styles, c, span);
            if is_single {
                // Duplicate what `layout_glyph` does exactly even if it's
                // probably incorrect here.
                match EquationElem::size_in(styles) {
                    MathSize::Script => glyph.make_script_size(ctx),
                    MathSize::ScriptScript => glyph.make_script_script_size(ctx),
                    _ => {}
                }
            }
            fragments.push(glyph.into());
        }
        let frame = MathRun::new(fragments).into_frame(styles);
        Ok(FrameFragment::new(styles, frame).with_text_like(true))
    } else {
        let local = [
            TextElem::set_top_edge(TopEdge::Metric(TopEdgeMetric::Bounds)),
            TextElem::set_bottom_edge(BottomEdge::Metric(BottomEdgeMetric::Bounds)),
        ]
        .map(|p| p.wrap());

        let styles = styles.chain(&local);
        let styled_text: EcoString =
            text.chars().map(|c| styled_char(styles, c, false)).collect();

        let spaced = styled_text.graphemes(true).nth(1).is_some();
        let elem = TextElem::packed(styled_text).spanned(span);

        // There isn't a natural width for a paragraph in a math environment;
        // because it will be placed somewhere probably not at the left margin
        // it will overflow. So emulate an `hbox` instead and allow the
        // paragraph to extend as far as needed.
        let frame = crate::inline::layout_inline(
            ctx.engine,
            &[(&elem, styles)],
            &mut ctx.locator.next(&span).split(),
            styles,
            Size::splat(Abs::inf()),
            false,
        )?
        .into_frame();

        Ok(FrameFragment::new(styles, frame)
            .with_class(MathClass::Alphabetic)
            .with_text_like(true)
            .with_spaced(spaced))
    }
}

/// Layout a single character in the math font with the correct styling applied
/// (includes auto-italics).
pub fn layout_symbol(
    elem: &Packed<SymbolElem>,
    ctx: &mut MathContext,
    styles: StyleChain,
) -> SourceResult<()> {
    // Switch dotless char to normal when we have the dtls OpenType feature.
    // This should happen before the main styling pass.
    let (unstyled_c, dtls) = match try_dotless(elem.text) {
        Some(c) if ctx.dtls_table.is_some() => (c, true),
        _ => (elem.text, false),
    };
    let c = styled_char(styles, unstyled_c, true);
    let fragment = match GlyphFragment::try_new(ctx, styles, c, elem.span()) {
        Some(glyph) => layout_glyph(glyph, dtls, ctx, styles),
        None => {
            // Not in the math font, fallback to normal inline text layout.
            layout_inline_text(c.encode_utf8(&mut [0; 4]), elem.span(), ctx, styles)?
                .into()
        }
    };
    ctx.push(fragment);
    Ok(())
}

/// Layout a [`GlyphFragment`].
fn layout_glyph(
    mut glyph: GlyphFragment,
    dtls: bool,
    ctx: &mut MathContext,
    styles: StyleChain,
) -> MathFragment {
    if dtls {
        glyph.make_dotless_form(ctx);
    }
    let math_size = EquationElem::size_in(styles);
    match math_size {
        MathSize::Script => glyph.make_script_size(ctx),
        MathSize::ScriptScript => glyph.make_script_script_size(ctx),
        _ => {}
    }

    if glyph.class == MathClass::Large {
        let mut variant = if math_size == MathSize::Display {
            let height = scaled!(ctx, styles, display_operator_min_height)
                .max(SQRT_2 * glyph.height());
            glyph.stretch_vertical(ctx, height, Abs::zero())
        } else {
            glyph.into_variant()
        };
        // TeXbook p 155. Large operators are always vertically centered on the
        // axis.
        variant.center_on_axis(ctx);
        variant.into()
    } else {
        glyph.into()
    }
}

/// Style the character by selecting the unicode codepoint for italic, bold,
/// caligraphic, etc.
///
/// <https://www.w3.org/TR/mathml-core/#new-text-transform-mappings>
/// <https://en.wikipedia.org/wiki/Mathematical_Alphanumeric_Symbols>
fn styled_char(styles: StyleChain, c: char, auto_italic: bool) -> char {
    use MathVariant::*;

    let variant = EquationElem::variant_in(styles);
    let bold = EquationElem::bold_in(styles);
    let default_style = EquationElem::default_style_in(styles);
    let default_italic = match c {
        'a'..='z' | 'ħ' | 'ı' | 'ȷ' | 'α'..='ω' | 'ϵ' | 'ϑ' | 'ϰ' | 'ϕ' | 'ϱ' | 'ϖ' =>
        {
            matches!(default_style, MathStyle::Iso | MathStyle::Tex | MathStyle::French)
        }
        'A'..='Z' => matches!(default_style, MathStyle::Iso | MathStyle::Tex),
        'Α'..='Ω' => matches!(default_style, MathStyle::Iso),
        '∂' => matches!(default_style, MathStyle::Tex | MathStyle::French),
        _ => false,
    };
    let italic = EquationElem::italic_in(styles)
        .unwrap_or(auto_italic && default_italic && matches!(variant, Sans | Serif));

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

/// The non-dotless version of a dotless character that can be used with the
/// `dtls` OpenType feature.
pub fn try_dotless(c: char) -> Option<char> {
    match c {
        'ı' => Some('i'),
        'ȷ' => Some('j'),
        _ => None,
    }
}
