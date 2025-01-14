use ecow::eco_format;
use typst_syntax::Spanned;

use crate::diag::{At, SourceResult};
use crate::engine::Engine;
use crate::foundations::{func, scope, Str, Value};
use crate::loading::{DataSource, Load, Readable};

/// Reads structured data from a JSON file.
///
/// The file must contain a valid JSON value, such as object or array. JSON
/// objects will be converted into Typst dictionaries, and JSON arrays will be
/// converted into Typst arrays. Strings and booleans will be converted into the
/// Typst equivalents, `null` will be converted into `{none}`, and numbers will
/// be converted to floats or integers depending on whether they are whole
/// numbers.
///
/// Be aware that integers larger than 2<sup>63</sup>-1 will be converted to
/// floating point numbers, which may result in an approximative value.
///
/// The function returns a dictionary, an array or, depending on the JSON file,
/// another JSON data type.
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
    /// The engine.
    engine: &mut Engine,
    /// Path to a JSON file or raw JSON bytes.
    ///
    /// For more details about paths, see the [Paths section]($syntax/#paths).
    source: Spanned<DataSource>,
) -> SourceResult<Value> {
    let data = source.load(engine.world)?;
    serde_json::from_slice(data.as_slice())
        .map_err(|err| eco_format!("failed to parse JSON ({err})"))
        .at(source.span)
}

#[scope]
impl json {
    /// Reads structured data from a JSON string/bytes.
    ///
    /// This function is deprecated. The [`json`] function now accepts bytes
    /// directly.
    #[func(title = "Decode JSON")]
    pub fn decode(
        /// The engine.
        engine: &mut Engine,
        /// JSON data.
        data: Spanned<Readable>,
    ) -> SourceResult<Value> {
        json(engine, data.map(Readable::into_source))
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
