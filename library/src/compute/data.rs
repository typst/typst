use typst::diag::{format_xml_like_error, FileError};

use crate::prelude::*;

/// Read plain text from a file.
///
/// The file will be read and returned as a string.
///
/// ## Example
/// ```example
/// #let text = read("data.html")
///
/// An example for a HTML file:\
/// #raw(text, lang: "html")
/// ```
///
/// Display: Plain text
/// Category: data-loading
/// Returns: string
#[func]
pub fn read(
    /// Path to a file.
    path: Spanned<EcoString>,
) -> Value {
    let Spanned { v: path, span } = path;
    let path = vm.locate(&path).at(span)?;
    let data = vm.world().file(&path).at(span)?;
    let text = std::str::from_utf8(&data)
        .map_err(|_| "file is not valid utf-8")
        .at(span)?;
    Value::Str(text.into())
}

/// Read structured data from a CSV file.
///
/// The CSV file will be read and parsed into a 2-dimensional array of strings:
/// Each row in the CSV file will be represented as an array of strings, and all
/// rows will be collected into a single array. Header rows will not be
/// stripped.
///
/// ## Example
/// ```example
/// #let results = csv("data.csv")
///
/// #table(
///   columns: 2,
///   [*Condition*], [*Result*],
///   ..results.flatten(),
/// )
/// ```
///
/// Display: CSV
/// Category: data-loading
/// Returns: array
#[func]
pub fn csv(
    /// Path to a CSV file.
    path: Spanned<EcoString>,
    /// The delimiter that separates columns in the CSV file.
    /// Must be a single ASCII character.
    /// Defaults to a comma.
    #[named]
    #[default]
    delimiter: Delimiter,
) -> Value {
    let Spanned { v: path, span } = path;
    let path = vm.locate(&path).at(span)?;
    let data = vm.world().file(&path).at(span)?;

    let mut builder = csv::ReaderBuilder::new();
    builder.has_headers(false);
    builder.delimiter(delimiter.0);

    let mut reader = builder.from_reader(data.as_slice());
    let mut array = Array::new();

    for (line, result) in reader.records().enumerate() {
        // Original solution use line from error, but that is incorrect with has_headers set to false
        // See issue: https://github.com/BurntSushi/rust-csv/issues/184
        let line = line + 1; // Counting lines from 1
        let row = result.map_err(|err| format_csv_error(err, line)).at(span)?;
        let sub = row.iter().map(|field| Value::Str(field.into())).collect();
        array.push(Value::Array(sub))
    }

    Value::Array(array)
}

/// The delimiter to use when parsing CSV files.
struct Delimiter(u8);

cast_from_value! {
    Delimiter,
    v: EcoString => {
        let mut chars = v.chars();
        let first = chars.next().ok_or("delimiter must not be empty")?;
        if chars.next().is_some() {
            Err("delimiter must be a single character")?
        }

        if !first.is_ascii() {
            Err("delimiter must be an ASCII character")?
        }

        Self(first as u8)
    },
}

impl Default for Delimiter {
    fn default() -> Self {
        Self(b',')
    }
}

/// Format the user-facing CSV error message.
fn format_csv_error(error: csv::Error, line: usize) -> String {
    match error.kind() {
        csv::ErrorKind::Utf8 { .. } => "file is not valid utf-8".into(),
        csv::ErrorKind::UnequalLengths { expected_len, len, .. } => {
            format!(
                "failed to parse csv file: found {len} instead of {expected_len} fields in line {line}"
            )
        }
        _ => "failed to parse csv file".into(),
    }
}

/// Read structured data from a JSON file.
///
/// The file must contain a valid JSON object or array. JSON objects will be
/// converted into Typst dictionaries, and JSON arrays will be converted into
/// Typst arrays. Strings and booleans will be converted into the Typst
/// equivalents, `null` will be converted into `{none}`, and numbers will be
/// converted to floats or integers depending on whether they are whole numbers.
///
/// The function returns a dictionary or an array, depending on the JSON file.
///
/// The JSON files in the example contain objects with the keys `temperature`,
/// `unit`, and `weather`.
///
/// ## Example
/// ```example
/// #let forecast(day) = block[
///   #box(square(
///     width: 2cm,
///     inset: 8pt,
///     fill: if day.weather == "sunny" {
///       yellow
///     } else {
///       aqua
///     },
///     align(
///       bottom + right,
///       strong(day.weather),
///     ),
///   ))
///   #h(6pt)
///   #set text(22pt, baseline: -8pt)
///   #day.temperature °#day.unit
/// ]
///
/// #forecast(json("monday.json"))
/// #forecast(json("tuesday.json"))
/// ```
///
/// Display: JSON
/// Category: data-loading
/// Returns: array or dictionary
#[func]
pub fn json(
    /// Path to a JSON file.
    path: Spanned<EcoString>,
) -> Value {
    let Spanned { v: path, span } = path;
    let path = vm.locate(&path).at(span)?;
    let data = vm.world().file(&path).at(span)?;
    let value: serde_json::Value =
        serde_json::from_slice(&data).map_err(format_json_error).at(span)?;
    convert_json(value)
}

/// Convert a JSON value to a Typst value.
fn convert_json(value: serde_json::Value) -> Value {
    match value {
        serde_json::Value::Null => Value::None,
        serde_json::Value::Bool(v) => Value::Bool(v),
        serde_json::Value::Number(v) => match v.as_i64() {
            Some(int) => Value::Int(int),
            None => Value::Float(v.as_f64().unwrap_or(f64::NAN)),
        },
        serde_json::Value::String(v) => Value::Str(v.into()),
        serde_json::Value::Array(v) => {
            Value::Array(v.into_iter().map(convert_json).collect())
        }
        serde_json::Value::Object(v) => Value::Dict(
            v.into_iter()
                .map(|(key, value)| (key.into(), convert_json(value)))
                .collect(),
        ),
    }
}

/// Format the user-facing JSON error message.
#[track_caller]
fn format_json_error(error: serde_json::Error) -> String {
    assert!(error.is_syntax() || error.is_eof());
    format!("failed to parse json file: syntax error in line {}", error.line())
}

/// Read structured data from an XML file.
///
/// The XML file is parsed into an array of dictionaries and strings. XML nodes
/// can be elements or strings. Elements are represented as dictionaries with
/// the the following keys:
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
/// ## Example
/// ```example
/// #let findChild(elem, tag) = {
///   elem.children
///     .find(e => "tag" in e and e.tag == tag)
/// }
///
/// #let article(elem) = {
///   let title = findChild(elem, "title")
///   let author = findChild(elem, "author")
///   let pars = findChild(elem, "content")
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
/// #for child in data.first().children {
///   if (type(child) == "dictionary") {
///     article(child)
///   }
/// }
/// ```
///
/// Display: XML
/// Category: data-loading
/// Returns: array
#[func]
pub fn xml(
    /// Path to an XML file.
    path: Spanned<EcoString>,
) -> Value {
    let Spanned { v: path, span } = path;
    let path = vm.locate(&path).at(span)?;
    let data = vm.world().file(&path).at(span)?;
    let text = std::str::from_utf8(&data).map_err(FileError::from).at(span)?;
    let document = roxmltree::Document::parse(text).map_err(format_xml_error).at(span)?;
    convert_xml(document.root())
}

/// Convert an XML node to a Typst value.
fn convert_xml(node: roxmltree::Node) -> Value {
    if node.is_text() {
        return Value::Str(node.text().unwrap_or_default().into());
    }

    let children: Array = node.children().map(convert_xml).collect();
    if node.is_root() {
        return Value::Array(children);
    }

    let tag: Str = node.tag_name().name().into();
    let attrs: Dict = node
        .attributes()
        .iter()
        .map(|attr| (attr.name().into(), attr.value().into()))
        .collect();

    Value::Dict(dict! {
        "tag" => tag,
        "attrs" => attrs,
        "children" => children,
    })
}

/// Format the user-facing XML error message.
fn format_xml_error(error: roxmltree::Error) -> String {
    format_xml_like_error("xml file", error)
}
