use super::*;

/// Mathematical text.
///
/// Displays variables, symbols and other text as mathematics
/// rather than ordinary text.
///
/// ## Example { #example }
///
/// ```example
/// #set text(fill:blue)
/// #show math.var: set text(fill:green)
/// // Using dedicated syntax.
/// $ e^(pi i) + 1 = 0 $
///
/// // Ordinary text in a formula
/// // appears in double quotes.
/// $ a < b "iff" b > a $
///
/// // Mathematical text with more than
/// // one character is upright by default.
/// $ var("foo") eq.not f o o eq.not "foo" $
///
/// $ italic(var("slanted")) $
/// ```
///
/// ## Syntax { #syntax }
/// Typst automatically creates mathematical text
/// from single letters, numbers and [symbols]($category/symbols/)
/// appearing in a formula.
///
/// Display: Var
/// Category: math
#[element(LayoutMath)]
pub struct VarElem {
    /// The text.
    #[required]
    pub text: EcoString,

    /// A prioritized sequence of font families.
    ///
    #[default(FontList(vec![FontFamily::new("New Computer Modern Math")]))]
    pub font: FontList,

    /// The weight of the base family
    /// FIXME: I think this shoudl be base-weight so that
    /// someone doesn't think that it makes the ordinary font bold
    #[default(FontWeight::from_number(450))]
    pub weight: FontWeight,

    /// The size of the glyphs. This value forms the basis of the `em` unit:
    /// `{1em}` is equivalent to the font size.
    ///
    /// You can also give the font size itself in `em` units. Then, it is
    /// relative to the previous font size.
    ///
    /// ```example
    /// #set text(size: 20pt)
    /// very #text(1.5em)[big] text
    /// ```
    // FIXME: Does this parse really work in the presence of auto?
    // FIXME: Does fold work?
    #[parse(args.named_or_find("size")?)]
    #[fold]
    #[default(Smart::Auto)]
    pub size: Smart<TextSize>,

    /// The glyph fill color.
    ///
    /// ```example
    /// #set text(fill: red)
    /// This text is red.
    /// ```
    /// FIXME: Does this parse still work?
    #[parse(args.named_or_find("fill")?)]
    #[default(Smart::Auto)]
    pub fill: Smart<Paint>,

    /// Whether to allow last resort font fallback when the primary font list
    /// contains no match. If a matching font cannot be found, an error is flagged.
    /// If a font is found but a glyph is not available, it is replaced
    /// with a "tofu", a placeholder box.
    #[default(true)]
    pub fallback: bool,
}

impl VarElem {
    /// Create a new packed symbols element.
    pub fn packed(text: impl Into<EcoString>) -> Content {
        Self::new(text.into()).pack()
    }
}

impl LayoutMath for VarElem {
    #[tracing::instrument(skip(ctx))]
    fn layout_math(&self, ctx: &mut MathContext) -> SourceResult<()> {
        let fragment = ctx.layout_var(self)?;
        ctx.push(fragment);
        Ok(())
    }
}

impl<T> From<T> for VarElem
where
    T: Into<EcoString>,
{
    fn from(item: T) -> Self {
        VarElem::new(item.into())
    }
}
