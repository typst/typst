//! Diagnostics.

use serde::{Deserialize, Serialize};

use crate::loading::FileId;
use crate::syntax::Span;

/// The result type for typesetting and all its subpasses.
pub type TypResult<T> = Result<T, Box<Vec<Error>>>;

/// A result type with a string error message.
pub type StrResult<T> = Result<T, String>;

/// An error in a source file.
#[derive(Debug, Clone, Eq, PartialEq, Ord, PartialOrd, Serialize, Deserialize)]
pub struct Error {
    /// The file that contains the error.
    pub file: FileId,
    /// The erroneous location in the source code.
    pub span: Span,
    /// A diagnostic message describing the problem.
    pub message: String,
    /// The trace of function calls leading to the error.
    pub trace: Vec<(FileId, Span, Tracepoint)>,
}

/// A part of an error's [trace](Error::trace).
#[derive(Debug, Clone, Eq, PartialEq, Ord, PartialOrd, Serialize, Deserialize)]
pub enum Tracepoint {
    /// A function call.
    Call(Option<String>),
    /// A module import.
    Import,
}

impl Error {
    /// Create a new, bare error.
    pub fn new(file: FileId, span: impl Into<Span>, message: impl Into<String>) -> Self {
        Self {
            file,
            span: span.into(),
            trace: vec![],
            message: message.into(),
        }
    }

    /// Create a boxed vector containing one error. The return value is suitable
    /// as the `Err` variant of a [`TypResult`].
    pub fn boxed(
        file: FileId,
        span: impl Into<Span>,
        message: impl Into<String>,
    ) -> Box<Vec<Self>> {
        Box::new(vec![Self::new(file, span, message)])
    }

    /// Partially build a vec-boxed error, returning a function that just needs
    /// the message.
    ///
    /// This is useful in to convert from [`StrResult`] to a [`TypResult`] using
    /// [`map_err`](Result::map_err).
    pub fn partial(
        file: FileId,
        span: impl Into<Span>,
    ) -> impl FnOnce(String) -> Box<Vec<Self>> {
        move |message| Self::boxed(file, span, message)
    }
}

/// Early-return with a vec-boxed [`Error`].
#[macro_export]
macro_rules! bail {
    ($file:expr, $span:expr, $message:expr $(,)?) => {
        return Err(Box::new(vec![$crate::diag::Error::new(
            $file, $span, $message,
        )]));
    };

    ($file:expr, $span:expr, $fmt:expr, $($arg:expr),+ $(,)?) => {
        $crate::bail!($file, $span, format!($fmt, $($arg),+));
    };
}
