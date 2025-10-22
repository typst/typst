use crate::foundations::{Content, elem};
use crate::introspection::Tagged;

/// Hides content without affecting layout.
///
/// The `hide` function allows you to hide content while the layout still "sees"
/// it. This is useful to create blank space that is exactly as large as some
/// content. It may also be useful to redact content because its arguments are
/// not included in the output, at least visually. However, there can be _some_
/// traces of the hidden content such as bookmarked heading in the PDF Document
/// Outline. Generally speaking, it shouldn't be relied upon for hiding
/// sensitive information, as some content can be reverse engineered.
///
/// # Example
/// ```example
/// Hello Jane \
/// #hide[Hello] Joe
/// ```
#[elem(Tagged)]
pub struct HideElem {
    /// The content to hide.
    #[required]
    pub body: Content,

    /// This style is set on the content contained in the `hide` element.
    #[internal]
    #[ghost]
    pub hidden: bool,
}
