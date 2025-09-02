use ecow::eco_format;
use typst_syntax::Spanned;

use crate::diag::{At, LineCol, LoadError, LoadedWithin, ReportPos, SourceResult};
use crate::engine::Engine;
use crate::foundations::{Str, Value, func, scope};
use crate::loading::{DataSource, Load, Readable};

/// Reads structured data from a YAML file.
///
/// The file must contain a valid YAML object or array. The YAML values will be
/// converted into corresponding Typst values listed in the [table below](#conversion).
///
/// The function returns a dictionary, an array or, depending on the YAML file,
/// another YAML data type.
///
/// The YAML files in the example contain objects with authors as keys,
/// each with a sequence of their own submapping with the keys
/// "title" and "published".
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
///
/// # Conversion details { #conversion }
///
/// | YAML value                             | Converted into Typst |
/// | -------------------------------------- | -------------------- |
/// | null-values (`null`, `~` or empty ` `) | `{none}`             |
/// | boolean                                | [`bool`]             |
/// | number                                 | [`float`] or [`int`] |
/// | string                                 | [`str`]              |
/// | sequence                               | [`array`]            |
/// | mapping                                | [`dictionary`]       |
///
/// - In most cases, **YAML numbers** will be converted to floats or integers
///   depending on whether they are whole numbers. However, be aware that integers
///   larger than 2<sup>63</sup>-1 or smaller than -2<sup>63</sup> will be
///   approximated as floating-point numbers.
///
/// - **Custom YAML tags** are ignored, though the loaded value will still be present.
///
/// | Typst value                           | Converted into YAML              |
/// | ------------------------------------- | -------------------------------- |
/// | types that can be converted from YAML | corresponding YAML value         |
/// | [`bytes`]                             | string via [`repr`]              |
/// | [`symbol`]                            | string                           |
/// | [`content`]                           | a mapping describing the content |
/// | other types ([`length`], etc.)        | string via [`repr`]              |
///
/// - **Bytes** are not encoded as YAML sequences for performance reasons.
///   Consider using [`cbor.encode`] for binary data.
///
/// - The **`repr`** function is [for debugging purposes only]($repr/#debugging-only),
///   and its output is not guaranteed to be stable across typst versions.
#[func(scope, title = "YAML")]
pub fn yaml(
    engine: &mut Engine,
    /// A [path]($syntax/#paths) to a YAML file or raw YAML bytes.
    source: Spanned<DataSource>,
) -> SourceResult<Value> {
    let loaded = source.load(engine.world)?;
    serde_yaml::from_slice(loaded.data.as_slice())
        .map_err(format_yaml_error)
        .within(&loaded)
}

#[scope]
impl yaml {
    /// Reads structured data from a YAML string/bytes.
    #[func(title = "Decode YAML")]
    #[deprecated(
        message = "`yaml.decode` is deprecated, directly pass bytes to `yaml` instead",
        until = "0.15.0"
    )]
    pub fn decode(
        engine: &mut Engine,
        /// YAML data.
        data: Spanned<Readable>,
    ) -> SourceResult<Value> {
        yaml(engine, data.map(Readable::into_source))
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

/// Format the user-facing YAML error message.
pub fn format_yaml_error(error: serde_yaml::Error) -> LoadError {
    let pos = error
        .location()
        .map(|loc| {
            let line_col = LineCol::one_based(loc.line(), loc.column());
            let range = loc.index()..loc.index();
            ReportPos::full(range, line_col)
        })
        .unwrap_or_default();
    LoadError::new(pos, "failed to parse YAML", error)
}
