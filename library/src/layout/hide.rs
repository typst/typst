use crate::prelude::*;

/// # Hide
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
/// ## Parameters
/// - body: `Content` (positional, required)
///   The content to hide.
///
/// ## Category
/// layout
#[func]
#[capable(Show)]
#[derive(Debug, Hash)]
pub struct HideNode(pub Content);

#[node]
impl HideNode {
    fn construct(_: &Vm, args: &mut Args) -> SourceResult<Content> {
        Ok(Self(args.expect("body")?).pack())
    }
}

impl Show for HideNode {
    fn show(&self, _: &mut Vt, _: &Content, _: StyleChain) -> SourceResult<Content> {
        Ok(self.0.clone().styled(Meta::DATA, vec![Meta::Hidden]))
    }
}
