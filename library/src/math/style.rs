use super::*;

/// # Bold
/// Bold font style in math.
///
/// ## Example
/// ```
/// $ bold(A) := B^+ $
/// ```
///
/// ## Parameters
/// - body: Content (positional, required)
///   The piece of formula to style.
///
/// ## Category
/// math
#[func]
#[capable(LayoutMath)]
#[derive(Debug, Hash)]
pub struct BoldNode(pub Content);

#[node]
impl BoldNode {
    fn construct(_: &Vm, args: &mut Args) -> SourceResult<Content> {
        Ok(Self(args.expect("body")?).pack())
    }
}

impl LayoutMath for BoldNode {
    fn layout_math(&self, ctx: &mut MathContext) -> SourceResult<()> {
        ctx.style(ctx.style.with_italic(false).with_bold_toggled());
        self.0.layout_math(ctx)?;
        ctx.unstyle();
        Ok(())
    }
}

/// # Italic
/// Italic font style in math.
///
/// This is already the default.
///
/// ## Parameters
/// - body: Content (positional, required)
///   The piece of formula to style.
///
/// ## Category
/// math
#[func]
#[capable(LayoutMath)]
#[derive(Debug, Hash)]
pub struct ItalicNode(pub Content);

#[node]
impl ItalicNode {
    fn construct(_: &Vm, args: &mut Args) -> SourceResult<Content> {
        Ok(Self(args.expect("body")?).pack())
    }
}

impl LayoutMath for ItalicNode {
    fn layout_math(&self, ctx: &mut MathContext) -> SourceResult<()> {
        ctx.style(ctx.style.with_italic_toggled());
        self.0.layout_math(ctx)?;
        ctx.unstyle();
        Ok(())
    }
}

/// # Serif
/// Serif (roman) font style in math.
///
/// This is already the default.
///
/// ## Parameters
/// - body: Content (positional, required)
///   The piece of formula to style.
///
/// ## Category
/// math
#[func]
#[capable(LayoutMath)]
#[derive(Debug, Hash)]
pub struct SerifNode(pub Content);

#[node]
impl SerifNode {
    fn construct(_: &Vm, args: &mut Args) -> SourceResult<Content> {
        Ok(Self(args.expect("body")?).pack())
    }
}

impl LayoutMath for SerifNode {
    fn layout_math(&self, ctx: &mut MathContext) -> SourceResult<()> {
        ctx.style(ctx.style.with_variant(MathVariant::Serif));
        self.0.layout_math(ctx)?;
        ctx.unstyle();
        Ok(())
    }
}

/// # Sans-serif
/// Sans-serif font style in math.
///
/// ## Example
/// ```
/// $ sans(A B C) $
/// ```
///
/// ## Parameters
/// - body: Content (positional, required)
///   The piece of formula to style.
///
/// ## Category
/// math
#[func]
#[capable(LayoutMath)]
#[derive(Debug, Hash)]
pub struct SansNode(pub Content);

#[node]
impl SansNode {
    fn construct(_: &Vm, args: &mut Args) -> SourceResult<Content> {
        Ok(Self(args.expect("body")?).pack())
    }
}

impl LayoutMath for SansNode {
    fn layout_math(&self, ctx: &mut MathContext) -> SourceResult<()> {
        ctx.style(ctx.style.with_variant(MathVariant::Sans));
        self.0.layout_math(ctx)?;
        ctx.unstyle();
        Ok(())
    }
}

/// # Calligraphic
/// Calligraphic font style in math.
///
/// ## Example
/// ```
/// Let $cal(P)$ be the set of ...
/// ```
///
/// ## Parameters
/// - body: Content (positional, required)
///   The piece of formula to style.
///
/// ## Category
/// math
#[func]
#[capable(LayoutMath)]
#[derive(Debug, Hash)]
pub struct CalNode(pub Content);

#[node]
impl CalNode {
    fn construct(_: &Vm, args: &mut Args) -> SourceResult<Content> {
        Ok(Self(args.expect("body")?).pack())
    }
}

impl LayoutMath for CalNode {
    fn layout_math(&self, ctx: &mut MathContext) -> SourceResult<()> {
        ctx.style(ctx.style.with_variant(MathVariant::Cal));
        self.0.layout_math(ctx)?;
        ctx.unstyle();
        Ok(())
    }
}

/// # Fraktur
/// Fraktur font style in math.
///
/// ## Example
/// ```
/// $ frak(P) $
/// ```
///
/// ## Parameters
/// - body: Content (positional, required)
///   The piece of formula to style.
///
/// ## Category
/// math
#[func]
#[capable(LayoutMath)]
#[derive(Debug, Hash)]
pub struct FrakNode(pub Content);

#[node]
impl FrakNode {
    fn construct(_: &Vm, args: &mut Args) -> SourceResult<Content> {
        Ok(Self(args.expect("body")?).pack())
    }
}

impl LayoutMath for FrakNode {
    fn layout_math(&self, ctx: &mut MathContext) -> SourceResult<()> {
        ctx.style(ctx.style.with_variant(MathVariant::Frak));
        self.0.layout_math(ctx)?;
        ctx.unstyle();
        Ok(())
    }
}

/// # Monospace
/// Monospace font style in math.
///
/// ## Example
/// ```
/// $ mono(x + y = z) $
/// ```
///
/// ## Parameters
/// - body: Content (positional, required)
///   The piece of formula to style.
///
/// ## Category
/// math
#[func]
#[capable(LayoutMath)]
#[derive(Debug, Hash)]
pub struct MonoNode(pub Content);

#[node]
impl MonoNode {
    fn construct(_: &Vm, args: &mut Args) -> SourceResult<Content> {
        Ok(Self(args.expect("body")?).pack())
    }
}

impl LayoutMath for MonoNode {
    fn layout_math(&self, ctx: &mut MathContext) -> SourceResult<()> {
        ctx.style(ctx.style.with_variant(MathVariant::Mono));
        self.0.layout_math(ctx)?;
        ctx.unstyle();
        Ok(())
    }
}

/// # Blackboard Bold
/// Blackboard bold (double-struck) font style in math.
///
/// For uppercase latin letters, blackboard bold is additionally available
/// through [symmie symbols](@symbol) of the form `NN` and `RR`.
///
/// ## Example
/// ```
/// $ bb(b) $
/// $ bb(N) = NN $
/// $ f: NN -> RR $
/// ```
///
/// ## Parameters
/// - body: Content (positional, required) The piece of formula to style.
///
/// ## Category
/// math
#[func]
#[capable(LayoutMath)]
#[derive(Debug, Hash)]
pub struct BbNode(pub Content);

#[node]
impl BbNode {
    fn construct(_: &Vm, args: &mut Args) -> SourceResult<Content> {
        Ok(Self(args.expect("body")?).pack())
    }
}

impl LayoutMath for BbNode {
    fn layout_math(&self, ctx: &mut MathContext) -> SourceResult<()> {
        ctx.style(ctx.style.with_variant(MathVariant::Bb));
        self.0.layout_math(ctx)?;
        ctx.unstyle();
        Ok(())
    }
}

/// The style in a formula.
#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub struct MathStyle {
    /// The style variant to select.
    pub variant: MathVariant,
    /// The size of the glyphs.
    pub size: MathSize,
    /// Affects the height of exponents.
    pub cramped: bool,
    /// Whether to use bold glyphs.
    pub bold: bool,
    /// Whether to use italic glyphs.
    pub italic: bool,
}

/// The size of elements in a formula.
///
/// See the TeXbook p. 141.
#[derive(Debug, Copy, Clone, Eq, PartialEq, Ord, PartialOrd)]
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

impl MathStyle {
    /// This style, with the given `variant`.
    pub fn with_variant(self, variant: MathVariant) -> Self {
        Self { variant, ..self }
    }

    /// This style, with the given `size`.
    pub fn with_size(self, size: MathSize) -> Self {
        Self { size, ..self }
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
        Self { italic, ..self }
    }

    /// This style, with boldness inverted.
    pub fn with_bold_toggled(self) -> Self {
        self.with_bold(!self.bold)
    }

    /// This style, with italicness inverted.
    pub fn with_italic_toggled(self) -> Self {
        self.with_italic(!self.italic)
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

/// A mathematical style variant, as defined by Unicode.
#[derive(Debug, Copy, Clone, Eq, PartialEq)]
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

    let tuple = (style.variant, style.bold, style.italic);
    let base = match c {
        'a'..='z' => 'a',
        'A'..='Z' => 'A',
        'α'..='ω' => 'α',
        'Α'..='Ω' => 'Α',
        '0'..='9' => '0',
        '-' => return '−',
        _ => return c,
    };

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

        _ => return c,
    };

    // Map and fix up codepoints that are defined in previous Unicode Blocks.
    let code = start + (c as u32 - base as u32);
    match code {
        0x1D455 => '\u{210E}',
        0x1D49D => '\u{212C}',
        0x1D4A0 => '\u{2130}',
        0x1D4A1 => '\u{2131}',
        0x1D4A3 => '\u{210B}',
        0x1D4A4 => '\u{2110}',
        0x1D4A7 => '\u{2112}',
        0x1D4A8 => '\u{2133}',
        0x1D4AD => '\u{211B}',
        0x1D4BA => '\u{212F}',
        0x1D4BC => '\u{210A}',
        0x1D4C4 => '\u{2134}',
        0x1D506 => '\u{212D}',
        0x1D50B => '\u{210C}',
        0x1D50C => '\u{2111}',
        0x1D515 => '\u{211C}',
        0x1D51D => '\u{2128}',
        0x1D53A => '\u{2102}',
        0x1D53F => '\u{210D}',
        0x1D545 => '\u{2115}',
        0x1D547 => '\u{2119}',
        0x1D548 => '\u{211A}',
        0x1D549 => '\u{211D}',
        0x1D551 => '\u{2124}',
        code => std::char::from_u32(code).unwrap(),
    }
}
