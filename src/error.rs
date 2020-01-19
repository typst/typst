#[derive(Debug, Clone, Eq, PartialEq)]
pub struct Error {
    pub message: String,
}

impl Error {
    pub fn new(message: impl Into<String>) -> Error {
        Error { message: message.into() }
    }
}
