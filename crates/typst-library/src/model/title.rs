use crate::foundations::{Content, Packed, ShowSet, Smart, StyleChain, Styles, elem};
use crate::introspection::Locatable;
use crate::layout::{BlockElem, Em};
use crate::text::{FontWeight, TextElem, TextSize};

/// A document title.
///
/// This should be used to display the main title of the whole document and
/// should occur only once per document. In contrast, level 1
/// [headings]($heading) are intended to be used for the top-level sections of
/// the document.
///
/// In HTML export, this shows as a `h1` element while level 1 headings show
/// as `h2` elements.
///
/// # Example
/// ```example
/// #title[Interstellar Mail Delivery]
///
/// = Introduction
/// In recent years, ...
/// ```
#[elem(Locatable, ShowSet)]
pub struct TitleElem {
    /// The content of the title.
    #[required]
    pub body: Content,
}

impl ShowSet for Packed<TitleElem> {
    fn show_set(&self, _styles: StyleChain) -> Styles {
        const SIZE: Em = Em::new(1.6);
        const ABOVE: Em = Em::new(1.125);
        const BELOW: Em = Em::new(0.75);

        let mut out = Styles::new();
        out.set(TextElem::size, TextSize(SIZE.into()));
        out.set(TextElem::weight, FontWeight::BOLD);
        out.set(BlockElem::above, Smart::Custom(ABOVE.into()));
        out.set(BlockElem::below, Smart::Custom(BELOW.into()));
        out.set(BlockElem::sticky, true);
        out
    }
}
