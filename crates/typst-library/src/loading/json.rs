use ecow::eco_format;
use typst_syntax::Spanned;

use crate::diag::{At, LineCol, LoadError, LoadedWithin, SourceResult, bail};
use crate::engine::Engine;
use crate::foundations::{Array, Str, Value, func, scope};
use crate::loading::{DataSource, Load};

/// Reads structured data from a JSON file.
///
/// The file must contain a valid JSON value, such as object or array. The JSON
/// values will be converted into corresponding Typst values as listed in the
/// @json:conversion[table below].
///
/// The function returns a dictionary, an array or, depending on the JSON file,
/// another JSON data type.
///
/// With `{lines: true}`, reads #link("https://jsonlines.org/")[JSON Lines]
/// (JSONL or NDJSON) instead and returns an array containing the value from each
/// line.
///
/// The JSON files in the example contain objects with the keys `temperature`,
/// `unit`, and `weather`.
///
/// = Example <example>
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
/// = #short-or-long[Conversion][Conversion details] <conversion>
/// #docs-table(
///   table.header[JSON value][Converted into Typst],
///
///   [`null`],
///   [`{none}`],
///
///   [bool],
///   [@bool],
///
///   [number],
///   [@float or @int],
///
///   [string],
///   [@str],
///
///   [array],
///   [@array],
///
///   [object],
///   [@dictionary],
/// )
///
/// #docs-table(
///   table.header[Typst value][Converted into JSON],
///
///   [types that can be converted from JSON],
///   [corresponding JSON value],
///
///   [@bytes],
///   [string via @repr],
///
///   [@symbol],
///   [string],
///
///   [@content],
///   [an object describing the content],
///
///   [other types (@length, etc.)],
///   [string via @repr],
/// )
///
/// == Notes <notes>
/// - In most cases, JSON numbers will be converted to floats or integers
///   depending on whether they are whole numbers. However, be aware that
///   integers larger than 2#super[63]-1 or smaller than -2#super[63] will be
///   converted to floating-point numbers, which may result in an approximative
///   value.
///
/// - Bytes are not encoded as JSON arrays for performance and readability
///   reasons. Consider using @cbor.encode for binary data.
///
/// - The `repr` function is @repr:debugging-only[for debugging purposes only],
///   and its output is not guaranteed to be stable across Typst versions.
#[func(scope, title = "JSON", since = "forever")]
pub fn json(
    engine: &mut Engine,
    /// A path to a JSON file or raw JSON bytes.
    source: Spanned<DataSource>,
    /// Whether to read one JSON value per line and collect the values into an
    /// array. Each line must contain a complete JSON value; blank lines are
    /// invalid. Both LF and CRLF line endings are supported, and the final
    /// newline is optional. An empty file produces an empty array.
    ///
    /// ```typ
    /// #let records = json(
    ///   "results.jsonl", lines: true,
    /// )
    /// ```
    #[named]
    #[default(false)]
    lines: bool,
) -> SourceResult<Value> {
    let loaded = source.load(engine.world)?;
    let raw = loaded.data.as_slice();
    // If the file starts with a UTF-8 Byte Order Mark (BOM), return a
    // friendly error message.
    if raw.starts_with(b"\xef\xbb\xbf") {
        bail!(
            LoadError::text(
                LineCol::one_based(1, 1),
                "failed to parse JSON",
                "unexpected Byte Order Mark",
            )
            .within(&loaded)
            .with_hint("JSON requires UTF-8 without a BOM")
        );
    }

    if lines {
        let mut array = Array::new();
        for (index, line) in raw.split_inclusive(|&b| b == b'\n').enumerate() {
            let line = line.strip_suffix(b"\n").unwrap_or(line);
            let value = serde_json::from_slice(line)
                .map_err(|err| {
                    let pos = LineCol::one_based(index + 1, err.column().max(1));
                    // The parser's location is relative to the individual line.
                    // Let the loading diagnostic report the position in the file.
                    let message = err.to_string();
                    let message = message
                        .rsplit_once(" at line ")
                        .map_or(message.as_str(), |(message, _)| message);
                    LoadError::text(pos, "failed to parse JSON Lines", message)
                })
                .within(&loaded)?;
            array.push(value);
        }
        return Ok(Value::Array(array));
    }

    serde_json::from_slice(raw)
        .map_err(|err| {
            let pos = LineCol::one_based(err.line(), err.column());
            LoadError::text(pos, "failed to parse JSON", err)
        })
        .within(&loaded)
}

#[scope]
impl json {
    /// Encodes structured data into a JSON string.
    #[func(title = "Encode JSON", since = "0.8.0")]
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
