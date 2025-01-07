use ecow::EcoString;
use typst_syntax::{Span, Spanned};

use crate::diag::{At, SourceResult, StrResult};
use crate::engine::Engine;
use crate::foundations::{
    elem, func, scope, Cast, Content, NativeElement, Packed, Show, Smart, StyleChain,
};
use crate::introspection::Locatable;
use crate::loading::Readable;
use crate::World;

/// A file that will be embedded into the output PDF.
/// This element is ignored by other export formats.
#[elem(scope, Show, Locatable)]
pub struct EmbedElem {
    /// Path to a file to be embedded.
    ///
    /// For more details, see the [Paths section]($syntax/#paths).
    #[required]
    #[parse(
        let Spanned { v: path, span } =
        args.expect::<Spanned<EcoString>>("path to the file to be embedded")?;
        let id = span.resolve_path(&path).at(span)?;
        let data = engine.world.file(id).at(span)?;
        path
    )]
    #[borrowed]
    pub path: EcoString,

    /// The resolved rootless path.
    #[internal]
    #[required]
    #[parse(EcoString::from(id.vpath().as_rootless_path().to_string_lossy()))]
    pub resolved_path: EcoString,

    /// The raw file data.
    #[internal]
    #[required]
    #[parse(Readable::Bytes(data))]
    pub data: Readable,

    /// The name of the attached file.
    ///
    /// If no name is given, the path is used instead.
    #[borrowed]
    pub name: Smart<EcoString>,

    /// A description for the attached file.
    #[borrowed]
    pub description: Option<EcoString>,

    /// The mime-type of the embedded file.
    #[borrowed]
    pub mime_type: Option<EcoString>,

    /// The relationship of the embedded file to the document.
    #[borrowed]
    pub relationship: Option<EmbeddedFileRelationship>,
}

#[scope]
impl EmbedElem {
    /// Decode a file embedding from bytes or a string.
    #[func(title = "Embed bytes or a string as a file")]
    fn decode(
        /// The call span of this function.
        span: Span,
        /// The data to embed as a file.
        data: Readable,
        /// The path of the file embedding.
        path: EcoString,
        /// The name of the attached file.
        ///
        /// If no name is given, the path is used instead.
        #[named]
        name: Option<Smart<EcoString>>,
        /// A description for the attached file.
        #[named]
        description: Option<Option<EcoString>>,
        /// The mime-type of the embedded file.
        #[named]
        mime_type: Option<Option<EcoString>>,
        /// The mime-type of the embedded file.
        #[named]
        relationship: Option<Option<EmbeddedFileRelationship>>,
    ) -> StrResult<Content> {
        let mut elem = EmbedElem::new(path.clone(), path, data);
        if let Some(name) = name {
            elem.push_name(name);
        }
        if let Some(description) = description {
            elem.push_description(description);
        }
        if let Some(mime_type) = mime_type {
            elem.push_mime_type(mime_type);
        }
        if let Some(relationship) = relationship {
            elem.push_relationship(relationship);
        }

        Ok(elem.pack().spanned(span))
    }
}

impl Show for Packed<EmbedElem> {
    fn show(&self, _: &mut Engine, _: StyleChain) -> SourceResult<Content> {
        Ok(Content::empty())
    }
}

/// The relationship of an embedded file with the relevant document content.
#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash, Cast)]
pub enum EmbeddedFileRelationship {
    /// The PDF document was created from this source file.
    Source,
    /// This file was used to derive a visual presentation in the PDF.
    Data,
    /// An alternative representation of this document.
    Alternative,
    /// Additional resources for this document.
    Supplement,
    /// There is no clear relationship or it is not known.
    Unspecified,
}
