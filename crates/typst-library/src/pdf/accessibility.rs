use typst_macros::{cast, elem};

use crate::diag::SourceResult;
use crate::engine::Engine;
use crate::foundations::{Content, Packed, Show, StyleChain};
use crate::introspection::Locatable;

// TODO: docs

/// Mark content as a PDF artifact.
/// TODO: also use to mark html elements with `aria-hidden="true"`?
#[elem(Locatable, Show)]
pub struct ArtifactElem {
    #[default(ArtifactKind::Other)]
    pub kind: ArtifactKind,

    /// The content to underline.
    #[required]
    pub body: Content,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum ArtifactKind {
    /// Page header artifacts.
    Header,
    /// Page footer artifacts.
    Footer,
    /// Other page artifacts.
    Page,
    /// Other artifacts.
    #[default]
    Other,
}

cast! {
    ArtifactKind,
    self => match self {
        ArtifactKind::Header => "header".into_value(),
        ArtifactKind::Footer => "footer".into_value(),
        ArtifactKind::Page => "page".into_value(),
        ArtifactKind::Other => "other".into_value(),
    },
    "header" => Self::Header,
    "footer" => Self::Footer,
    "page" => Self::Page,
    "other" => Self::Other,
}

impl Show for Packed<ArtifactElem> {
    #[typst_macros::time(name = "underline", span = self.span())]
    fn show(&self, _: &mut Engine, _: StyleChain) -> SourceResult<Content> {
        Ok(self.body.clone())
    }
}
