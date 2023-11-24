use crate::foundations::{elem, Behave, Behaviour, PlainText, Repr, Unlabellable};
use ecow::EcoString;

/// A text space.
#[elem(Behave, Unlabellable, PlainText, Repr)]
pub struct SpaceElem {}

impl Repr for SpaceElem {
    fn repr(&self) -> EcoString {
        EcoString::inline("[ ]")
    }
}

impl Behave for SpaceElem {
    fn behaviour(&self) -> Behaviour {
        Behaviour::Weak(2)
    }
}

impl Unlabellable for SpaceElem {}

impl PlainText for SpaceElem {
    fn plain_text(&self, text: &mut EcoString) {
        text.push(' ');
    }
}
