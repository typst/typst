use crate::foundations::{Content, NativeElement, Packed, PlainText, Repr, elem};
use crate::math::Mathy;
use ecow::{EcoString, eco_format};

/// A number in math, potentially containing multiple digits and a decimal point.
///
/// # Example
/// ```example
/// #show math.number: set text(red)
/// $ 2.1 "2.1" $
/// ```
#[elem(Mathy, Repr, PlainText)]
pub struct NumberElem {
    /// The number's text.
    #[required]
    pub text: EcoString, // This is called `text` for consistency with `TextElem`.
}

impl NumberElem {
    /// Create a new packed symbol element.
    pub fn packed(text: impl Into<EcoString>) -> Content {
        Self::new(text.into()).pack()
    }
}

impl PlainText for Packed<NumberElem> {
    fn plain_text(&self, text: &mut EcoString) {
        text.push_str(&self.text);
    }
}

impl Repr for NumberElem {
    /// Use a custom repr that matches normal content.
    fn repr(&self) -> EcoString {
        eco_format!("[{}]", self.text)
    }
}
