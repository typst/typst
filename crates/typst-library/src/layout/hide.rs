use crate::foundations::{Content, elem};
use crate::introspection::Tagged;

/// Hides content without affecting layout.
///
/// The `hide` function allows you to hide content while the layout still "sees"
/// it. This is useful for creating blank space that is exactly as large as some
/// content.
///
/// # Example
/// ```example
/// Hello Jane \
/// #hide[Hello] Joe
/// ```
///
/// # Redaction
/// This function may also be useful for redacting content as its arguments are
/// neither present visually nor accessible to Assistive Technology. That said,
/// there can be _some_ traces of the hidden content (such as a bookmarked
/// heading in the PDF's Document Outline).
///
/// Note that, depending on the circumstances, it may be possible for content to
/// be reverse engineered based on its size in the layout. We thus do not
/// recommend using this function to hide highly sensitive information.
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
