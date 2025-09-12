use crate::foundations::{Content, elem};
use crate::introspection::Locatable;

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
#[elem(Locatable)]
pub struct HideElem {
    /// The content to hide.
    #[required]
    pub body: Content,

    /// This style is set on the content contained in the `hide` element.
    #[internal]
    #[ghost]
    pub hidden: bool,
}
