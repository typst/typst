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
