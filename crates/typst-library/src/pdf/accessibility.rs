use typst_macros::{Cast, elem};

use crate::foundations::Content;
use crate::introspection::Locatable;

/// Mark content as a PDF artifact.
// TODO: maybe generalize this and use it to mark html elements with `aria-hidden="true"`?
#[elem(Locatable)]
pub struct ArtifactElem {
    /// The artifact kind.
    #[default(ArtifactKind::Other)]
    pub kind: ArtifactKind,

    /// The content that is an artifact.
    #[required]
    pub body: Content,
}

/// The type of artifact.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Cast)]
pub enum ArtifactKind {
    /// Page header artifacts.
    Header,
    /// Page footer artifacts.
    Footer,
    /// Page artifacts, such as cut marks or color bars.
    Page,
    /// Other artifacts.
    #[default]
    Other,
}
