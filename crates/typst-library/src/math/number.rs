use crate::foundations::{Content, NativeElement, Packed, PlainText, Repr, elem};
use crate::math::Mathy;
use ecow::{EcoString, eco_format};

/// A number in math.
///
/// If you want to render a string like a number, you may use function `number`.
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
    /// Create a new packed number element.
    pub fn packed(text: impl Into<EcoString>) -> Content {
        Self::new(text.into()).pack()
    }
}

impl Repr for NumberElem {
    fn repr(&self) -> EcoString {
        eco_format!("[{}]", self.text)
    }
}

impl PlainText for Packed<NumberElem> {
    fn plain_text(&self, text: &mut EcoString) {
        text.push_str(&self.text);
    }
}
