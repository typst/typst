use crate::prelude::*;

/// Provide custom metadata for the Typst query system.
///
/// The Typst query system allows users to extract metadata from the document
/// using a generic Typst selector string. The 'metadata' element is an essential
/// part of this mechanism, associating a single piece of metadata with the
/// specified label without any visible representation in the compiled document.
/// Subsequent invocations will append to a list of values for the same label.
///
/// While this note has no visible output in the document, it embeds metadata
/// into the document! This metadata can be retrieved using the 'query' command
/// via CLI and the 'query()' function from within the document:
/// Example:
/// ```example
/// #metadata((
///     page: 2,
///     description: "This is a note"
/// ))<note>;
///

/// ```
///
/// How to retrieve the metadata:
/// ```sh
/// $ typst query example.typ '<note>'
/// [
///   {
///     "type": "metadata",
///     "value": {
///       "page": 2,
///       "description": "This is a note"
///     },
///     "label": "<note>"
///   }
/// ]
///
/// $ typst query example.typ '<note>' --field value
/// [
///     {
///         "page": 2,
///         "description": "This is a note"
///     }
/// ]
/// ```
///
/// Display: Metadata
/// Category: meta
#[element(Behave, Show, Locatable)]
pub struct MetadataElem {
    /// This value will be associated with the given key.
    #[required]
    pub value: Value,
}

impl Show for MetadataElem {
    fn show(&self, _vt: &mut Vt, _styles: StyleChain) -> SourceResult<Content> {
        Ok(Content::empty())
    }
}

impl Behave for MetadataElem {
    fn behaviour(&self) -> Behaviour {
        Behaviour::Ignorant
    }
}
