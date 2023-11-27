use crate::diag::SourceResult;
use crate::engine::Engine;
use crate::foundations::{elem, Content, Show, StyleChain};
use crate::text::{ItalicToggle, TextElem};

/// Emphasizes content by setting it in italics.
///
/// - If the current [text style]($text.style) is `{"normal"}`, this turns it
///   into `{"italic"}`.
/// - If it is already `{"italic"}` or `{"oblique"}`, it turns it back to
///   `{"normal"}`.
///
/// # Example
/// ```example
/// This is _emphasized._ \
/// This is #emph[too.]
///
/// #show emph: it => {
///   text(blue, it.body)
/// }
///
/// This is _emphasized_ differently.
/// ```
///
/// # Syntax
/// This function also has dedicated syntax: To emphasize content, simply
/// enclose it in underscores (`_`). Note that this only works at word
/// boundaries. To emphasize part of a word, you have to use the function.
#[elem(title = "Emphasis", Show)]
pub struct EmphElem {
    /// The content to emphasize.
    #[required]
    pub body: Content,
}

impl Show for EmphElem {
    #[tracing::instrument(name = "EmphElem::show", skip(self))]
    fn show(&self, _: &mut Engine, _: StyleChain) -> SourceResult<Content> {
        Ok(self.body().clone().styled(TextElem::set_emph(ItalicToggle)))
    }
}
