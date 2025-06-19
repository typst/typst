use crate::diag::SourceResult;
use crate::engine::Engine;
use crate::foundations::{
    Content, NativeElement, Packed, Show, StyleChain, TargetElem, elem,
};
use crate::html::{HtmlElem, tag};
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
#[elem(title = "Strong Emphasis", keywords = ["bold", "weight"], Show)]
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
        let body = self.body.clone();
        Ok(if TargetElem::target_in(styles).is_html() {
            HtmlElem::new(tag::strong)
                .with_body(Some(body))
                .pack()
                .spanned(self.span())
        } else {
            body.styled(TextElem::set_delta(WeightDelta(self.delta(styles))))
        })
    }
}
