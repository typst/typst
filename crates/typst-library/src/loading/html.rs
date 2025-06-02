use ecow::eco_format;
use ego_tree::NodeRef;
use scraper::Node;
use typst_syntax::Spanned;

use crate::diag::{At, FileError, SourceDiagnostic, SourceResult};
use crate::engine::Engine;
use crate::foundations::{dict, func, Array, Dict, IntoValue, Value};
use crate::loading::{DataSource, Load};

/// Reads structured data from an HTML file.
///
/// The HTML file is parsed into an array of dictionaries and strings. It is compatible with
/// the XML format, parsed by the [`xml`]($xml) function.
#[func(title = "HTML")]
pub fn html_decode(
    engine: &mut Engine,
    /// A [path]($syntax/#paths) to an HTML file or raw HTML bytes.
    source: Spanned<DataSource>,
) -> SourceResult<Value> {
    let data = source.load(engine.world)?;
    let text = data.as_str().map_err(FileError::from).at(source.span)?;
    let document = scraper::Html::parse_document(text);

    if !document.errors.is_empty() {
        let errors = document.errors.iter();
        return Err(errors
            .map(|msg| {
                SourceDiagnostic::error(
                    source.span,
                    eco_format!("failed to parse HTML ({msg})"),
                )
            })
            .collect());
    }

    Ok(convert_html(document.tree.root()))
}

/// Convert an HTML node to a Typst value.
fn convert_html(node_ref: NodeRef<Node>) -> Value {
    // `prefix` and `name` are part of the tag name. For example,
    // in the following HTML, `html5` is the prefix and `div` is the name:
    // ```
    // <html5:div class="example" />
    // ```
    let (prefix, name, attrs) = match node_ref.value() {
        Node::Text(text) => return (*text).into_value(),
        Node::Document => return Value::Array(convert_html_children(node_ref)),
        // todo: the namespace is ignored
        Node::Element(element) => {
            (element.name.prefix.as_ref(), &*element.name.local, Some(element.attrs()))
        }
        Node::Fragment => (None, "fragment", None),
        // todo: doc type and processing instruction are ignored
        // https://en.wikipedia.org/wiki/Processing_Instruction
        Node::Doctype(..) | Node::ProcessingInstruction(..) => return Value::None,
        Node::Comment(comment) => {
            return Value::Dict(dict! {
                "tag" => "",
                "attrs" => dict! {},
                "children" => [(*comment).into_value()].into_iter().collect::<Array>(),
            });
        }
    };

    let children = convert_html_children(node_ref);

    let attrs: Dict = attrs
        .into_iter()
        .flatten()
        .map(|(name, value)| (name.into(), value.into_value()))
        .collect();

    let mut converted = dict! {
        "tag" => name.into_value(),
        "attrs" => attrs,
        "children" => children,
    };

    // In most cases, the prefix is not set, so we only add it if it exists.
    if let Some(prefix) = prefix {
        converted.insert("prefix".into(), (*prefix).into_value());
    }

    Value::Dict(converted)
}

/// Convert children an HTML node to a Typst value.
fn convert_html_children(node_ref: NodeRef<Node>) -> Array {
    node_ref
        .children()
        .filter(|v| {
            !matches!(v.value(), Node::Doctype(..) | Node::ProcessingInstruction(..))
        })
        .map(convert_html)
        .collect()
}
