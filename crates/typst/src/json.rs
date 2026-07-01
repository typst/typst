use ecow::{EcoString, eco_format};
use typst_library::{
    diag::{LineCol, LoadError},
    foundations::{Str, Value},
};

pub(crate) fn decode(data: &[u8]) -> Result<Value, LoadError> {
    serde_json::from_slice(data).map_err(|err| {
        let pos = LineCol::one_based(err.line(), err.column());
        LoadError::text(pos, "failed to parse JSON", err)
    })
}

pub(crate) fn encode(value: Value, pretty: bool) -> Result<Str, EcoString> {
    if pretty {
        serde_json::to_string_pretty(&value)
    } else {
        serde_json::to_string(&value)
    }
    .map(|v| v.into())
    .map_err(|err| eco_format!("failed to encode value as JSON ({err})"))
}
