use crate::prelude::*;

/// Exposes a value to the query system without producing visible content.
///
/// This element can be queried for with the [`query`]($func/query) function and
/// the command line `typst query` command. Its purpose is to expose an
/// arbitrary value to the introspection system. To identify a metadata value
/// among others, you can attach a [`label`]($type/label) to it and query for
/// that label.
///
/// ```typ
/// #metadata("This is a note") <note>
/// ```
///
/// ## Within Typst: `query` function { #within-typst }
/// Metadata can be retrieved from with the [`query`]($func/query) function
/// (like other elements):
///
/// ```example
/// // Put metadata somewhere.
/// #metadata("This is a note") <note>
///
/// // And find it from anywhere else.
/// #locate(loc => {
///   query(<note>, loc).first().value
/// })
/// ```
///
/// ## Outside of Typst: `typst query` command { #outside-of-typst }
/// You can also retrieve the metadata from the command line with the
/// `typst query` command. This command executes an arbitrary query on the
/// document and returns the resulting elements in serialized form.
///
/// The `metadata` element is especially useful for command line queries because
/// it allows you to expose arbitrary values to the outside world. However,
/// `typst query` also works with other elements `metadata` and complex
/// [selectors]($type/selector) like `{heading.where(level: 1)}`.
///
/// ```sh
/// $ typst query example.typ "<note>"
/// [
///   {
///     "func": "metadata",
///     "value": "This is a note",
///     "label": "<note>"
///   }
/// ]
/// ```
///
/// Frequently, you're interested in only one specific field of the resulting
/// elements. In the case of the `metadata` element, the `value` field is the
/// interesting one. You can extract just this field with the `--field`
/// argument.
///
/// ```sh
/// $ typst query example.typ "<note>" --field value
/// ["This is a note"]
/// ```
///
/// If you are interested in just a single element, you can use the `--one`
/// flag to extract just it.
///
/// ```sh
/// $ typst query example.typ "<note>" --field value --one
/// "This is a note"
/// ```
///
/// Display: Metadata
/// Category: meta
#[element(Behave, Show, Locatable)]
pub struct MetadataElem {
    /// The value to embed into the document.
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
