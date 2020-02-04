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

/// Construct an error with formatted message and optionally severity and / or
/// span.
///
/// # Examples
/// ```
/// # use typstc::err;
/// # use typstc::syntax::span::Span;
/// # let span = Span::ZERO;
/// # let value = 0;
///
/// // With span and default severity `Error`.
/// err!(span; "the wrong {}", value);
///
/// // With no span and severity `Warning`.
/// err!(@Warning: span; "non-fatal!");
///
/// // Without span and default severity.
/// err!("no spans here ...");
/// ```
#[macro_export]
macro_rules! err {
    (@$severity:ident: $span:expr; $($args:tt)*) => {
        $crate::syntax::span::Spanned { v: err!(@$severity: $($args)*), span: $span }
    };

    (@$severity:ident: $($args:tt)*) => {
        $crate::error::Error {
            message: format!($($args)*),
            severity: $crate::error::Severity::$severity,
        }
    };

    ($($tts:tt)*) => { err!(@Error: $($tts)*) };
}
