use ecow::eco_format;
use typst_syntax::Spanned;

use crate::diag::{At, LineCol, LoadError, LoadedWithin, ReportPos, SourceResult};
use crate::engine::Engine;
use crate::foundations::{Str, Value, func, scope};
use crate::loading::{DataSource, Load, Readable};

/// Reads structured data from a YAML file.
///
/// The file must contain a valid YAML object or array. The YAML values will be
/// converted into corresponding Typst values as listed in the
/// [table below](#conversion).
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
/// | Typst value                           | Converted into YAML              |
/// | ------------------------------------- | -------------------------------- |
/// | types that can be converted from YAML | corresponding YAML value         |
/// | [`bytes`]                             | string via [`repr`]              |
/// | [`symbol`]                            | string                           |
/// | [`content`]                           | a mapping describing the content |
/// | other types ([`length`], etc.)        | string via [`repr`]              |
///
/// ## Notes
/// - In most cases, YAML numbers will be converted to floats or integers
///   depending on whether they are whole numbers. However, be aware that
///   integers larger than 2<sup>63</sup>-1 or smaller than -2<sup>63</sup> will
///   be converted to floating-point numbers, which may result in an
///   approximative value.
///
/// - Custom YAML tags are ignored, though the loaded value will still be present.
///
/// - Bytes are not encoded as YAML sequences for performance and readability
///   reasons. Consider using [`cbor.encode`] for binary data.
///
/// - The `repr` function is [for debugging purposes only]($repr/#debugging-only),
///   and its output is not guaranteed to be stable across Typst versions.
#[func(scope, title = "YAML")]
pub fn yaml(
    engine: &mut Engine,
    /// A path to a YAML file or raw YAML bytes.
    source: Spanned<DataSource>,
    /// Whether to perform merging of `<<`` keys into the surrounding mapping
    /// according to the [YAML specification](https://yaml.org/type/merge.html).
    ///
    /// Merged keys cannot be determined until the whole YAML is loaded. If you
    /// don't need this feature, you can disable it for better performance.
    ///
    /// ```example
    /// #let source = bytes(
    ///   ```yaml
    ///   presets:
    ///     - &left { x: 0, y: 0 }
    ///     - &center { x: 1, y: 0 }
    ///     - &small { r: 2 }
    ///     - &large { r: 10 }
    ///
    ///   merge-one:
    ///     <<: *left
    ///     r: 2
    ///     fill: red
    ///
    ///   merge-multiple:
    ///     <<: [ *center, *small ]
    ///     fill: yellow
    ///
    ///   override:
    ///     <<: [ *small, *left, *large ]
    ///     x: 2
    ///     fill: green
    ///   ```.text,
    /// )
    ///
    /// #let (presets, ..data) = yaml(source)
    /// #set page(height: auto, width: auto)
    /// #grid(
    ///   columns: data.len(),
    ///   gutter: 1em,
    ///   ..data
    ///     .values()
    ///     .map(((x, y, r, fill)) => grid.cell(
    ///       x: x,
    ///       y: y,
    ///       circle(
    ///         fill: eval(fill),
    ///         radius: r * 1em,
    ///       ),
    ///     )),
    /// )
    /// ```
    #[named]
    #[default(true)]
    merge_keys: bool,
) -> SourceResult<Value> {
    let loaded = source.load(engine.world)?;
    let mut value = serde_yaml::from_slice(loaded.data.as_slice())
        .map_err(format_yaml_error)
        .within(&loaded)?;

    if merge_keys {
        apply_yaml_merge(&mut value).within(&loaded)?;
    }
    Ok(value)
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
        // Typst 0.14 did not merge keys, so it's false here.
        yaml(engine, data.map(Readable::into_source), false)
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

/// Performs merging of `<<` keys into the surrounding mapping.
/// A copy of [`serde_yaml::Value::apply_merge`] to [`Value`]. (Apache-2.0 license)
fn apply_yaml_merge(value: &mut Value) -> Result<(), LoadError> {
    let yaml_error = |error: &str| {
        // Even serde_yaml can't report positions of these errors, so we give up.
        Err(LoadError::new(ReportPos::default(), "failed to parse YAML", error))
    };

    let mut stack = Vec::new();
    stack.push(value);
    while let Some(node) = stack.pop() {
        match node {
            Value::Dict(dict) => {
                match dict.take("<<") {
                    Ok(Value::Dict(merge)) => {
                        for (k, v) in merge {
                            dict.entry(k).or_insert(v);
                        }
                    }
                    Ok(Value::Array(array)) => {
                        for value in array {
                            match value {
                                Value::Dict(merge) => {
                                    for (k, v) in merge {
                                        dict.entry(k).or_insert(v);
                                    }
                                }
                                Value::Array(_) => {
                                    return yaml_error(
                                        "expected a mapping for merging, but found sequence",
                                    );
                                }
                                _unexpected => {
                                    return yaml_error(
                                        "expected a mapping for merging, but found scalar",
                                    );
                                }
                            }
                        }
                    }
                    Err(_) => {}
                    Ok(_unexpected) => {
                        return yaml_error(
                            "expected a mapping or list of mappings for merging, but found scalar",
                        );
                    }
                }
                stack.extend(dict.values_mut());
            }
            Value::Array(array) => stack.extend(array.iter_mut()),
            _ => {}
        }
    }
    Ok(())
}
