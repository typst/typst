use crate::diag::SourceResult;
use crate::engine::Engine;
use crate::foundations::{elem, Content, Packed, Show, StyleChain};

/// Hides content without affecting layout.
///
/// The `hide` function allows you to hide content while the layout still 'sees'
/// it. This is useful to create whitespace that is exactly as large as some
/// content. It may also be useful to redact content because its arguments are
/// not included in the output.
///
/// # Example
/// ```example
/// Hello Jane \
/// #hide[Hello] Joe
/// ```
#[elem(Show)]
pub struct HideElem {
    /// The content to hide.
    #[required]
    pub body: Content,

    /// This style is set on the content contained in the `hide` element.
    #[internal]
    #[ghost]
    pub hidden: bool,
}

impl Show for Packed<HideElem> {
    #[typst_macros::time(name = "hide", span = self.span())]
    fn show(&self, _: &mut Engine, _: StyleChain) -> SourceResult<Content> {
        Ok(self.body.clone().set(HideElem::hidden, true))
    }
}
