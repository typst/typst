use crate::diag::SourceResult;
use crate::engine::Engine;
use crate::foundations::{elem, Content, Packed, Show, StyleChain, Value};
use crate::introspection::Locatable;

/// Exposes a value to the query system without producing visible content.
///
/// This element can be retrieved with the [`query`] function and from the
/// command line with
/// [`typst query`]($reference/introspection/query/#command-line-queries). Its
/// purpose is to expose an arbitrary value to the introspection system. To
/// identify a metadata value among others, you can attach a [`label`] to it and
/// query for that label.
///
/// The `metadata` element is especially useful for command line queries because
/// it allows you to expose arbitrary values to the outside world.
///
/// ```example
/// // Put metadata somewhere.
/// #metadata("This is a note") <note>
///
/// // And find it from anywhere else.
/// #context {
///   query(<note>).first().value
/// }
/// ```
#[elem(Show, Locatable)]
pub struct MetadataElem {
    /// The value to embed into the document.
    #[required]
    pub value: Value,
}

impl Show for Packed<MetadataElem> {
    fn show(&self, _: &mut Engine, _styles: StyleChain) -> SourceResult<Content> {
        Ok(Content::empty())
    }
}
