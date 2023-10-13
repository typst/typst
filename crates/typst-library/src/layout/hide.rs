use crate::prelude::*;

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
}

impl Show for HideElem {
    #[tracing::instrument(name = "HideElem::show", skip(self))]
    fn show(&self, _: &mut Vt, _: StyleChain) -> SourceResult<Content> {
        Ok(self.body().styled(MetaElem::set_data(vec![Meta::Hide])))
    }
}
