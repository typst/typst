use ecow::{eco_format, EcoString};
use typst_syntax::Spanned;

use crate::diag::{At, SourceResult};
use crate::engine::Engine;
use crate::foundations::{func, scope, Bytes, Value};
use crate::World;

/// Reads structured data from a CBOR file.
///
/// The file must contain a valid CBOR serialization. Mappings will be
/// converted into Typst dictionaries, and sequences will be converted into
/// Typst arrays. Strings and booleans will be converted into the Typst
/// equivalents, null-values (`null`, `~` or empty ``) will be converted into
/// `{none}`, and numbers will be converted to floats or integers depending on
/// whether they are whole numbers.
///
/// Be aware that integers larger than 2<sup>63</sup>-1 will be converted to
/// floating point numbers, which may result in an approximative value.
#[func(scope, title = "CBOR")]
pub fn cbor(
    /// The engine.
    engine: &mut Engine,
    /// Path to a CBOR file.
    ///
    /// For more details, see the [Paths section]($syntax/#paths).
    path: Spanned<EcoString>,
) -> SourceResult<Value> {
    let Spanned { v: path, span } = path;
    let id = span.resolve_path(&path).at(span)?;
    let data = engine.world.file(id).at(span)?;
    cbor::decode(Spanned::new(data, span))
}

#[scope]
impl cbor {
    /// Reads structured data from CBOR bytes.
    #[func(title = "Decode CBOR")]
    pub fn decode(
        /// cbor data.
        data: Spanned<Bytes>,
    ) -> SourceResult<Value> {
        let Spanned { v: data, span } = data;
        ciborium::from_reader(data.as_slice())
            .map_err(|err| eco_format!("failed to parse CBOR ({err})"))
            .at(span)
    }

    /// Encode structured data into CBOR bytes.
    #[func(title = "Encode CBOR")]
    pub fn encode(
        /// Value to be encoded.
        value: Spanned<Value>,
    ) -> SourceResult<Bytes> {
        let Spanned { v: value, span } = value;
        let mut res = Vec::new();
        ciborium::into_writer(&value, &mut res)
            .map(|_| res.into())
            .map_err(|err| eco_format!("failed to encode value as CBOR ({err})"))
            .at(span)
    }
}
