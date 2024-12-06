use ecow::{eco_format, eco_vec, EcoString};
use typst_syntax::{Span, Spanned};

use crate::diag::{self, At, SourceResult};
use crate::engine::Engine;
use crate::foundations::{func, scope, Str, Value};
use crate::loading::Readable;
use crate::World;

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
    /// Path to a JSON file.
    ///
    /// For more details, see the [Paths section]($syntax/#paths).
    path: Spanned<EcoString>,
) -> SourceResult<Value> {
    let Spanned { v: path, span } = path;
    let id = span.resolve_path(&path).at(span)?;
    let data = engine.world.file(id).at(span)?;

    serde_json::from_slice(data.as_slice()).map_err(|err| {
        let loc = (err.line() - 1, err.column().saturating_sub(1));
        eco_vec![diag::error!(
            Span::from_row_column(
                id,
                loc,
                loc,
                &String::from_utf8_lossy(data.as_slice())
            )
            .unwrap_or(span),
            "failed to parse JSON ({err})"
        )]
    })
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
