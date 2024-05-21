use ecow::EcoString;

use crate::foundations::{elem, Packed, PlainText, Repr, Unlabellable};
use crate::realize::{Behave, Behaviour};

/// A text space.
#[elem(Behave, Unlabellable, PlainText, Repr)]
pub struct SpaceElem {}

impl Repr for SpaceElem {
    fn repr(&self) -> EcoString {
        "[ ]".into()
    }
}

impl Behave for Packed<SpaceElem> {
    fn behaviour(&self) -> Behaviour {
        Behaviour::Weak(2)
    }
}

impl Unlabellable for Packed<SpaceElem> {}

impl PlainText for Packed<SpaceElem> {
    fn plain_text(&self, text: &mut EcoString) {
        text.push(' ');
    }
}
