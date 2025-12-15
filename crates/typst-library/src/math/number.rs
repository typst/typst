use crate::foundations::{Content, NativeElement, Packed, PlainText, Repr, elem};
use crate::math::Mathy;
use ecow::{EcoString, eco_format};

/// A number in math.
///
/// A number is made of one or more ASCII digits and a possibly a decimal
/// point in the middle.
///
/// If you want to make a string that doesn't fit the above definition
/// to be rendered like a number, you may use function `number()`.
///
/// # Example
/// ```example
/// #show math.number: set text(red)
/// $ "2.1", 2.1, number("1,000.01") $
/// ```
#[elem(Mathy, Repr, PlainText)]
pub struct NumberElem {
    /// The number.
    #[required]
    pub text: EcoString,
}

impl NumberElem {
    /// Create a new packed symbol element.
    pub fn packed(text: impl Into<EcoString>) -> Content {
        Self::new(text.into()).pack()
    }
}

impl Repr for NumberElem {
    /// Use a custom repr that matches normal content.
    fn repr(&self) -> EcoString {
        eco_format!("[{}]", self.text)
    }
}

impl PlainText for Packed<NumberElem> {
    fn plain_text(&self, text: &mut EcoString) {
        text.push_str(&self.text);
    }
}
