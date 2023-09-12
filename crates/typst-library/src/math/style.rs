use super::*;

/// Bold font style in math.
///
/// ```example
/// $ bold(A) := B^+ $
/// ```
#[func]
pub fn bold(
    /// The content to style.
    body: Content,
) -> Content {
    MathStyleElem::new(body).with_bold(Some(true)).pack()
}

/// Upright (non-italic) font style in math.
///
/// ```example
/// $ upright(A) != A $
/// ```
#[func]
pub fn upright(
    /// The content to style.
    body: Content,
) -> Content {
    MathStyleElem::new(body).with_italic(Some(false)).pack()
}

/// Italic font style in math.
///
/// For roman letters and greek lowercase letters, this is already the default.
#[func]
pub fn italic(
    /// The content to style.
    body: Content,
) -> Content {
    MathStyleElem::new(body).with_italic(Some(true)).pack()
}
/// Serif (roman) font style in math.
///
/// This is already the default.
#[func]
pub fn serif(
    /// The content to style.
    body: Content,
) -> Content {
    MathStyleElem::new(body).with_variant(Some(MathVariant::Serif)).pack()
}

/// Sans-serif font style in math.
///
/// ```example
/// $ sans(A B C) $
/// ```
#[func(title = "Sans Serif")]
pub fn sans(
    /// The content to style.
    body: Content,
) -> Content {
    MathStyleElem::new(body).with_variant(Some(MathVariant::Sans)).pack()
}

/// Calligraphic font style in math.
///
/// ```example
/// Let $cal(P)$ be the set of ...
/// ```
#[func(title = "Calligraphic")]
pub fn cal(
    /// The content to style.
    body: Content,
) -> Content {
    MathStyleElem::new(body).with_variant(Some(MathVariant::Cal)).pack()
}

/// Fraktur font style in math.
///
/// ```example
/// $ frak(P) $
/// ```
#[func(title = "Fraktur")]
pub fn frak(
    /// The content to style.
    body: Content,
) -> Content {
    MathStyleElem::new(body).with_variant(Some(MathVariant::Frak)).pack()
}

/// Monospace font style in math.
///
/// ```example
/// $ mono(x + y = z) $
/// ```
#[func(title = "Monospace")]
pub fn mono(
    /// The content to style.
    body: Content,
) -> Content {
    MathStyleElem::new(body).with_variant(Some(MathVariant::Mono)).pack()
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
#[func(title = "Blackboard Bold")]
pub fn bb(
    /// The content to style.
    body: Content,
) -> Content {
    MathStyleElem::new(body).with_variant(Some(MathVariant::Bb)).pack()
}

/// Forced display style in math.
///
/// This is the normal size for block equations.
///
/// ```example
/// $sum_i x_i/2 = display(sum_i x_i/2)$
/// ```
#[func(title = "Display Size")]
pub fn display(
    /// The content to size.
    body: Content,
    /// Whether to impose a height restriction for exponents, like regular sub-
    /// and superscripts do.
    #[named]
    #[default(false)]
    cramped: bool,
) -> Content {
    MathStyleElem::new(body)
        .with_size(Some(MathSize::Display))
        .with_cramped(Some(cramped))
        .pack()
}

/// Forced inline (text) style in math.
///
/// This is the normal size for inline equations.
///
/// ```example
/// $ sum_i x_i/2
///     = inline(sum_i x_i/2) $
/// ```
#[func(title = "Inline Size")]
pub fn inline(
    /// The content to size.
    body: Content,
    /// Whether to impose a height restriction for exponents, like regular sub-
    /// and superscripts do.
    #[named]
    #[default(false)]
    cramped: bool,
) -> Content {
    MathStyleElem::new(body)
        .with_size(Some(MathSize::Text))
        .with_cramped(Some(cramped))
        .pack()
}

/// Forced script style in math.
///
/// This is the smaller size used in powers or sub- or superscripts.
///
/// ```example
/// $sum_i x_i/2 = script(sum_i x_i/2)$
/// ```
#[func(title = "Script Size")]
pub fn script(
    /// The content to size.
    body: Content,
    /// Whether to impose a height restriction for exponents, like regular sub-
    /// and superscripts do.
    #[named]
    #[default(true)]
    cramped: bool,
) -> Content {
    MathStyleElem::new(body)
        .with_size(Some(MathSize::Script))
        .with_cramped(Some(cramped))
        .pack()
}

/// Forced second script style in math.
///
/// This is the smallest size, used in second-level sub- and superscripts
/// (script of the script).
///
/// ```example
/// $sum_i x_i/2 = sscript(sum_i x_i/2)$
/// ```
#[func(title = "Script-Script Size")]
pub fn sscript(
    /// The content to size.
    body: Content,
    /// Whether to impose a height restriction for exponents, like regular sub-
    /// and superscripts do.
    #[named]
    #[default(true)]
    cramped: bool,
) -> Content {
    MathStyleElem::new(body)
        .with_size(Some(MathSize::ScriptScript))
        .with_cramped(Some(cramped))
        .pack()
}

/// A font variant in math.
#[elem(LayoutMath)]
pub struct MathStyleElem {
    /// The content to style.
    #[required]
    pub body: Content,

    /// The variant to select.
    pub variant: Option<MathVariant>,

    /// Whether to use bold glyphs.
    pub bold: Option<bool>,

    /// Whether to use italic glyphs.
    pub italic: Option<bool>,

    /// Whether to use forced size
    pub size: Option<MathSize>,

    /// Whether to limit height of exponents
    pub cramped: Option<bool>,
}

impl LayoutMath for MathStyleElem {
    #[tracing::instrument(skip(ctx))]
    fn layout_math(&self, ctx: &mut MathContext) -> SourceResult<()> {
        let mut style = ctx.style;
        if let Some(variant) = self.variant(StyleChain::default()) {
            style = style.with_variant(variant);
        }
        if let Some(bold) = self.bold(StyleChain::default()) {
            style = style.with_bold(bold);
        }
        if let Some(italic) = self.italic(StyleChain::default()) {
            style = style.with_italic(italic);
        }
        if let Some(size) = self.size(StyleChain::default()) {
            style = style.with_size(size);
        }
        if let Some(cramped) = self.cramped(StyleChain::default()) {
            style = style.with_cramped(cramped);
        }
        ctx.style(style);
        self.body().layout_math(ctx)?;
        ctx.unstyle();
        Ok(())
    }
}

/// Text properties in math.
#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub struct MathStyle {
    /// The style variant to select.
    pub variant: MathVariant,
    /// The size of the glyphs.
    pub size: MathSize,
    /// The class of the element.
    pub class: Smart<MathClass>,
    /// Affects the height of exponents.
    pub cramped: bool,
    /// Whether to use bold glyphs.
    pub bold: bool,
    /// Whether to use italic glyphs.
    pub italic: Smart<bool>,
}

impl MathStyle {
    /// This style, with the given `variant`.
    pub fn with_variant(self, variant: MathVariant) -> Self {
        Self { variant, ..self }
    }

    /// This style, with the given `size`.
    pub fn with_size(self, size: MathSize) -> Self {
        Self { size, ..self }
    }

    // This style, with the given `class`.
    pub fn with_class(self, class: MathClass) -> Self {
        Self { class: Smart::Custom(class), ..self }
    }

    /// This style, with `cramped` set to the given value.
    pub fn with_cramped(self, cramped: bool) -> Self {
        Self { cramped, ..self }
    }

    /// This style, with `bold` set to the given value.
    pub fn with_bold(self, bold: bool) -> Self {
        Self { bold, ..self }
    }

    /// This style, with `italic` set to the given value.
    pub fn with_italic(self, italic: bool) -> Self {
        Self { italic: Smart::Custom(italic), ..self }
    }

    /// The style for subscripts in the current style.
    pub fn for_subscript(self) -> Self {
        self.for_superscript().with_cramped(true)
    }

    /// The style for superscripts in the current style.
    pub fn for_superscript(self) -> Self {
        self.with_size(match self.size {
            MathSize::Display | MathSize::Text => MathSize::Script,
            MathSize::Script | MathSize::ScriptScript => MathSize::ScriptScript,
        })
    }

    /// The style for numerators in the current style.
    pub fn for_numerator(self) -> Self {
        self.with_size(match self.size {
            MathSize::Display => MathSize::Text,
            MathSize::Text => MathSize::Script,
            MathSize::Script | MathSize::ScriptScript => MathSize::ScriptScript,
        })
    }

    /// The style for denominators in the current style.
    pub fn for_denominator(self) -> Self {
        self.for_numerator().with_cramped(true)
    }

    /// Apply the style to a character.
    pub fn styled_char(self, c: char) -> char {
        styled_char(self, c)
    }
}

/// The size of elements in an equation.
///
/// See the TeXbook p. 141.
#[derive(Debug, Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Cast)]
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
    pub(super) fn factor(self, ctx: &MathContext) -> f64 {
        match self {
            Self::Display | Self::Text => 1.0,
            Self::Script => percent!(ctx, script_percent_scale_down),
            Self::ScriptScript => percent!(ctx, script_script_percent_scale_down),
        }
    }
}

/// A mathematical style variant, as defined by Unicode.
#[derive(Debug, Copy, Clone, Eq, PartialEq, Cast)]
pub enum MathVariant {
    Serif,
    Sans,
    Cal,
    Frak,
    Mono,
    Bb,
}

impl Default for MathVariant {
    fn default() -> Self {
        Self::Serif
    }
}

/// Select the correct styled math letter.
///
/// https://www.w3.org/TR/mathml-core/#new-text-transform-mappings
/// https://en.wikipedia.org/wiki/Mathematical_Alphanumeric_Symbols
pub(super) fn styled_char(style: MathStyle, c: char) -> char {
    use MathVariant::*;

    let MathStyle { variant, bold, .. } = style;
    let italic = style.italic.unwrap_or(matches!(
        c,
        'a'..='z' | 'ı' | 'ȷ' | 'A'..='Z' | 'α'..='ω' |
        '∂' | 'ϵ' | 'ϑ' | 'ϰ' | 'ϕ' | 'ϱ' | 'ϖ'
    ));

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
        'ϴ' => ['𝚹', '𝛳', '𝜭', '𝝧', '𝞡'],
        '∇' => ['𝛁', '𝛻', '𝜵', '𝝯', '𝞩'],
        '∂' => ['𝛛', '𝜕', '𝝏', '𝞉', '𝟃'],
        'ϵ' => ['𝛜', '𝜖', '𝝐', '𝞊', '𝟄'],
        'ϑ' => ['𝛝', '𝜗', '𝝑', '𝞋', '𝟅'],
        'ϰ' => ['𝛞', '𝜘', '𝝒', '𝞌', '𝟆'],
        'ϕ' => ['𝛟', '𝜙', '𝝓', '𝞍', '𝟇'],
        'ϱ' => ['𝛠', '𝜚', '𝝔', '𝞎', '𝟈'],
        'ϖ' => ['𝛡', '𝜛', '𝝕', '𝞏', '𝟉'],
        _ => return None,
    };

    Some(match (variant, bold, italic) {
        (Serif, true, false) => list[0],
        (Serif, false, true) => list[1],
        (Serif, true, true) => list[2],
        (Sans, _, false) => list[3],
        (Sans, _, true) => list[4],
        _ => return None,
    })
}
