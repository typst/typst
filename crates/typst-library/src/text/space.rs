use ecow::EcoString;
use typst_utils::singleton;

use crate::foundations::{
    elem, Content, NativeElement, Packed, PlainText, Repr, Unlabellable,
};

/// A text space.
#[elem(Unlabellable, PlainText, Repr)]
pub struct SpaceElem {}

impl SpaceElem {
    /// Get the globally shared space element.
    pub fn shared() -> &'static Content {
        singleton!(Content, SpaceElem::new().pack())
    }
}

impl Repr for SpaceElem {
    fn repr(&self) -> EcoString {
        "[ ]".into()
    }
}

impl Unlabellable for Packed<SpaceElem> {}

impl PlainText for Packed<SpaceElem> {
    fn plain_text(&self, text: &mut EcoString) {
        text.push(' ');
    }
}
