use crate::prelude::*;

/// Aligns content horizontally and vertically.
///
/// # Example
/// ```example
/// #set align(center)
///
/// Centered text, a sight to see \
/// In perfect balance, visually \
/// Not left nor right, it stands alone \
/// A work of art, a visual throne
/// ```
#[elem(Show)]
pub struct AlignElem {
    /// The [alignment]($alignment) along both axes.
    ///
    /// ```example
    /// #set page(height: 6cm)
    /// #set text(lang: "ar")
    ///
    /// مثال
    /// #align(
    ///   end + horizon,
    ///   rect(inset: 12pt)[ركن]
    /// )
    /// ```
    #[positional]
    #[fold]
    #[default]
    pub alignment: Align,

    /// The content to align.
    #[required]
    pub body: Content,
}

impl Show for AlignElem {
    #[tracing::instrument(name = "AlignElem::show", skip_all)]
    fn show(&self, _: &mut Vt, styles: StyleChain) -> SourceResult<Content> {
        Ok(self.body().styled(Self::set_alignment(self.alignment(styles))))
    }
}
