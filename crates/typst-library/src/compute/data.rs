use typst::diag::{format_xml_like_error, FileError};
use typst::eval::Bytes;
use typst::syntax::is_newline;

use crate::prelude::*;

/// Hook up all data loading definitions.
pub(super) fn define(global: &mut Scope) {
    global.category("data-loading");
    global.define_func::<read>();
    global.define_func::<csv>();
    global.define_func::<json>();
    global.define_func::<toml>();
    global.define_func::<yaml>();
    global.define_func::<cbor>();
    global.define_func::<xml>();
}

/// Reads plain text or data from a file.
///
/// By default, the file will be read as UTF-8 and returned as a [string]($str).
///
/// If you specify `{encoding: none}`, this returns raw [bytes]($bytes) instead.
///
/// # Example
/// ```example
/// An example for a HTML file: \
/// #let text = read("data.html")
/// #raw(text, lang: "html")
///
/// Raw bytes:
/// #read("tiger.jpg", encoding: none)
/// ```
#[func]
pub fn read(
    /// The virtual machine.
    vm: &mut Vm,
    /// Path to a file.
    path: Spanned<EcoString>,
    /// The encoding to read the file with.
    ///
    /// If set to `{none}`, this function returns raw bytes.
    #[named]
    #[default(Some(Encoding::Utf8))]
    encoding: Option<Encoding>,
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

/// Reads structured data from a CSV file.
///
/// The CSV file will be read and parsed into a 2-dimensional array of strings:
/// Each row in the CSV file will be represented as an array of strings, and all
/// rows will be collected into a single array. Header rows will not be
/// stripped.
///
/// # Example
/// ```example
/// #let results = csv("data.csv")
///
/// #table(
///   columns: 2,
///   [*Condition*], [*Result*],
///   ..results.flatten(),
/// )
/// ```
#[func(scope, title = "CSV")]
pub fn csv(
    /// The virtual machine.
    vm: &mut Vm,
    /// Path to a CSV file.
    path: Spanned<EcoString>,
    /// The delimiter that separates columns in the CSV file.
    /// Must be a single ASCII character.
    #[named]
    #[default]
    delimiter: Delimiter,
) -> SourceResult<Array> {
    let Spanned { v: path, span } = path;
    let id = vm.resolve_path(&path).at(span)?;
    let data = vm.world().file(id).at(span)?;
    self::csv::decode(Spanned::new(Readable::Bytes(data), span), delimiter)
}

#[scope]
impl csv {
    /// Reads structured data from a CSV string/bytes.
    #[func(title = "Decode CSV")]
    pub fn decode(
        /// CSV data.
        data: Spanned<Readable>,
        /// The delimiter that separates columns in the CSV file.
        /// Must be a single ASCII character.
        #[named]
        #[default]
        delimiter: Delimiter,
    ) -> SourceResult<Array> {
        let Spanned { v: data, span } = data;
        let mut builder = ::csv::ReaderBuilder::new();
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
fn format_csv_error(err: ::csv::Error, line: usize) -> EcoString {
    match err.kind() {
        ::csv::ErrorKind::Utf8 { .. } => "file is not valid utf-8".into(),
        ::csv::ErrorKind::UnequalLengths { expected_len, len, .. } => {
            eco_format!(
                "failed to parse CSV (found {len} instead of \
                 {expected_len} fields in line {line})"
            )
        }
        _ => eco_format!("failed to parse CSV ({err})"),
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
/// # Example
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
#[func(scope, title = "JSON")]
pub fn json(
    /// The virtual machine.
    vm: &mut Vm,
    /// Path to a JSON file.
    path: Spanned<EcoString>,
) -> SourceResult<Value> {
    let Spanned { v: path, span } = path;
    let id = vm.resolve_path(&path).at(span)?;
    let data = vm.world().file(id).at(span)?;
    json::decode(Spanned::new(Readable::Bytes(data), span))
}

#[scope]
impl json {
    /// Reads structured data from a JSON string/bytes.
    #[func(title = "Decode JSON")]
    pub fn decode(
        /// JSON data.
        data: Spanned<Readable>,
    ) -> SourceResult<Value> {
        let Spanned { v: data, span } = data;
        serde_json::from_slice(data.as_slice())
            .map_err(|err| eco_format!("failed to parse JSON ({err})"))
            .at(span)
    }

    /// Encodes structured data into a JSON string.
    #[func(title = "Encode JSON")]
    pub fn encode(
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
        .map_err(|err| eco_format!("failed to encode value as JSON ({err})"))
        .at(span)
    }
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
/// # Example
/// ```example
/// #let details = toml("details.toml")
///
/// Title: #details.title \
/// Version: #details.version \
/// Authors: #(details.authors
///   .join(", ", last: " and "))
/// ```
#[func(scope, title = "TOML")]
pub fn toml(
    /// The virtual machine.
    vm: &mut Vm,
    /// Path to a TOML file.
    path: Spanned<EcoString>,
) -> SourceResult<Value> {
    let Spanned { v: path, span } = path;
    let id = vm.resolve_path(&path).at(span)?;
    let data = vm.world().file(id).at(span)?;
    toml::decode(Spanned::new(Readable::Bytes(data), span))
}

#[scope]
impl toml {
    /// Reads structured data from a TOML string/bytes.
    #[func(title = "Decode TOML")]
    pub fn decode(
        /// TOML data.
        data: Spanned<Readable>,
    ) -> SourceResult<Value> {
        let Spanned { v: data, span } = data;
        let raw = std::str::from_utf8(data.as_slice())
            .map_err(|_| "file is not valid utf-8")
            .at(span)?;
        ::toml::from_str(raw)
            .map_err(|err| format_toml_error(err, raw))
            .at(span)
    }

    /// Encodes structured data into a TOML string.
    #[func(title = "Encode TOML")]
    pub fn encode(
        /// Value to be encoded.
        value: Spanned<Value>,
        /// Whether to pretty-print the resulting TOML.
        #[named]
        #[default(true)]
        pretty: bool,
    ) -> SourceResult<Str> {
        let Spanned { v: value, span } = value;
        if pretty { ::toml::to_string_pretty(&value) } else { ::toml::to_string(&value) }
            .map(|v| v.into())
            .map_err(|err| eco_format!("failed to encode value as TOML ({err})"))
            .at(span)
    }
}

/// Format the user-facing TOML error message.
fn format_toml_error(error: ::toml::de::Error, raw: &str) -> EcoString {
    if let Some(head) = error.span().and_then(|range| raw.get(..range.start)) {
        let line = head.lines().count();
        let column = 1 + head.chars().rev().take_while(|&c| !is_newline(c)).count();
        eco_format!(
            "failed to parse TOML ({} at line {line} column {column})",
            error.message(),
        )
    } else {
        eco_format!("failed to parse TOML ({})", error.message())
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
/// # Example
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
#[func(scope, title = "YAML")]
pub fn yaml(
    /// The virtual machine.
    vm: &mut Vm,
    /// Path to a YAML file.
    path: Spanned<EcoString>,
) -> SourceResult<Value> {
    let Spanned { v: path, span } = path;
    let id = vm.resolve_path(&path).at(span)?;
    let data = vm.world().file(id).at(span)?;
    yaml::decode(Spanned::new(Readable::Bytes(data), span))
}

#[scope]
impl yaml {
    /// Reads structured data from a YAML string/bytes.
    #[func(title = "Decode YAML")]
    pub fn decode(
        /// YAML data.
        data: Spanned<Readable>,
    ) -> SourceResult<Value> {
        let Spanned { v: data, span } = data;
        serde_yaml::from_slice(data.as_slice())
            .map_err(|err| eco_format!("failed to parse YAML ({err})"))
            .at(span)
    }

    /// Encode structured data into a YAML string.
    #[func(title = "Encode YAML")]
    pub fn encode(
        /// Value to be encoded.
        value: Spanned<Value>,
    ) -> SourceResult<Str> {
        let Spanned { v: value, span } = value;
        serde_yaml::to_string(&value)
            .map(|v| v.into())
            .map_err(|err| eco_format!("failed to encode value as YAML ({err})"))
            .at(span)
    }
}

/// Reads structured data from a CBOR file.
///
/// The file must contain a valid cbor serialization. Mappings will be
/// converted into Typst dictionaries, and sequences will be converted into
/// Typst arrays. Strings and booleans will be converted into the Typst
/// equivalents, null-values (`null`, `~` or empty ``) will be converted into
/// `{none}`, and numbers will be converted to floats or integers depending on
/// whether they are whole numbers.
#[func(scope, title = "CBOR")]
pub fn cbor(
    /// The virtual machine.
    vm: &mut Vm,
    /// Path to a CBOR file.
    path: Spanned<EcoString>,
) -> SourceResult<Value> {
    let Spanned { v: path, span } = path;
    let id = vm.resolve_path(&path).at(span)?;
    let data = vm.world().file(id).at(span)?;
    cbor::decode(Spanned::new(data, span))
}

#[scope]
impl cbor {
    /// Reads structured data from CBOR bytes.
    #[func(title = "Decode CBOR")]
    pub fn decode(
        /// cbor data.
        data: Spanned<Bytes>,
    ) -> SourceResult<Value> {
        let Spanned { v: data, span } = data;
        ciborium::from_reader(data.as_slice())
            .map_err(|err| eco_format!("failed to parse CBOR ({err})"))
            .at(span)
    }

    /// Encode structured data into CBOR bytes.
    #[func(title = "Encode CBOR")]
    pub fn encode(
        /// Value to be encoded.
        value: Spanned<Value>,
    ) -> SourceResult<Bytes> {
        let Spanned { v: value, span } = value;
        let mut res = Vec::new();
        ciborium::into_writer(&value, &mut res)
            .map(|_| res.into())
            .map_err(|err| eco_format!("failed to encode value as CBOR ({err})"))
            .at(span)
    }
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
    /// The virtual machine.
    vm: &mut Vm,
    /// Path to an XML file.
    path: Spanned<EcoString>,
) -> SourceResult<Value> {
    let Spanned { v: path, span } = path;
    let id = vm.resolve_path(&path).at(span)?;
    let data = vm.world().file(id).at(span)?;
    xml::decode(Spanned::new(Readable::Bytes(data), span))
}

#[scope]
impl xml {
    /// Reads structured data from an XML string/bytes.
    #[func(title = "Decode XML")]
    pub fn decode(
        /// XML data.
        data: Spanned<Readable>,
    ) -> SourceResult<Value> {
        let Spanned { v: data, span } = data;
        let text = std::str::from_utf8(data.as_slice())
            .map_err(FileError::from)
            .at(span)?;
        let document =
            roxmltree::Document::parse(text).map_err(format_xml_error).at(span)?;
        Ok(convert_xml(document.root()))
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
fn format_xml_error(error: roxmltree::Error) -> EcoString {
    format_xml_like_error("XML", error)
}
