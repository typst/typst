use crate::diag::SourceResult;
use crate::engine::Engine;
use crate::foundations::{elem, Content, Packed, Show, StyleChain};
use crate::text::{TextElem, WeightDelta};

/// Strongly emphasizes content by increasing the font weight.
///
/// Increases the current font weight by a given `delta`.
///
/// # Example
/// ```example
/// This is *strong.* \
/// This is #strong[too.] \
///
/// #show strong: set text(red)
/// And this is *evermore.*
/// ```
///
/// # Syntax
/// This function also has dedicated syntax: To strongly emphasize content,
/// simply enclose it in stars/asterisks (`*`). Note that this only works at
/// word boundaries. To strongly emphasize part of a word, you have to use the
/// function.
#[elem(title = "Strong Emphasis", Show)]
pub struct StrongElem {
    /// The delta to apply on the font weight.
    ///
    /// ```example
    /// #set strong(delta: 0)
    /// No *effect!*
    /// ```
    #[default(300)]
    pub delta: i64,

    /// The content to strongly emphasize.
    #[required]
    pub body: Content,
}

impl Show for Packed<StrongElem> {
    #[typst_macros::time(name = "strong", span = self.span())]
    fn show(&self, _: &mut Engine, styles: StyleChain) -> SourceResult<Content> {
        Ok(self
            .body()
            .clone()
            .styled(TextElem::set_delta(WeightDelta(self.delta(styles)))))
    }
}
