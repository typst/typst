use ecow::EcoString;

use crate::diag::SourceResult;
use crate::foundations::{elem, Content, NativeElement, Packed, StyleChain};
use crate::math::{LayoutMath, MathContext};

/// Variables and other characters in math typeset in the math font.
///
/// TODO: basic example
///
/// TODO: mention implicit creation in math via syntax. I.e. `$a$`
///
/// TODO: example with show-set rule to set math font
/// ```example
/// #show math.var: set text(font: "...")
/// ```
#[elem(title = "Math Variable", LayoutMath)]
pub struct VarElem {
    /// The variable's text.
    #[required]
    pub text: EcoString,
}

impl VarElem {
    /// Create a new packed `math.var` element.
    pub fn packed(text: impl Into<EcoString>) -> Content {
        Self::new(text.into()).pack()
    }
}

impl LayoutMath for Packed<VarElem> {
    #[typst_macros::time(name = "math.var", span = self.span())]
    fn layout_math(&self, ctx: &mut MathContext, styles: StyleChain) -> SourceResult<()> {
        ctx.layout_math_variable(self, styles)
    }
}
