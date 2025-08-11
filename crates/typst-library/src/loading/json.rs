use ecow::eco_format;
use typst_syntax::Spanned;

use crate::diag::{At, LineCol, LoadError, LoadedWithin, SourceResult};
use crate::engine::Engine;
use crate::foundations::{Str, Value, func, scope};
use crate::loading::{DataSource, Load, Readable};

/// Reads structured data from a JSON file.
///
/// The file must contain a valid JSON value, such as object or array. The JSON
/// values will be converted into corresponding Typst values listed in the
/// [table below](#conversion).
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
///
/// # Conversion details { #conversion }
///
/// | JSON value | Converted into Typst |
/// | ---------- | -------------------- |
/// | `null`     | `{none}`             |
/// | bool       | [`bool`]             |
/// | number     | [`float`] or [`int`] |
/// | string     | [`str`]              |
/// | array      | [`array`]            |
/// | object     | [`dictionary`]       |
///
/// In most cases, **JSON numbers** will be converted to floats or integers
/// depending on whether they are whole numbers. However, be aware that integers
/// larger than 2<sup>63</sup>-1 or smaller than -2<sup>63</sup> will be converted
/// to floating point numbers, which may result in an approximative value.
///
/// | Typst value                           | Converted into JSON              |
/// | ------------------------------------- | -------------------------------- |
/// | types that can be converted from JSON | corresponding JSON value         |
/// | [`bytes`]                             | string via [`repr`]              |
/// | [`symbol`]                            | string                           |
/// | [`content`]                           | an object describing the content |
/// | other types ([`length`], etc.)        | string via [`repr`]              |
///
/// - **Bytes** are not encoded as JSON arrays for performance reasons. Consider
///   using [`cbor.encode`] for binary data.
///
/// - The **`repr`** function is [for debugging purposes only]($repr/#debugging-only),
///   and its output is not guaranteed to be stable across typst versions.
#[func(scope, title = "JSON")]
pub fn json(
    engine: &mut Engine,
    /// A [path]($syntax/#paths) to a JSON file or raw JSON bytes.
    source: Spanned<DataSource>,
) -> SourceResult<Value> {
    let loaded = source.load(engine.world)?;
    serde_json::from_slice(loaded.data.as_slice())
        .map_err(|err| {
            let pos = LineCol::one_based(err.line(), err.column());
            LoadError::new(pos, "failed to parse JSON", err)
        })
        .within(&loaded)
}

#[scope]
impl json {
    /// Reads structured data from a JSON string/bytes.
    #[func(title = "Decode JSON")]
    #[deprecated(
        message = "`json.decode` is deprecated, directly pass bytes to `json` instead",
        until = "0.15.0"
    )]
    pub fn decode(
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
