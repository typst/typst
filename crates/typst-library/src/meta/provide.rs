use crate::prelude::*;

/// Provide custom metadata for the Typst query system.
///
/// The Typst query system allows users to extract metadata from the document,
/// offering both a generic selector and a specific key-value mechanism. The
/// 'provide()' function is an essential part of this mechanism, associating a
/// single piece of metadata with the specified key. Subsequent invocations
/// will append to a list of values for the same key.
///
/// While this note has no visible output in the document, it embeds metadata
/// into the document! This metadata can be retrieved using the 'query' command
/// via CLI and the 'query()' function from within the document:
/// Example:
/// ```example
/// #provide("note", (
///     page: 2,
///     description: "This is a note"
/// ));
///

/// ```
///
/// How to retrieve the metadata:
/// ```sh
/// $ typst query example.typ --key note
/// [{
///     "page": 2,
///     "description": "This is a note"
/// }]
/// ```
///
/// Display: Provide
/// Category: meta
#[element(Behave, Show, Locatable)]
pub struct ProvideElem {
    #[required]
    pub key: EcoString,
    #[required]
    pub value: Value,
}

impl Show for ProvideElem {
    fn show(&self, _vt: &mut Vt, _styles: StyleChain) -> SourceResult<Content> {
        Ok(Content::empty())
    }
}

impl Behave for ProvideElem {
    fn behaviour(&self) -> Behaviour {
        Behaviour::Ignorant
    }
}
