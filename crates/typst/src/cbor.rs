use ciborium::de::Error;
use ecow::{EcoString, eco_format};
use typst_library::{diag::LoadError, foundations::Value};

pub(crate) fn decode(data: &[u8]) -> Result<Value, LoadError> {
    ciborium::from_reader(data).map_err(|error| {
        // Format a user-facing error encountered while parsing a CBOR file
        // ([`ciborium::de::Error`]'s [`Display`](std::fmt::Display) implementation
        // just forwards to [`Debug`]).
        LoadError::binary(
            "failed to parse CBOR",
            typst_utils::display(|f| match &error {
                Error::Io(e) => write!(f, "IO error: {e}"),
                Error::Syntax(_) => f.write_str("syntax error"),
                Error::Semantic(_, s) => f.write_str(s),
                Error::RecursionLimitExceeded => f.write_str("recursion limit exceeded"),
            }),
        )
    })
}

pub(crate) fn encode(value: Value) -> Result<Vec<u8>, EcoString> {
    let mut res = Vec::new();
    ciborium::into_writer(&value, &mut res)
        .map(|_| res)
        .map_err(|err| eco_format!("failed to encode value as CBOR ({err})"))
}
