use typst::diag::{format_xml_like_error, FileError};
use typst::eval::{Bytes, Datetime};

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
    let id = vm.location().join(&path).at(span)?;
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
    let id = vm.location().join(&path).at(span)?;
    let data = vm.world().file(id).at(span)?;

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
pub fn json(
    /// Path to a JSON file.
    path: Spanned<EcoString>,
    /// The virtual machine.
    vm: &mut Vm,
) -> SourceResult<Value> {
    let Spanned { v: path, span } = path;
    let id = vm.location().join(&path).at(span)?;
    let data = vm.world().file(id).at(span)?;
    let value: serde_json::Value =
        serde_json::from_slice(&data).map_err(format_json_error).at(span)?;
    Ok(convert_json(value))
}

/// Convert a JSON value to a Typst value.
fn convert_json(value: serde_json::Value) -> Value {
    match value {
        serde_json::Value::Null => Value::None,
        serde_json::Value::Bool(v) => v.into_value(),
        serde_json::Value::Number(v) => match v.as_i64() {
            Some(int) => int.into_value(),
            None => v.as_f64().unwrap_or(f64::NAN).into_value(),
        },
        serde_json::Value::String(v) => v.into_value(),
        serde_json::Value::Array(v) => {
            v.into_iter().map(convert_json).collect::<Array>().into_value()
        }
        serde_json::Value::Object(v) => v
            .into_iter()
            .map(|(key, value)| (key.into(), convert_json(value)))
            .collect::<Dict>()
            .into_value(),
    }
}

/// Format the user-facing JSON error message.
fn format_json_error(error: serde_json::Error) -> EcoString {
    assert!(error.is_syntax() || error.is_eof());
    eco_format!("failed to parse json file: syntax error in line {}", error.line())
}

/// Reads structured data from a TOML file.
///
/// The file must contain a valid TOML table. TOML tables will be
/// converted into Typst dictionaries, and TOML arrays will be converted into
/// Typst arrays. Strings and booleans will be converted into the Typst
/// equivalents and numbers will be converted to floats or integers depending on
/// whether they are whole numbers. For the time being, datetimes will be
/// converted to strings as Typst does not have a built-in datetime yet.
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
pub fn toml(
    /// Path to a TOML file.
    path: Spanned<EcoString>,
    /// The virtual machine.
    vm: &mut Vm,
) -> SourceResult<Value> {
    let Spanned { v: path, span } = path;
    let id = vm.location().join(&path).at(span)?;
    let data = vm.world().file(id).at(span)?;

    let raw = std::str::from_utf8(&data)
        .map_err(|_| "file is not valid utf-8")
        .at(span)?;

    let value: toml::Value = toml::from_str(raw).map_err(format_toml_error).at(span)?;
    Ok(convert_toml(value))
}

/// Convert a TOML value to a Typst value.
fn convert_toml(value: toml::Value) -> Value {
    match value {
        toml::Value::String(v) => v.into_value(),
        toml::Value::Integer(v) => v.into_value(),
        toml::Value::Float(v) => v.into_value(),
        toml::Value::Boolean(v) => v.into_value(),
        toml::Value::Array(v) => {
            v.into_iter().map(convert_toml).collect::<Array>().into_value()
        }
        toml::Value::Table(v) => v
            .into_iter()
            .map(|(key, value)| (key.into(), convert_toml(value)))
            .collect::<Dict>()
            .into_value(),
        toml::Value::Datetime(v) => match (v.date, v.time) {
            (None, None) => Value::None,
            (Some(date), None) => {
                Datetime::from_ymd(date.year as i32, date.month, date.day).into_value()
            }
            (None, Some(time)) => {
                Datetime::from_hms(time.hour, time.minute, time.second).into_value()
            }
            (Some(date), Some(time)) => Datetime::from_ymd_hms(
                date.year as i32,
                date.month,
                date.day,
                time.hour,
                time.minute,
                time.second,
            )
            .into_value(),
        },
    }
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
/// whether they are whole numbers.
///
/// Note that mapping keys that are not a string cause the entry to be
/// discarded.
///
/// Custom YAML tags are ignored, though the loaded value will still be
/// present.
///
/// The function returns a dictionary or value or an array, depending on
/// the YAML file.
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
pub fn yaml(
    /// Path to a YAML file.
    path: Spanned<EcoString>,
    /// The virtual machine.
    vm: &mut Vm,
) -> SourceResult<Value> {
    let Spanned { v: path, span } = path;
    let id = vm.location().join(&path).at(span)?;
    let data = vm.world().file(id).at(span)?;
    let value: serde_yaml::Value =
        serde_yaml::from_slice(&data).map_err(format_yaml_error).at(span)?;
    Ok(convert_yaml(value))
}

/// Convert a YAML value to a Typst value.
fn convert_yaml(value: serde_yaml::Value) -> Value {
    match value {
        serde_yaml::Value::Null => Value::None,
        serde_yaml::Value::Bool(v) => v.into_value(),
        serde_yaml::Value::Number(v) => match v.as_i64() {
            Some(int) => int.into_value(),
            None => v.as_f64().unwrap_or(f64::NAN).into_value(),
        },
        serde_yaml::Value::String(v) => v.into_value(),
        serde_yaml::Value::Sequence(v) => {
            v.into_iter().map(convert_yaml).collect::<Array>().into_value()
        }
        serde_yaml::Value::Mapping(v) => v
            .into_iter()
            .map(|(key, value)| (convert_yaml_key(key), convert_yaml(value)))
            .filter_map(|(key, value)| key.map(|key| (key, value)))
            .collect::<Dict>()
            .into_value(),
    }
}

/// Converts an arbitrary YAML mapping key into a Typst Dict Key.
/// Currently it only does so for strings, everything else
/// returns None
fn convert_yaml_key(key: serde_yaml::Value) -> Option<Str> {
    match key {
        serde_yaml::Value::String(v) => Some(Str::from(v)),
        _ => None,
    }
}

/// Format the user-facing YAML error message.
fn format_yaml_error(error: serde_yaml::Error) -> EcoString {
    eco_format!("failed to parse yaml file: {}", error.to_string().trim())
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
pub fn xml(
    /// Path to an XML file.
    path: Spanned<EcoString>,
    /// The virtual machine.
    vm: &mut Vm,
) -> SourceResult<Value> {
    let Spanned { v: path, span } = path;
    let id = vm.location().join(&path).at(span)?;
    let data = vm.world().file(id).at(span)?;
    let text = std::str::from_utf8(&data).map_err(FileError::from).at(span)?;
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
