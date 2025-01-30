use crate::diag::{warning, SourceResult};
use crate::engine::Engine;
use crate::foundations::{elem, Content, Packed, SequenceElem, Show, StyleChain};
use crate::layout::{Em, Length};
use crate::text::{SpaceElem, TextElem, TextSize};

/// Renders text in subscript.
///
/// The text is rendered smaller and its baseline is lowered.
///
/// # Example
/// ```example
/// Revenue#sub[yearly]
/// ```
#[elem(title = "Subscript", Show)]
pub struct SubElem {
    /// Whether to use the `subs` OpenType feature to render the glyphs.
    ///
    /// Do not use this is the body contains non-text elements.
    ///
    /// Note that some fonts might not support this feature, or not support it
    /// for all characters.
    ///
    /// ```example
    /// N#sub(typographic: true)[1]
    /// N#sub(typographic: false)[1]
    /// ```
    #[default(false)]
    pub typographic: bool,

    /// The baseline shift for synthetic subscripts. Does not apply if
    /// `typographic` is true and the font has subscript codepoints for the
    /// given `body`.
    #[default(Em::new(0.2).into())]
    pub baseline: Length,

    /// The font size for synthetic subscripts. Does not apply if
    /// `typographic` is true and the font has subscript codepoints for the
    /// given `body`.
    #[default(TextSize(Em::new(0.6).into()))]
    pub size: TextSize,

    /// The text to display in subscript.
    #[required]
    pub body: Content,
}

impl Show for Packed<SubElem> {
    #[typst_macros::time(name = "sub", span = self.span())]
    fn show(&self, engine: &mut Engine, styles: StyleChain) -> SourceResult<Content> {
        let body = self.body.clone();

        if self.typographic(styles) {
            if !is_text(&body) {
                engine.sink.warn(warning!(
                    self.span(),
                    "typographic subscript element contains non-textual content";
                    hint: "this may have undesired behavior";
                    hint: "set the `typographic` parameter to `false`"
                ))
            }
            return Ok(body.styled(TextElem::set_subscript(true)));
        };

        Ok(body
            .styled(TextElem::set_baseline(self.baseline(styles)))
            .styled(TextElem::set_size(self.size(styles))))
    }
}

/// Renders text in superscript.
///
/// The text is rendered smaller and its baseline is raised.
///
/// # Example
/// ```example
/// 1#super[st] try!
/// ```
#[elem(title = "Superscript", Show)]
pub struct SuperElem {
    /// Whether to use the `sups` OpenType feature to render the glyphs.
    ///
    /// Do not use this is the body contains non-text elements.
    ///
    /// Note that some fonts might not support this feature, or not support it
    /// for all characters.
    ///
    /// ```example
    /// N#super(typographic: true)[1]
    /// N#super(typographic: false)[1]
    /// ```
    #[default(false)]
    pub typographic: bool,

    /// The baseline shift for synthetic superscripts. Does not apply if
    /// `typographic` is true and the font has superscript codepoints for the
    /// given `body`.
    #[default(Em::new(-0.5).into())]
    pub baseline: Length,

    /// The font size for synthetic superscripts. Does not apply if
    /// `typographic` is true and the font has superscript codepoints for the
    /// given `body`.
    #[default(TextSize(Em::new(0.6).into()))]
    pub size: TextSize,

    /// The text to display in superscript.
    #[required]
    pub body: Content,
}

impl Show for Packed<SuperElem> {
    #[typst_macros::time(name = "super", span = self.span())]
    fn show(&self, engine: &mut Engine, styles: StyleChain) -> SourceResult<Content> {
        let body = self.body.clone();

        if self.typographic(styles) {
            if !is_text(&body) {
                engine.sink.warn(warning!(
                    self.span(),
                    "typographic superscript element contains non-textual content";
                    hint: "this may have undesired behavior";
                    hint: "set the `typographic` parameter to `false`"
                ))
            }
            return Ok(body.styled(TextElem::set_superscript(true)));
        };

        Ok(body
            .styled(TextElem::set_baseline(self.baseline(styles)))
            .styled(TextElem::set_size(self.size(styles))))
    }
}

fn is_text(content: &Content) -> bool {
    content.is::<SpaceElem>()
        || content.is::<TextElem>()
        || content
            .to_packed::<SequenceElem>()
            .is_some_and(|sequence| sequence.children.iter().all(is_text))
}
