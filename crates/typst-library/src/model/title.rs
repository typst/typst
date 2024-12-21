use crate::foundations::{Content, Packed, ShowSet, Smart, StyleChain, Styles, elem};
use crate::introspection::Locatable;
use crate::layout::{AlignElem, Alignment, BlockElem, Em};
use crate::text::{FontWeight, TextElem, TextSize};

/// A document title.
///
/// Should be used to display the main title of the whole document, and should
/// occur only once per document.
///
/// Shows as `h1` in HTML. In contrast, a heading of level 1
/// (created with `= Some Heading`) will show as `h2`.
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
        out.set(AlignElem::alignment, Alignment::CENTER);
        out
    }
}
