use crate::diag::SourceResult;
use crate::engine::Engine;
use crate::foundations::{
    elem, Content, Packed, Show, StyleChain,
};

/// Creates non-selectable content.
///
/// Makes the content non-selectable in both PDF and HTML output:
/// - In PDF: Uses PDF artifacts (primarily supported in Adobe Acrobat Reader)
/// - In HTML: Uses CSS user-select property, not implemented for now.
///
/// Note: While the text appears non-selectable in viewers, the text information
/// remains in the document and can still be extracted programmatically.
///
/// # Example
/// ```example
/// #watermark[Confidential]
/// ```
#[elem(Show)]
pub struct WatermarkElem {
    /// The content to render as non-selectable.
    #[required]
    pub body: Content,

    /// This style is set on the content contained in the `watermark` element.
    #[internal]
    #[ghost]
    pub watermarked: bool,
}

impl Show for Packed<WatermarkElem> {
    #[typst_macros::time(name = "watermark", span = self.span())]
    fn show(&self, _: &mut Engine, _styles: StyleChain) -> SourceResult<Content> {
        Ok(self.body.clone().styled(WatermarkElem::set_watermarked(true)))
    }
}
