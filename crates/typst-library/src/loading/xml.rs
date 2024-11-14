use ecow::{EcoString, EcoVec};
use roxmltree::ParsingOptions;
use typst_syntax::{FileId, Span, Spanned};

use crate::diag::{format_xml_like_error, At, FileError, SourceDiagnostic, SourceResult};
use crate::engine::Engine;
use crate::foundations::{dict, func, scope, Array, Dict, IntoValue, Str, Value};
use crate::loading::Readable;
use crate::World;

/// Reads structured data from an XML file.
///
/// The XML file is parsed into an array of dictionaries and strings. XML nodes
/// can be elements or strings. Elements are represented as dictionaries with
/// the following keys:
///
/// - `tag`: The name of the element as a string.
/// - `attrs`: A dictionary of the element's attributes as strings.
/// - `children`: An array of the element's child nodes.
///
/// The XML file in the example contains a root `news` tag with multiple
/// `article` tags. Each article has a `title`, `author`, and `content` tag. The
/// `content` tag contains one or more paragraphs, which are represented as `p`
/// tags.
///
/// # Example
/// ```example
/// #let find-child(elem, tag) = {
///   elem.children
///     .find(e => "tag" in e and e.tag == tag)
/// }
///
/// #let article(elem) = {
///   let title = find-child(elem, "title")
///   let author = find-child(elem, "author")
///   let pars = find-child(elem, "content")
///
///   heading(title.children.first())
///   text(10pt, weight: "medium")[
///     Published by
///     #author.children.first()
///   ]
///
///   for p in pars.children {
///     if (type(p) == "dictionary") {
///       parbreak()
///       p.children.first()
///     }
///   }
/// }
///
/// #let data = xml("example.xml")
/// #for elem in data.first().children {
///   if (type(elem) == "dictionary") {
///     article(elem)
///   }
/// }
/// ```
#[func(scope, title = "XML")]
pub fn xml(
    /// The engine.
    engine: &mut Engine,
    /// Path to an XML file.
    ///
    /// For more details, see the [Paths section]($syntax/#paths).
    path: Spanned<EcoString>,
) -> SourceResult<Value> {
    let Spanned { v: path, span } = path;
    let id = span.resolve_path(&path).at(span)?;
    let data = engine.world.file(id).at(span)?;
    decode_inner(Spanned::new(Readable::Bytes(data), span), Some(id))
}

#[scope]
impl xml {
    /// Reads structured data from an XML string/bytes.
    #[func(title = "Decode XML")]
    pub fn decode(
        /// XML data.
        data: Spanned<Readable>,
    ) -> SourceResult<Value> {
        decode_inner(data, None)
    }
}

fn decode_inner(data: Spanned<Readable>, file: Option<FileId>) -> SourceResult<Value> {
    let Spanned { v: data, span } = data;
    let text = std::str::from_utf8(data.as_slice())
        .map_err(FileError::from)
        .at(span)?;
    let document = roxmltree::Document::parse_with_options(
        text,
        ParsingOptions { allow_dtd: true, ..Default::default() },
    )
    .map_err(|err| format_xml_error(err, span, file, text))?;

    Ok(convert_xml(document.root()))
}

/// Convert an XML node to a Typst value.
fn convert_xml(node: roxmltree::Node) -> Value {
    if node.is_text() {
        return node.text().unwrap_or_default().into_value();
    }

    let children: Array = node.children().map(convert_xml).collect();
    if node.is_root() {
        return Value::Array(children);
    }

    let tag: Str = node.tag_name().name().into();
    let attrs: Dict = node
        .attributes()
        .map(|attr| (attr.name().into(), attr.value().into_value()))
        .collect();

    Value::Dict(dict! {
        "tag" => tag,
        "attrs" => attrs,
        "children" => children,
    })
}

/// Format the user-facing XML error message.
fn format_xml_error(
    error: roxmltree::Error,
    span: Span,
    file: Option<FileId>,
    text: &str,
) -> EcoVec<SourceDiagnostic> {
    format_xml_like_error("XML", error, span, file, text)
}
