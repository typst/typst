use ecow::eco_format;
use typst_syntax::Spanned;

use crate::diag::{At, LoadError, LoadedWithin, ReportPos, SourceResult};
use crate::engine::Engine;
use crate::foundations::{Str, Value, func, scope};
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
    engine: &mut Engine,
    /// A [path]($syntax/#paths) to a TOML file or raw TOML bytes.
    source: Spanned<DataSource>,
) -> SourceResult<Value> {
    let loaded = source.load(engine.world)?;
    let raw = loaded.data.as_str().within(&loaded)?;
    ::toml::from_str(raw).map_err(format_toml_error).within(&loaded)
}

#[scope]
impl toml {
    /// Reads structured data from a TOML string/bytes.
    #[func(title = "Decode TOML")]
    #[deprecated = "`toml.decode` is deprecated, directly pass bytes to `toml` instead"]
    pub fn decode(
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
fn format_toml_error(error: ::toml::de::Error) -> LoadError {
    let pos = error.span().map(ReportPos::from).unwrap_or_default();
    LoadError::new(pos, "failed to parse TOML", error.message())
}
