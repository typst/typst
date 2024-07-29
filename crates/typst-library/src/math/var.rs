use ecow::{eco_format, EcoString};

use crate::foundations::{elem, Content, NativeElement, Repr};
use crate::math::Mathy;

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
#[elem(title = "Math Variable", Mathy, Repr)]
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

impl Repr for VarElem {
    /// Use a custom repr to reduce noise in the output. This elides the "text: " part,
    /// uses `var` instead of `math.var`, and surrounds the repr with dollar signs.
    ///
    /// This looks like: `$var("x")$` instead of `math.var(text: "x")`.
    ///
    /// TODO: It would be nice if we could simplify the repr to just dollar signs if we're
    /// only representing a single character (i.e. `$a$`). But there would be issues with
    /// problematic characters like hashtag/dollar sign/backslash/shorthands which would
    /// need to be escaped. It would be a really nice repr though so maybe it's worth the
    /// manual effort to check for those?
    fn repr(&self) -> EcoString {
        eco_format!("[{}]", self.text)
    }
}
