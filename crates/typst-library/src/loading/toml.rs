use ecow::{eco_format, EcoString};
use typst_syntax::{is_newline, Spanned};

use crate::diag::{At, SourceResult};
use crate::engine::Engine;
use crate::foundations::{func, scope, Str, Value};
use crate::loading::Readable;
use crate::World;

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
    /// Path to a TOML file.
    ///
    /// For more details, see the [Paths section]($syntax/#paths).
    path: Spanned<EcoString>,
) -> SourceResult<Value> {
    let Spanned { v: path, span } = path;
    let id = span.resolve_path(&path).at(span)?;
    let data = engine.world.file(id).at(span)?;
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
