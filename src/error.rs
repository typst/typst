//! Errors in source code.
//!
//! There are no fatal errors in _Typst_. The document will always compile and
//! yield a layout. However, this is a best effort process and bad things will
//! still generate errors and warnings.

use serde::Serialize;
use crate::syntax::span::SpanVec;


/// A spanned list of errors.
pub type Errors = SpanVec<Error>;

/// An error that arose in parsing or layouting.
#[derive(Debug, Clone, Eq, PartialEq, Serialize)]
pub struct Error {
    /// An error message describing the problem.
    pub message: String,
    /// How severe / important the error is.
    pub severity: Severity,
}

/// How severe / important an error is.
#[derive(Debug, Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Serialize)]
pub enum Severity {
    /// Something in the code is not good.
    Warning,
    /// Something in the code is wrong!
    Error,
}

impl Error {
    /// Create a new error from message and severity.
    pub fn new(message: impl Into<String>, severity: Severity) -> Error {
        Error { message: message.into(), severity }
    }
}
