use crate::{
    diag::SourceResult,
    engine::Engine,
    foundations::{
        elem, Content, NativeElement, Packed, Show, ShowSet, Smart, StyleChain, Styles,
        TargetElem,
    },
    html::{tag, HtmlElem},
    introspection::Locatable,
    layout::{AlignElem, Alignment, BlockBody, BlockElem, Em},
    text::{FontWeight, TextElem, TextSize},
};

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
#[elem(Locatable, Show, ShowSet)]
pub struct TitleElem {
    /// The content of the title.
    #[required]
    pub body: Content,
}

impl Show for Packed<TitleElem> {
    #[typst_macros::time(name = "title", span = self.span())]
    fn show(&self, _engine: &mut Engine, styles: StyleChain) -> SourceResult<Content> {
        let html = TargetElem::target_in(styles).is_html();

        let span = self.span();
        let realized = self.body().clone();

        Ok(if html {
            HtmlElem::new(tag::h1).with_body(Some(realized)).pack().spanned(span)
        } else {
            let realized = BlockBody::Content(realized);
            BlockElem::new().with_body(Some(realized)).pack().spanned(span)
        })
    }
}

impl ShowSet for Packed<TitleElem> {
    fn show_set(&self, _styles: StyleChain) -> Styles {
        const SIZE: Em = Em::new(1.6);
        const ABOVE: Em = Em::new(1.125);
        const BELOW: Em = Em::new(0.75);

        let mut out = Styles::new();
        out.set(TextElem::set_size(TextSize(SIZE.into())));
        out.set(TextElem::set_weight(FontWeight::BOLD));
        out.set(BlockElem::set_above(Smart::Custom(ABOVE.into())));
        out.set(BlockElem::set_below(Smart::Custom(BELOW.into())));
        out.set(BlockElem::set_sticky(true));
        out.set(AlignElem::set_alignment(Alignment::CENTER));
        out
    }
}
