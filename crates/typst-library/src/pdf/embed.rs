use crate::diag::{At, SourceResult, StrResult};
use crate::engine::Engine;
use crate::foundations::{func, Bytes, Content, NativeElement, Packed, Show, StyleChain};
use crate::introspection::Locatable;
use crate::loading::Readable;
use crate::text::LocalName;
use crate::{Span, World};
use ecow::EcoString;
use std::sync::Arc;
use typst_macros::{elem, scope, Cast};
use typst_syntax::Spanned;
use typst_utils::LazyHash;

#[elem(scope, Show, LocalName, Locatable)]
pub struct EmbedElem {
    /// Path to a file to be embedded
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

    /// The raw file data.
    #[internal]
    #[required]
    #[parse(Readable::Bytes(data))]
    pub data: Readable,

    /// The name of the attached file
    ///
    /// If no name is given, the path is used instead
    #[borrowed]
    pub name: Option<EcoString>,

    /// A description for the attached file
    #[borrowed]
    pub description: Option<EcoString>,

    /// The mime-type of the embedded file
    #[borrowed]
    pub mime_type: Option<EcoString>,

    /// The relationship of the embedded file to the document
    #[borrowed]
    pub relationship: Option<EmbeddedFileRelationship>,
}

#[scope]
impl EmbedElem {
    #[func(title = "Embed the given data as a file")]
    fn decode(
        /// The call span of this function.
        span: Span,
        /// The data to embed as a file
        data: Readable,
        /// The path of the file embedding
        path: EcoString,
        /// The name of the attached file
        ///
        /// If no name is given, the path is used instead
        #[named]
        name: Option<Option<EcoString>>,
        /// A description for the attached file
        #[named]
        description: Option<Option<EcoString>>,
        /// The mime-type of the embedded file
        #[named]
        mime_type: Option<Option<EcoString>>,
        /// The mime-type of the embedded file
        #[named]
        relationship: Option<Option<EmbeddedFileRelationship>>,
    ) -> StrResult<Content> {
        let mut elem = EmbedElem::new(path, data);
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

impl LocalName for Packed<EmbedElem> {
    const KEY: &'static str = "embedding";
}

impl Show for Packed<EmbedElem> {
    fn show(&self, _: &mut Engine, _: StyleChain) -> SourceResult<Content> {
        Ok(Content::empty())
    }
}

/// The internal representation of a file embedding
#[derive(Hash)]
pub struct Repr {
    /// The raw file data.
    data: Bytes,
    /// Path of this embedding
    path: EcoString,
    /// Name of this embedding
    name: EcoString,
    /// Name of this embedding
    description: Option<EcoString>,
    /// Name of this embedding
    mime_type: Option<EcoString>,
    /// Name of this embedding
    relationship: Option<EmbeddedFileRelationship>,
}

/// The relationship of an embedded file with the relevant document content
#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash, Cast)]
pub enum EmbeddedFileRelationship {
    /// The embedded file is the original source material of the document content
    Source,
    /// The embedded file represents information used to derive a visual presentation – such
    /// as for a table or a graph.
    Data,
    /// The embedded file is an alternative representation of document content
    Alternative,
    /// The embedded file is a supplemental representation of document content
    Supplement,
    /// The embedded file is encrypted and should be displayed to the user if
    /// the PDF processor has the cryptographic filter needed to
    /// decrypt the document.
    EncryptedPayload,
    /// The embedded file is data associated with an AcroForm
    FormData,
    /// The embedded file is a schema definition
    Schema,
    /// The embedded file has an unknown relationship to the document or the relationship cannot be
    /// described by the other variants
    Unspecified,
}

/// A loaded file to be embedded.
///
/// Values of this type are cheap to clone and hash.
#[derive(Clone, Hash, Eq, PartialEq)]
pub struct Embed(Arc<LazyHash<Repr>>);

impl Embed {
    pub fn from_element(element: &Packed<EmbedElem>) -> Self {
        let repr = Repr {
            data: element.data.clone().into(),
            path: element.path.clone(),
            name: if let Some(Some(name)) = element.name.as_ref() {
                name.clone()
            } else {
                element.path.clone()
            },
            description: if let Some(Some(description)) = element.description.as_ref() {
                Some(description.clone())
            } else {
                None
            },
            mime_type: if let Some(Some(mime_type)) = element.mime_type.as_ref() {
                Some(mime_type.clone())
            } else {
                None
            },
            relationship: if let Some(Some(relationship)) = element.relationship.as_ref()
            {
                Some(*relationship)
            } else {
                None
            },
        };

        Embed(Arc::new(LazyHash::new(repr)))
    }

    /// The raw file data.
    pub fn data(&self) -> &Bytes {
        &self.0.data
    }

    /// The name of the file embedding
    pub fn name(&self) -> &EcoString {
        &self.0.name
    }

    /// The path of the file embedding
    pub fn path(&self) -> &EcoString {
        &self.0.path
    }

    /// The description of the file embedding
    pub fn description(&self) -> Option<&str> {
        self.0.description.as_deref()
    }

    /// The mime type of the embedded file
    pub fn mime_type(&self) -> Option<&str> {
        self.0.mime_type.as_deref()
    }

    /// The relationship of the file with the document
    pub fn relationship(&self) -> Option<&EmbeddedFileRelationship> {
        self.0.relationship.as_ref()
    }
}
