use ecow::EcoVec;
use roxmltree::ParsingOptions;
use typst_syntax::Spanned;

use crate::diag::{format_xml_like_error, SourceDiagnostic, SourceResult};
use crate::engine::Engine;
use crate::foundations::{dict, func, scope, Array, Dict, IntoValue, Str, Value};
use crate::loading::{Data, DataSource, Load, Readable};

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
///   [= #title.children.first()]
///   text(10pt, weight: "medium")[
///     Published by
///     #author.children.first()
///   ]
///
///   for p in pars.children {
///     if type(p) == dictionary {
///       parbreak()
///       p.children.first()
///     }
///   }
/// }
///
/// #let data = xml("example.xml")
/// #for elem in data.first().children {
///   if type(elem) == dictionary {
///     article(elem)
///   }
/// }
/// ```
#[func(scope, title = "XML")]
pub fn xml(
    engine: &mut Engine,
    /// A [path]($syntax/#paths) to an XML file or raw XML bytes.
    source: Spanned<DataSource>,
) -> SourceResult<Value> {
    let data = source.load(engine.world)?;
    let text = data.as_str()?;
    let document = roxmltree::Document::parse_with_options(
        text,
        ParsingOptions { allow_dtd: true, ..Default::default() },
    )
    .map_err(|err| format_xml_error(&data, err))?;
    Ok(convert_xml(document.root()))
}

#[scope]
impl xml {
    /// Reads structured data from an XML string/bytes.
    #[func(title = "Decode XML")]
    #[deprecated = "`xml.decode` is deprecated, directly pass bytes to `xml` instead"]
    pub fn decode(
        engine: &mut Engine,
        /// XML data.
        data: Spanned<Readable>,
    ) -> SourceResult<Value> {
        xml(engine, data.map(Readable::into_source))
    }
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
fn format_xml_error(data: &Data, error: roxmltree::Error) -> EcoVec<SourceDiagnostic> {
    format_xml_like_error("XML", data, error)
}
