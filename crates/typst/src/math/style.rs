use crate::foundations::{func, Cast, Content, Smart, Style, StyleChain};
use crate::layout::Abs;
use crate::math::{EquationElem, MathContext};
use crate::text::TextElem;
use crate::utils::LazyHash;

/// Bold font style in math.
///
/// ```example
/// $ bold(A) := B^+ $
/// ```
#[func(keywords = ["mathbf"])]
pub fn bold(
    /// The content to style.
    body: Content,
) -> Content {
    body.styled(EquationElem::set_bold(true))
}

/// Upright (non-italic) font style in math.
///
/// ```example
/// $ upright(A) != A $
/// ```
#[func(keywords = ["mathup"])]
pub fn upright(
    /// The content to style.
    body: Content,
) -> Content {
    body.styled(EquationElem::set_italic(Smart::Custom(false)))
}

/// Italic font style in math.
///
/// For roman letters and greek lowercase letters, this is already the default.
#[func(keywords = ["mathit"])]
pub fn italic(
    /// The content to style.
    body: Content,
) -> Content {
    body.styled(EquationElem::set_italic(Smart::Custom(true)))
}

/// Serif (roman) font style in math.
///
/// This is already the default.
#[func(keywords = ["mathrm"])]
pub fn serif(
    /// The content to style.
    body: Content,
) -> Content {
    body.styled(EquationElem::set_variant(MathVariant::Serif))
}

/// Sans-serif font style in math.
///
/// ```example
/// $ sans(A B C) $
/// ```
#[func(title = "Sans Serif", keywords = ["mathsf"])]
pub fn sans(
    /// The content to style.
    body: Content,
) -> Content {
    body.styled(EquationElem::set_variant(MathVariant::Sans))
}

/// Calligraphic font style in math.
///
/// ```example
/// Let $cal(P)$ be the set of ...
/// ```
#[func(title = "Calligraphic", keywords = ["mathcal"])]
pub fn cal(
    /// The content to style.
    body: Content,
) -> Content {
    body.styled(EquationElem::set_variant(MathVariant::Cal))
}

/// Fraktur font style in math.
///
/// ```example
/// $ frak(P) $
/// ```
#[func(title = "Fraktur", keywords = ["mathfrak"])]
pub fn frak(
    /// The content to style.
    body: Content,
) -> Content {
    body.styled(EquationElem::set_variant(MathVariant::Frak))
}

/// Monospace font style in math.
///
/// ```example
/// $ mono(x + y = z) $
/// ```
#[func(title = "Monospace", keywords = ["mathtt"])]
pub fn mono(
    /// The content to style.
    body: Content,
) -> Content {
    body.styled(EquationElem::set_variant(MathVariant::Mono))
}

/// Blackboard bold (double-struck) font style in math.
///
/// For uppercase latin letters, blackboard bold is additionally available
/// through [symbols]($category/symbols/sym) of the form `NN` and `RR`.
///
/// ```example
/// $ bb(b) $
/// $ bb(N) = NN $
/// $ f: NN -> RR $
/// ```
#[func(title = "Blackboard Bold", keywords = ["mathbb"])]
pub fn bb(
    /// The content to style.
    body: Content,
) -> Content {
    body.styled(EquationElem::set_variant(MathVariant::Bb))
}

/// Forced display style in math.
///
/// This is the normal size for block equations.
///
/// ```example
/// $sum_i x_i/2 = display(sum_i x_i/2)$
/// ```
#[func(title = "Display Size", keywords = ["displaystyle"])]
pub fn display(
    /// The content to size.
    body: Content,
    /// Whether to impose a height restriction for exponents, like regular sub-
    /// and superscripts do.
    #[named]
    #[default(false)]
    cramped: bool,
) -> Content {
    body.styled(EquationElem::set_size(MathSize::Display))
        .styled(EquationElem::set_cramped(cramped))
}

/// Forced inline (text) style in math.
///
/// This is the normal size for inline equations.
///
/// ```example
/// $ sum_i x_i/2
///     = inline(sum_i x_i/2) $
/// ```
#[func(title = "Inline Size", keywords = ["textstyle"])]
pub fn inline(
    /// The content to size.
    body: Content,
    /// Whether to impose a height restriction for exponents, like regular sub-
    /// and superscripts do.
    #[named]
    #[default(false)]
    cramped: bool,
) -> Content {
    body.styled(EquationElem::set_size(MathSize::Text))
        .styled(EquationElem::set_cramped(cramped))
}

/// Forced script style in math.
///
/// This is the smaller size used in powers or sub- or superscripts.
///
/// ```example
/// $sum_i x_i/2 = script(sum_i x_i/2)$
/// ```
#[func(title = "Script Size", keywords = ["scripstyle"])]
pub fn script(
    /// The content to size.
    body: Content,
    /// Whether to impose a height restriction for exponents, like regular sub-
    /// and superscripts do.
    #[named]
    #[default(true)]
    cramped: bool,
) -> Content {
    body.styled(EquationElem::set_size(MathSize::Script))
        .styled(EquationElem::set_cramped(cramped))
}

/// Forced second script style in math.
///
/// This is the smallest size, used in second-level sub- and superscripts
/// (script of the script).
///
/// ```example
/// $sum_i x_i/2 = sscript(sum_i x_i/2)$
/// ```
#[func(title = "Script-Script Size", keywords = ["scriptscriptstyle"])]
pub fn sscript(
    /// The content to size.
    body: Content,
    /// Whether to impose a height restriction for exponents, like regular sub-
    /// and superscripts do.
    #[named]
    #[default(true)]
    cramped: bool,
) -> Content {
    body.styled(EquationElem::set_size(MathSize::ScriptScript))
        .styled(EquationElem::set_cramped(cramped))
}

/// The size of elements in an equation.
///
/// See the TeXbook p. 141.
#[derive(Debug, Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Cast, Hash)]
pub enum MathSize {
    /// Second-level sub- and superscripts.
    ScriptScript,
    /// Sub- and superscripts.
    Script,
    /// Math in text.
    Text,
    /// Math on its own line.
    Display,
}

impl MathSize {
    /// The scaling factor.
    pub fn factor(self, ctx: &MathContext) -> f64 {
        match self {
            Self::Display | Self::Text => 1.0,
            Self::Script => percent!(ctx, script_percent_scale_down),
            Self::ScriptScript => percent!(ctx, script_script_percent_scale_down),
        }
    }
}

/// A mathematical style variant, as defined by Unicode.
#[derive(Debug, Default, Copy, Clone, Eq, PartialEq, Cast, Hash)]
pub enum MathVariant {
    #[default]
    Serif,
    Sans,
    Cal,
    Frak,
    Mono,
    Bb,
}

/// Get the font size scaled with the `MathSize`.
pub fn scaled_font_size(ctx: &MathContext, styles: StyleChain) -> Abs {
    EquationElem::size_in(styles).factor(ctx) * TextElem::size_in(styles)
}

/// Styles something as cramped.
pub fn style_cramped() -> LazyHash<Style> {
    EquationElem::set_cramped(true).wrap()
}

/// The style for subscripts in the current style.
pub fn style_for_subscript(styles: StyleChain) -> [LazyHash<Style>; 2] {
    [style_for_superscript(styles), EquationElem::set_cramped(true).wrap()]
}

/// The style for superscripts in the current style.
pub fn style_for_superscript(styles: StyleChain) -> LazyHash<Style> {
    EquationElem::set_size(match EquationElem::size_in(styles) {
        MathSize::Display | MathSize::Text => MathSize::Script,
        MathSize::Script | MathSize::ScriptScript => MathSize::ScriptScript,
    })
    .wrap()
}

/// The style for numerators in the current style.
pub fn style_for_numerator(styles: StyleChain) -> LazyHash<Style> {
    EquationElem::set_size(match EquationElem::size_in(styles) {
        MathSize::Display => MathSize::Text,
        MathSize::Text => MathSize::Script,
        MathSize::Script | MathSize::ScriptScript => MathSize::ScriptScript,
    })
    .wrap()
}

/// The style for denominators in the current style.
pub fn style_for_denominator(styles: StyleChain) -> [LazyHash<Style>; 2] {
    [style_for_numerator(styles), EquationElem::set_cramped(true).wrap()]
}

/// Select the correct styled math letter.
///
/// <https://www.w3.org/TR/mathml-core/#new-text-transform-mappings>
/// <https://en.wikipedia.org/wiki/Mathematical_Alphanumeric_Symbols>
pub fn styled_char(styles: StyleChain, c: char, auto_italic: bool) -> char {
    use MathVariant::*;

    let variant = EquationElem::variant_in(styles);
    let bold = EquationElem::bold_in(styles);
    let italic = EquationElem::italic_in(styles).unwrap_or(
        auto_italic
            && matches!(
                c,
                'a'..='z' | 'ı' | 'ȷ' | 'A'..='Z' | 'α'..='ω' |
                '∂' | 'ϵ' | 'ϑ' | 'ϰ' | 'ϕ' | 'ϱ' | 'ϖ'
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
