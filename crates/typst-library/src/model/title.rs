use crate::diag::{Hint, HintedStrResult};
use crate::foundations::{Content, Packed, ShowSet, Smart, StyleChain, Styles, elem};
use crate::introspection::Locatable;
use crate::layout::{BlockElem, Em};
use crate::model::DocumentElem;
use crate::text::{FontWeight, TextElem, TextSize};

/// A document title.
///
/// This should be used to display the main title of the whole document and
/// should occur only once per document. In contrast, level 1
/// [headings]($heading) are intended to be used for the top-level sections of
/// the document.
///
/// Note that additional frontmatter (like an author list) that should appear
/// together with the title does not belong in its body.
///
/// In HTML export, this shows as a `h1` element while level 1 headings show
/// as `h2` elements.
///
/// # Example
/// ```example
/// #set document(
///   title: [Interstellar Mail Delivery]
/// )
///
/// #title()
///
/// = Introduction
/// In recent years, ...
/// ```
#[elem(Locatable, ShowSet)]
pub struct TitleElem {
    /// The content of the title.
    ///
    /// When omitted (or `{auto}`), this will default to [`document.title`]. In
    /// this case, a document title must have been previously set with
    /// `{set document(title: [..])}`.
    ///
    /// ```example
    /// #set document(title: "Course ABC, Homework 1")
    /// #title[Homework 1]
    ///
    /// ...
    /// ```
    #[positional]
    pub body: Smart<Content>,
}

impl TitleElem {
    pub fn resolve_body(&self, styles: StyleChain) -> HintedStrResult<Content> {
        match self.body.get_cloned(styles) {
            Smart::Auto => styles
                .get_cloned(DocumentElem::title)
                .ok_or("document title was not set")
                .hint("set the title with `set document(title: [...])`")
                .hint("or provide an explicit body with `title[..]`"),
            Smart::Custom(body) => Ok(body),
        }
    }
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
