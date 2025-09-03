use ecow::eco_format;
use typst_syntax::Spanned;

use crate::diag::{At, LoadError, LoadedWithin, ReportPos, SourceResult};
use crate::engine::Engine;
use crate::foundations::{Dict, Str, func, scope};
use crate::loading::{DataSource, Load, Readable};

/// Reads structured data from a TOML file.
///
/// The file must contain a valid TOML table. The TOML values will be converted
/// into corresponding Typst values listed in the [table below](#conversion).
///
/// The function returns a dictionary representing the TOML table.
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
///
/// # Conversion details { #conversion }
///
/// First of all, TOML documents are tables. Other values must be put in a table
/// to be encoded or decoded.
///
/// | TOML value | Converted into Typst |
/// | ---------- | -------------------- |
/// | string     | [`str`]              |
/// | integer    | [`int`]              |
/// | float      | [`float`]            |
/// | boolean    | [`bool`]             |
/// | datetime   | [`datetime`]         |
/// | array      | [`array`]            |
/// | table      | [`dictionary`]       |
///
/// Be aware that **TOML integers** larger than 2<sup>63</sup>-1 or smaller than
/// -2<sup>63</sup> cannot be represented losslessly in Typst, and an error will
/// be thrown according to the [specification](https://toml.io/en/v1.0.0#integer).
///
/// | Typst value                           | Converted into TOML            |
/// | ------------------------------------- | ------------------------------ |
/// | types that can be converted from TOML | corresponding TOML value       |
/// | `{none}`                              | ignored                        |
/// | [`bytes`]                             | string via [`repr`]            |
/// | [`symbol`]                            | string                         |
/// | [`content`]                           | a table describing the content |
/// | other types ([`length`], etc.)        | string via [`repr`]            |
///
/// - **Bytes** are not encoded as TOML arrays for performance and readability
///   reasons. Consider using [`cbor.encode`] for binary data.
///
/// - The **`repr`** function is [for debugging purposes only]($repr/#debugging-only),
///   and its output is not guaranteed to be stable across typst versions.
#[func(scope, title = "TOML")]
pub fn toml(
    engine: &mut Engine,
    /// A [path]($syntax/#paths) to a TOML file or raw TOML bytes.
    source: Spanned<DataSource>,
) -> SourceResult<Dict> {
    let loaded = source.load(engine.world)?;
    let raw = loaded.data.as_str().within(&loaded)?;
    ::toml::from_str(raw).map_err(format_toml_error).within(&loaded)
}

#[scope]
impl toml {
    /// Reads structured data from a TOML string/bytes.
    #[func(title = "Decode TOML")]
    #[deprecated(
        message = "`toml.decode` is deprecated, directly pass bytes to `toml` instead",
        until = "0.15.0"
    )]
    pub fn decode(
        engine: &mut Engine,
        /// TOML data.
        data: Spanned<Readable>,
    ) -> SourceResult<Dict> {
        toml(engine, data.map(Readable::into_source))
    }

    /// Encodes structured data into a TOML string.
    #[func(title = "Encode TOML")]
    pub fn encode(
        /// Value to be encoded.
        ///
        /// TOML documents are tables. Therefore, only dictionaries are suitable.
        value: Spanned<Dict>,
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
fn format_toml_error(error: ::toml::de::Error) -> LoadError {
    let pos = error.span().map(ReportPos::from).unwrap_or_default();
    LoadError::new(pos, "failed to parse TOML", error.message())
}
