use ecow::eco_format;
use typst_syntax::Spanned;

use crate::diag::{At, SourceResult};
use crate::engine::Engine;
use crate::foundations::{func, scope, Bytes, Value};
use crate::loading::{DataSource, Load};

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
    engine: &mut Engine,
    /// A [path]($syntax/#paths) to a CBOR file or raw CBOR bytes.
    source: Spanned<DataSource>,
) -> SourceResult<Value> {
    let data = source.load(engine.world)?;
    ciborium::from_reader(data.bytes.as_slice())
        .map_err(|err| eco_format!("failed to parse CBOR ({err})"))
        .at(source.span)
}

#[scope]
impl cbor {
    /// Reads structured data from CBOR bytes.
    #[func(title = "Decode CBOR")]
    #[deprecated = "`cbor.decode` is deprecated, directly pass bytes to `cbor` instead"]
    pub fn decode(
        engine: &mut Engine,
        /// CBOR data.
        data: Spanned<Bytes>,
    ) -> SourceResult<Value> {
        cbor(engine, data.map(DataSource::Bytes))
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
            .map(|_| Bytes::new(res))
            .map_err(|err| eco_format!("failed to encode value as CBOR ({err})"))
            .at(span)
    }
}
