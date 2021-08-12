//! Diagnostics.

use serde::{Deserialize, Serialize};

use crate::source::SourceId;
use crate::syntax::Span;

/// The result type for typesetting and all its subpasses.
pub type TypResult<T> = Result<T, Box<Vec<Error>>>;

/// A result type with a string error message.
pub type StrResult<T> = Result<T, String>;

/// An error in a source file.
#[derive(Debug, Clone, Eq, PartialEq, Ord, PartialOrd, Serialize, Deserialize)]
pub struct Error {
    /// The id of the source file that contains the error.
    pub source: SourceId,
    /// The erroneous location in the source code.
    pub span: Span,
    /// A diagnostic message describing the problem.
    pub message: String,
    /// The trace of function calls leading to the error.
    pub trace: Vec<(SourceId, Span, Tracepoint)>,
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
    pub fn new(
        source: SourceId,
        span: impl Into<Span>,
        message: impl Into<String>,
    ) -> Self {
        Self {
            source,
            span: span.into(),
            trace: vec![],
            message: message.into(),
        }
    }

    /// Create a boxed vector containing one error. The return value is suitable
    /// as the `Err` variant of a [`TypResult`].
    pub fn boxed(
        source: SourceId,
        span: impl Into<Span>,
        message: impl Into<String>,
    ) -> Box<Vec<Self>> {
        Box::new(vec![Self::new(source, span, message)])
    }

    /// Create a closure that contains the positional information for an error
    /// and just needs the message to yield a vec-boxed error.
    ///
    /// This is useful in to convert from [`StrResult`] to a [`TypResult`] using
    /// [`map_err`](Result::map_err).
    pub fn at<S: Into<String>>(
        source: SourceId,
        span: impl Into<Span>,
    ) -> impl FnOnce(S) -> Box<Vec<Self>> {
        move |message| Self::boxed(source, span, message)
    }
}

/// Early-return with a vec-boxed [`Error`].
macro_rules! bail {
    ($source:expr, $span:expr, $message:expr $(,)?) => {
        return Err(Box::new(vec![$crate::diag::Error::new(
            $source, $span, $message,
        )]));
    };

    ($source:expr, $span:expr, $fmt:expr, $($arg:expr),+ $(,)?) => {
        bail!($source, $span, format!($fmt, $($arg),+));
    };
}
