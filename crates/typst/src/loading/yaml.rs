use ecow::{eco_format, EcoString};

use crate::diag::{At, SourceResult};
use crate::engine::Engine;
use crate::foundations::{func, scope, Str, Value};
use crate::loading::Readable;
use crate::syntax::Spanned;
use crate::World;

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
    /// The engine.
    engine: &mut Engine,
    /// Path to a YAML file.
    ///
    /// For more details, see the [Paths section]($syntax/#paths).
    path: Spanned<EcoString>,
) -> SourceResult<Value> {
    let Spanned { v: path, span } = path;
    let id = span.resolve_path(&path).at(span)?;
    let data = engine.world.file(id).at(span)?;
    yaml::decode(Spanned::new(Readable::Bytes(data), span))
}

#[scope]
impl yaml {
    /// Reads structured data from a YAML string/bytes.
    #[func(title = "Decode YAML")]
    pub fn decode(
        /// YAML data.
        data: Spanned<Readable>,
    ) -> SourceResult<Value> {
        let Spanned { v: data, span } = data;
        serde_yaml::from_slice(data.as_slice())
            .map_err(|err| eco_format!("failed to parse YAML ({err})"))
            .at(span)
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
