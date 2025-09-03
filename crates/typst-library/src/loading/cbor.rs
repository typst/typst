use ecow::eco_format;
use typst_syntax::Spanned;

use crate::diag::{At, SourceResult};
use crate::engine::Engine;
use crate::foundations::{Bytes, Value, func, scope};
use crate::loading::{DataSource, Load};

/// Reads structured data from a CBOR file.
///
/// The file must contain a valid CBOR serialization. The CBOR values will be
/// converted into corresponding Typst values listed in the
/// [table below](#conversion).
///
/// The function returns a dictionary, an array or, depending on the CBOR file,
/// another CBOR data type.
///
/// # Conversion details { #conversion }
///
/// | CBOR value | Converted into Typst   |
/// | ---------- | ---------------------- |
/// | integer    | [`int`] (or [`float`]) |
/// | bytes      | [`bytes`]              |
/// | float      | [`float`]              |
/// | text       | [`str`]                |
/// | bool       | [`bool`]               |
/// | null       | `{none}`               |
/// | array      | [`array`]              |
/// | map        | [`dictionary`]         |
///
/// - Be aware that **CBOR integers** larger than 2<sup>63</sup>-1 or smaller
///   than -2<sup>63</sup> will be converted to floating point numbers, which
///   may result in an approximative value.
///
/// - **CBOR tags** are not supported, and an error will be thrown.
///
/// | Typst value                           | Converted into CBOR          |
/// | ------------------------------------- | ---------------------------- |
/// | types that can be converted from CBOR | corresponding CBOR value     |
/// | [`symbol`]                            | text                         |
/// | [`content`]                           | a map describing the content |
/// | other types ([`length`], etc.)        | text via [`repr`]            |
///
/// Note that the **`repr`** function is [for debugging purposes only]($repr/#debugging-only),
/// and its output is not guaranteed to be stable across typst versions.
#[func(scope, title = "CBOR")]
pub fn cbor(
    engine: &mut Engine,
    /// A [path]($syntax/#paths) to a CBOR file or raw CBOR bytes.
    source: Spanned<DataSource>,
) -> SourceResult<Value> {
    let loaded = source.load(engine.world)?;
    ciborium::from_reader(loaded.data.as_slice())
        .map_err(|err| eco_format!("failed to parse CBOR ({err})"))
        .at(source.span)
}

#[scope]
impl cbor {
    /// Reads structured data from CBOR bytes.
    #[func(title = "Decode CBOR")]
    #[deprecated(
        message = "`cbor.decode` is deprecated, directly pass bytes to `cbor` instead",
        until = "0.15.0"
    )]
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
