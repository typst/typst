use ecow::eco_format;
use typst_syntax::Spanned;

use crate::diag::{At, SourceResult};
use crate::engine::Engine;
use crate::foundations::{func, scope, Str, Value};
use crate::loading::{DataSource, Load, Readable};

/// Reads structured data from a YAML file.
///
/// The file must contain a valid YAML object or array. YAML mappings will be
/// converted into Typst dictionaries, and YAML sequences will be converted into
/// Typst arrays. Strings and booleans will be converted into the Typst
/// equivalents, null-values (`null`, `~` or empty ``) will be converted into
/// `{none}`, and numbers will be converted to floats or integers depending on
/// whether they are whole numbers. Custom YAML tags are ignored, though the
/// loaded value will still be present.
///
/// Be aware that integers larger than 2<sup>63</sup>-1 will be converted to
/// floating point numbers, which may give an approximative value.
///
/// The YAML files in the example contain objects with authors as keys,
/// each with a sequence of their own submapping with the keys
/// "title" and "published"
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
#[func(scope, title = "YAML")]
pub fn yaml(
    engine: &mut Engine,
    /// A path to a YAML file or raw YAML bytes.
    ///
    /// For more details about paths, see the [Paths section]($syntax/#paths).
    source: Spanned<DataSource>,
) -> SourceResult<Value> {
    let data = source.load(engine.world)?;
    serde_yaml::from_slice(data.as_slice())
        .map_err(|err| eco_format!("failed to parse YAML ({err})"))
        .at(source.span)
}

#[scope]
impl yaml {
    /// Reads structured data from a YAML string/bytes.
    ///
    /// This function is deprecated. The [`yaml`] function now accepts bytes
    /// directly.
    #[func(title = "Decode YAML")]
    #[deprecated = "`yaml.decode` is deprecated, directly pass bytes to `yaml` instead"]
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
