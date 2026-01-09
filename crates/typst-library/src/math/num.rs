use crate::foundations::{Content, NativeElement, Packed, PlainText, Repr, elem};
use crate::math::Mathy;
use ecow::{EcoString, eco_format};

/// A number in math.
///
/// If you want to render a string like a number, you may use function `num`.
///
/// # Example
/// ```example
/// #show math.num: set text(red)
/// $ "2.1", 2.1, num("1,000.01") $
/// ```
#[elem(title = "Number", Mathy, Repr, PlainText)]
pub struct NumElem {
    /// The number.
    #[required]
    pub text: EcoString,
}

impl NumElem {
    /// Create a new packed number element.
    pub fn packed(text: impl Into<EcoString>) -> Content {
        Self::new(text.into()).pack()
    }
}

impl Repr for NumElem {
    fn repr(&self) -> EcoString {
        eco_format!("[{}]", self.text)
    }
}

impl PlainText for Packed<NumElem> {
    fn plain_text(&self, text: &mut EcoString) {
        text.push_str(&self.text);
    }
}
