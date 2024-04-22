use crate::diag::SourceResult;
use crate::engine::Engine;
use crate::foundations::{elem, Content, Packed, Show, StyleChain};
use crate::text::TextElem;

/// Displays text in small capitals.
///
/// _Note:_ This enables the OpenType `smcp` feature for the font. Not all fonts
/// support this feature. Sometimes smallcaps are part of a dedicated font and
/// sometimes they are not available at all. In the future, this function will
/// support selecting a dedicated smallcaps font as well as synthesizing
/// smallcaps from normal letters, but this is not yet implemented.
///
/// # Example
/// ```example
/// #set par(justify: true)
/// #set heading(numbering: "I.")
///
/// #show heading: it => {
///   set block(below: 10pt)
///   set text(weight: "regular")
///   align(center, smallcaps(it))
/// }
///
/// = Introduction
/// #lorem(40)
/// ```
#[elem(title = "Small Capitals", Show)]
pub struct SmallcapsElem {
    #[required]
    pub body: Content,
}

impl Show for Packed<SmallcapsElem> {
    #[typst_macros::time(name = "smallcaps", span = self.span())]
    fn show(&self, _: &mut Engine, _: StyleChain) -> SourceResult<Content> {
        Ok(self.body().clone().styled(TextElem::set_smallcaps(true)))
    }
}
