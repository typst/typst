use ecow::EcoString;
use typst_utils::singleton;

use crate::foundations::{
    Content, NativeElement, Packed, PlainText, Repr, Unlabellable, elem,
};

/// A text space.
#[elem(Unlabellable, PlainText, Repr, PartialEq)]
pub struct SpaceElem {
    #[required]
    pub had_newline: bool,
}

impl PartialEq for SpaceElem {
    fn eq(&self, _other: &Self) -> bool {
        // Note: This is fine for comemo because it only compares based on hash.
        true
    }
}

impl SpaceElem {
    /// Get the globally shared space element.
    pub fn shared() -> &'static Content {
        singleton!(Content, SpaceElem::new(false).pack())
    }

    /// A globally shared space element with `had_newline` set to true.
    pub fn shared_with_newline() -> &'static Content {
        singleton!(Content, SpaceElem::new(true).pack())
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
