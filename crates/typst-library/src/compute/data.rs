use typst::diag::{format_xml_like_error, FileError};
use typst::eval::Bytes;

use crate::prelude::*;

/// Reads plain text or data from a file.
///
/// By default, the file will be read as UTF-8 and returned as a
/// [string]($type/string).
///
/// If you specify `{encoding: none}`, this returns raw [bytes]($type/bytes)
/// instead.
///
/// ## Example { #example }
/// ```example
/// An example for a HTML file: \
/// #let text = read("data.html")
/// #raw(text, lang: "html")
///
/// Raw bytes:
/// #read("tiger.jpg", encoding: none)
/// ```
///
/// Display: Read
/// Category: data-loading
#[func]
pub fn read(
    /// Path to a file.
    path: Spanned<EcoString>,
    /// The encoding to read the file with.
    ///
    /// If set to `{none}`, this function returns raw bytes.
    #[named]
    #[default(Some(Encoding::Utf8))]
    encoding: Option<Encoding>,
    /// The virtual machine.
    vm: &mut Vm,
) -> SourceResult<Readable> {
    let Spanned { v: path, span } = path;
    let id = vm.resolve_path(&path).at(span)?;
    let data = vm.world().file(id).at(span)?;
    Ok(match encoding {
        None => Readable::Bytes(data),
        Some(Encoding::Utf8) => Readable::Str(
            std::str::from_utf8(&data)
                .map_err(|_| "file is not valid utf-8")
                .at(span)?
                .into(),
        ),
    })
}

/// An encoding of a file.
#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash, Cast)]
pub enum Encoding {
    /// The Unicode UTF-8 encoding.
    Utf8,
}

/// A value that can be read from a file.
pub enum Readable {
    /// A decoded string.
    Str(Str),
    /// Raw bytes.
    Bytes(Bytes),
}

impl Readable {
    fn as_slice(&self) -> &[u8] {
        match self {
            Readable::Bytes(v) => v,
            Readable::Str(v) => v.as_bytes(),
        }
    }
}

cast! {
    Readable,
    self => match self {
        Self::Str(v) => v.into_value(),
        Self::Bytes(v) => v.into_value(),
    },
    v: Str => Self::Str(v),
    v: Bytes => Self::Bytes(v),
}

impl From<Readable> for Bytes {
    fn from(value: Readable) -> Self {
        match value {
            Readable::Bytes(v) => v,
            Readable::Str(v) => v.as_bytes().into(),
        }
    }
}

/// Writes plain text to a file.
///
///
/// Display: Write
/// Category: data-loading
#[func]
pub fn write(
    /// Path to a file.
    path: Spanned<EcoString>,
    /// Text to write.
    text: Spanned<EcoString>,
    /// The virtual machine.
    vm: &mut Vm,
) -> SourceResult<()> {
    let Spanned { v: path, span } = path;
    let Spanned { v: text, span: _ } = text;
    let id = vm.resolve_path(&path).at(span)?;
    vm.world().write(id, text.as_bytes()).at(span)
}

/// Reads structured data from a CSV file.
///
/// The CSV file will be read and parsed into a 2-dimensional array of strings:
/// Each row in the CSV file will be represented as an array of strings, and all
/// rows will be collected into a single array. Header rows will not be
/// stripped.
///
/// ## Example { #example }
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
#[func]
#[scope(
    scope.define("decode", csv_decode_func());
    scope
)]
pub fn csv(
    /// Path to a CSV file.
    path: Spanned<EcoString>,
    /// The delimiter that separates columns in the CSV file.
    /// Must be a single ASCII character.
    #[named]
    #[default]
    delimiter: Delimiter,
    /// The virtual machine.
    vm: &mut Vm,
) -> SourceResult<Array> {
    let Spanned { v: path, span } = path;
    let id = vm.resolve_path(&path).at(span)?;
    let data = vm.world().file(id).at(span)?;
    csv_decode(Spanned::new(Readable::Bytes(data), span), delimiter)
}

/// Reads structured data from a CSV string/bytes.
///
/// Display: Decode CSV
/// Category: data-loading
#[func]
pub fn csv_decode(
    /// CSV data.
    data: Spanned<Readable>,
    /// The delimiter that separates columns in the CSV file.
    /// Must be a single ASCII character.
    #[named]
    #[default]
    delimiter: Delimiter,
) -> SourceResult<Array> {
    let Spanned { v: data, span } = data;
    let mut builder = csv::ReaderBuilder::new();
    builder.has_headers(false);
    builder.delimiter(delimiter.0 as u8);
    let mut reader = builder.from_reader(data.as_slice());
    let mut array = Array::new();

    for (line, result) in reader.records().enumerate() {
        // Original solution use line from error, but that is incorrect with
        // `has_headers` set to `false`. See issue:
        // https://github.com/BurntSushi/rust-csv/issues/184
        let line = line + 1; // Counting lines from 1
        let row = result.map_err(|err| format_csv_error(err, line)).at(span)?;
        let sub = row.into_iter().map(|field| field.into_value()).collect();
        array.push(Value::Array(sub))
    }

    Ok(array)
}

/// The delimiter to use when parsing CSV files.
pub struct Delimiter(char);

impl Default for Delimiter {
    fn default() -> Self {
        Self(',')
    }
}

cast! {
    Delimiter,
    self => self.0.into_value(),
    v: EcoString => {
        let mut chars = v.chars();
        let first = chars.next().ok_or("delimiter must not be empty")?;
        if chars.next().is_some() {
            bail!("delimiter must be a single character");
        }

        if !first.is_ascii() {
            bail!("delimiter must be an ASCII character");
        }

        Self(first)
    },
}

/// Format the user-facing CSV error message.
fn format_csv_error(error: csv::Error, line: usize) -> EcoString {
    match error.kind() {
        csv::ErrorKind::Utf8 { .. } => "file is not valid utf-8".into(),
        csv::ErrorKind::UnequalLengths { expected_len, len, .. } => {
            eco_format!(
                "failed to parse csv file: found {len} instead of {expected_len} fields in line {line}"
            )
        }
        _ => "failed to parse csv file".into(),
    }
}

/// Reads structured data from a JSON file.
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
/// ## Example { #example }
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
#[func]
#[scope(
    scope.define("decode", json_decode_func());
    scope.define("encode", json_encode_func());
    scope
)]
pub fn json(
    /// Path to a JSON file.
    path: Spanned<EcoString>,
    /// The virtual machine.
    vm: &mut Vm,
) -> SourceResult<Value> {
    let Spanned { v: path, span } = path;
    let id = vm.resolve_path(&path).at(span)?;
    let data = vm.world().file(id).at(span)?;
    json_decode(Spanned::new(Readable::Bytes(data), span))
}

/// Reads structured data from a JSON string/bytes.
///
/// Display: JSON
/// Category: data-loading
#[func]
pub fn json_decode(
    /// JSON data.
    data: Spanned<Readable>,
) -> SourceResult<Value> {
    let Spanned { v: data, span } = data;
    serde_json::from_slice(data.as_slice())
        .map_err(format_json_error)
        .at(span)
}

/// Encodes structured data into a JSON string.
///
/// Display: Encode JSON
/// Category: data-loading
#[func]
pub fn json_encode(
    /// Value to be encoded.
    value: Spanned<Value>,
    /// Whether to pretty print the JSON with newlines and indentation.
    #[named]
    #[default(true)]
    pretty: bool,
) -> SourceResult<Str> {
    let Spanned { v: value, span } = value;
    if pretty {
        serde_json::to_string_pretty(&value)
    } else {
        serde_json::to_string(&value)
    }
    .map(|v| v.into())
    .map_err(|e| eco_format!("failed to encode value as json: {e}"))
    .at(span)
}

/// Format the user-facing JSON error message.
fn format_json_error(error: serde_json::Error) -> EcoString {
    assert!(error.is_syntax() || error.is_eof());
    eco_format!("failed to parse json file: syntax error in line {}", error.line())
}

/// Reads structured data from a TOML file.
///
/// The file must contain a valid TOML table. TOML tables will be converted into
/// Typst dictionaries, and TOML arrays will be converted into Typst arrays.
/// Strings, booleans and datetimes will be converted into the Typst equivalents
/// and numbers will be converted to floats or integers depending on whether
/// they are whole numbers.
///
/// The TOML file in the example consists of a table with the keys `title`,
/// `version`, and `authors`.
///
/// ## Example { #example }
/// ```example
/// #let details = toml("details.toml")
///
/// Title: #details.title \
/// Version: #details.version \
/// Authors: #(details.authors
///   .join(", ", last: " and "))
/// ```
///
/// Display: TOML
/// Category: data-loading
#[func]
#[scope(
    scope.define("decode", toml_decode_func());
    scope.define("encode", toml_encode_func());
    scope
)]
pub fn toml(
    /// Path to a TOML file.
    path: Spanned<EcoString>,
    /// The virtual machine.
    vm: &mut Vm,
) -> SourceResult<Value> {
    let Spanned { v: path, span } = path;
    let id = vm.resolve_path(&path).at(span)?;
    let data = vm.world().file(id).at(span)?;
    toml_decode(Spanned::new(Readable::Bytes(data), span))
}

/// Reads structured data from a TOML string/bytes.
///
/// Display: Decode TOML
/// Category: data-loading
#[func]
pub fn toml_decode(
    /// TOML data.
    data: Spanned<Readable>,
) -> SourceResult<Value> {
    let Spanned { v: data, span } = data;
    let raw = std::str::from_utf8(data.as_slice())
        .map_err(|_| "file is not valid utf-8")
        .at(span)?;
    toml::from_str(raw).map_err(format_toml_error).at(span)
}

/// Encodes structured data into a TOML string.
///
/// Display: Encode TOML
/// Category: data-loading
#[func]
pub fn toml_encode(
    /// Value to be encoded.
    value: Spanned<Value>,
    /// Whether to pretty-print the resulting TOML.
    #[named]
    #[default(true)]
    pretty: bool,
) -> SourceResult<Str> {
    let Spanned { v: value, span } = value;
    if pretty { toml::to_string_pretty(&value) } else { toml::to_string(&value) }
        .map(|v| v.into())
        .map_err(|e| eco_format!("failed to encode value as toml: {e}"))
        .at(span)
}

/// Format the user-facing TOML error message.
fn format_toml_error(error: toml::de::Error) -> EcoString {
    if let Some(range) = error.span() {
        eco_format!(
            "failed to parse toml file: {}, index {}-{}",
            error.message(),
            range.start,
            range.end
        )
    } else {
        eco_format!("failed to parse toml file: {}", error.message())
    }
}

/// Reads structured data from a YAML file.
///
/// The file must contain a valid YAML object or array. YAML mappings will be
/// converted into Typst dictionaries, and YAML sequences will be converted into
/// Typst arrays. Strings and booleans will be converted into the Typst
/// equivalents, null-values (`null`, `~` or empty ``) will be converted into
/// `{none}`, and numbers will be converted to floats or integers depending on
/// whether they are whole numbers. Custom YAML tags are ignored, though the
/// loaded value will still be present.
///
/// The YAML files in the example contain objects with authors as keys,
/// each with a sequence of their own submapping with the keys
/// "title" and "published"
///
/// ## Example { #example }
/// ```example
/// #let bookshelf(contents) = {
///   for (author, works) in contents {
///     author
///     for work in works [
///       - #work.title (#work.published)
///     ]
///   }
/// }
///
/// #bookshelf(
///   yaml("scifi-authors.yaml")
/// )
/// ```
///
/// Display: YAML
/// Category: data-loading
#[func]
#[scope(
    scope.define("decode", yaml_decode_func());
    scope.define("encode", yaml_encode_func());
    scope
)]
pub fn yaml(
    /// Path to a YAML file.
    path: Spanned<EcoString>,
    /// The virtual machine.
    vm: &mut Vm,
) -> SourceResult<Value> {
    let Spanned { v: path, span } = path;
    let id = vm.resolve_path(&path).at(span)?;
    let data = vm.world().file(id).at(span)?;
    yaml_decode(Spanned::new(Readable::Bytes(data), span))
}

/// Reads structured data from a YAML string/bytes.
///
/// Display: Decode YAML
/// Category: data-loading
#[func]
pub fn yaml_decode(
    /// YAML data.
    data: Spanned<Readable>,
) -> SourceResult<Value> {
    let Spanned { v: data, span } = data;
    serde_yaml::from_slice(data.as_slice())
        .map_err(format_yaml_error)
        .at(span)
}

/// Encode structured data into a YAML string.
///
/// Display: Encode YAML
/// Category: data-loading
#[func]
pub fn yaml_encode(
    /// Value to be encoded.
    value: Spanned<Value>,
) -> SourceResult<Str> {
    let Spanned { v: value, span } = value;
    serde_yaml::to_string(&value)
        .map(|v| v.into())
        .map_err(|e| eco_format!("failed to encode value as yaml: {e}"))
        .at(span)
}

/// Format the user-facing YAML error message.
fn format_yaml_error(error: serde_yaml::Error) -> EcoString {
    eco_format!("failed to parse yaml file: {}", error.to_string().trim())
}

/// Reads structured data from a CBOR file.
///
/// The file must contain a valid cbor serialization. Mappings will be
/// converted into Typst dictionaries, and sequences will be converted into
/// Typst arrays. Strings and booleans will be converted into the Typst
/// equivalents, null-values (`null`, `~` or empty ``) will be converted into
/// `{none}`, and numbers will be converted to floats or integers depending on
/// whether they are whole numbers.
///
/// Display: CBOR
/// Category: data-loading
#[func]
#[scope(
    scope.define("decode", cbor_decode_func());
    scope.define("encode", cbor_encode_func());
    scope
)]
pub fn cbor(
    /// Path to a CBOR file.
    path: Spanned<EcoString>,
    /// The virtual machine.
    vm: &mut Vm,
) -> SourceResult<Value> {
    let Spanned { v: path, span } = path;
    let id = vm.resolve_path(&path).at(span)?;
    let data = vm.world().file(id).at(span)?;
    cbor_decode(Spanned::new(data, span))
}

/// Reads structured data from CBOR bytes.
///
/// Display: Decode CBOR
/// Category: data-loading
#[func]
pub fn cbor_decode(
    /// cbor data.
    data: Spanned<Bytes>,
) -> SourceResult<Value> {
    let Spanned { v: data, span } = data;
    ciborium::from_reader(data.as_slice())
        .map_err(|e| eco_format!("failed to parse cbor: {e}"))
        .at(span)
}

/// Encode structured data into CBOR bytes.
///
/// Display: Encode CBOR
/// Category: data-loading
#[func]
pub fn cbor_encode(
    /// Value to be encoded.
    value: Spanned<Value>,
) -> SourceResult<Bytes> {
    let Spanned { v: value, span } = value;
    let mut res = Vec::new();
    ciborium::into_writer(&value, &mut res)
        .map(|_| res.into())
        .map_err(|e| eco_format!("failed to encode value as cbor: {e}"))
        .at(span)
}

/// Reads structured data from an XML file.
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
/// ## Example { #example }
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
///
/// Display: XML
/// Category: data-loading
#[func]
#[scope(
    scope.define("decode", xml_decode_func());
    scope
)]
pub fn xml(
    /// Path to an XML file.
    path: Spanned<EcoString>,
    /// The virtual machine.
    vm: &mut Vm,
) -> SourceResult<Value> {
    let Spanned { v: path, span } = path;
    let id = vm.resolve_path(&path).at(span)?;
    let data = vm.world().file(id).at(span)?;
    xml_decode(Spanned::new(Readable::Bytes(data), span))
}

/// Reads structured data from an XML string/bytes.
///
/// Display: Decode XML
/// Category: data-loading
#[func]
pub fn xml_decode(
    /// XML data.
    data: Spanned<Readable>,
) -> SourceResult<Value> {
    let Spanned { v: data, span } = data;
    let text = std::str::from_utf8(data.as_slice())
        .map_err(FileError::from)
        .at(span)?;
    let document = roxmltree::Document::parse(text).map_err(format_xml_error).at(span)?;
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
fn format_xml_error(error: roxmltree::Error) -> EcoString {
    format_xml_like_error("xml file", error)
}
