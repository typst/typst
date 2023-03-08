use crate::prelude::*;

/// Hide content without affecting layout.
///
/// The `hide` function allows you to hide content while the layout still 'sees'
/// it. This is useful to create whitespace that is exactly as large as some
/// content. It may also be useful to redact content because its arguments are
/// not included in the output.
///
/// ## Example
/// ```example
/// Hello Jane \
/// #hide[Hello] Joe
/// ```
///
/// Display: Hide
/// Category: layout
#[node(Show)]
pub struct HideNode {
    /// The content to hide.
    #[positional]
    #[required]
    pub body: Content,
}

impl Show for HideNode {
    fn show(&self, _: &mut Vt, _: &Content, _: StyleChain) -> SourceResult<Content> {
        Ok(self.body().styled(MetaNode::set_data(vec![Meta::Hidden])))
    }
}
