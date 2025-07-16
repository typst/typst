use ecow::EcoString;
use typst_utils::singleton;

use crate::foundations::{
    Content, NativeElement, Packed, PlainText, Repr, Unlabellable, elem,
};
use crate::introspection::Unlocatable;

/// A text space.
#[elem(Unlabellable, PlainText, Repr, Unlocatable)]
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

impl Unlocatable for Packed<SpaceElem> {}

impl PlainText for Packed<SpaceElem> {
    fn plain_text(&self, text: &mut EcoString) {
        text.push(' ');
    }
}
