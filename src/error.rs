use serde::Serialize;


#[derive(Debug, Clone, Eq, PartialEq, Serialize)]
pub struct Error {
    pub message: String,
    pub severity: Severity,
}

impl Error {
    pub fn new(message: impl Into<String>, severity: Severity) -> Error {
        Error { message: message.into(), severity }
    }
}

#[derive(Debug, Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Serialize)]
pub enum Severity {
    Warning,
    Error,
}
