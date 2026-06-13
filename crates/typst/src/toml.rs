use ecow::{EcoString, eco_format};
use typst_library::{
    diag::{LoadError, ReportTextPos},
    foundations::{Dict, Str},
};

pub(crate) fn decode(data: &str) -> Result<Dict, LoadError> {
    ::toml::from_str(data).map_err(format_toml_error)
}

pub(crate) fn encode(value: Dict, pretty: bool) -> Result<Str, EcoString> {
    if pretty { ::toml::to_string_pretty(&value) } else { ::toml::to_string(&value) }
        .map(|v| v.into())
        .map_err(|err| eco_format!("failed to encode value as TOML ({err})"))
}

/// Format the user-facing TOML error message.
fn format_toml_error(error: ::toml::de::Error) -> LoadError {
    let pos = error.span().map(ReportTextPos::from).unwrap_or_default();
    LoadError::text(pos, "failed to parse TOML", error.message())
}
