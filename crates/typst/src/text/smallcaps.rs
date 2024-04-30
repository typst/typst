use crate::diag::SourceResult;
use crate::engine::Engine;
use crate::foundations::{elem, Content, Packed, Show, StyleChain};
use crate::text::TextElem;

/// Displays text in small capitals.
///
/// _Note:_ This enables the OpenType `smcp` feature for the font. Not all fonts
/// support this feature. Sometimes smallcaps are part of a dedicated font and
/// sometimes they are not available at all. In the future, this function will
/// support synthesizing smallcaps from normal letters, but this is not yet
/// implemented.
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
///
/// You can use show rule to customize the appearance of the text in smallcaps,
/// which allows selecting a dedicated smallcaps font.
/// ```example
/// #show smallcaps: set text(font: "linux libertine")
/// #smallcaps[Use Show Rule For Smallcaps]
/// ```
#[elem(title = "Small Capitals", Show)]
pub struct SmallcapsElem {
    /// The content to display to small capitals.
    #[required]
    pub body: Content,
}

impl Show for Packed<SmallcapsElem> {
    #[typst_macros::time(name = "smallcaps", span = self.span())]
    fn show(&self, _: &mut Engine, _: StyleChain) -> SourceResult<Content> {
        Ok(self.body().clone().styled(TextElem::set_smallcaps(true)))
    }
}
