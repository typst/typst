use ecow::{eco_format, EcoString};
use typst_syntax::{is_newline, Spanned};

use crate::diag::{At, FileError, SourceResult};
use crate::engine::Engine;
use crate::foundations::{func, scope, Str, Value};
use crate::loading::{DataSource, Load, Readable};

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
    /// The engine.
    engine: &mut Engine,
    /// A path to a TOML file or raw TOML bytes.
    ///
    /// For more details about paths, see the [Paths section]($syntax/#paths).
    source: Spanned<DataSource>,
) -> SourceResult<Value> {
    let data = source.load(engine.world)?;
    let raw = data.as_str().map_err(FileError::from).at(source.span)?;
    ::toml::from_str(raw)
        .map_err(|err| format_toml_error(err, raw))
        .at(source.span)
}

#[scope]
impl toml {
    /// Reads structured data from a TOML string/bytes.
    ///
    /// This function is deprecated. The [`toml`] function now accepts bytes
    /// directly.
    #[func(title = "Decode TOML")]
    pub fn decode(
        /// The engine.
        engine: &mut Engine,
        /// TOML data.
        data: Spanned<Readable>,
    ) -> SourceResult<Value> {
        toml(engine, data.map(Readable::into_source))
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
