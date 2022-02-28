//! Diagnostics.

use std::fmt::{self, Display, Formatter};

use crate::syntax::{Span, Spanned};

/// Early-return with a vec-boxed [`Error`].
macro_rules! bail {
    ($span:expr, $message:expr $(,)?) => {
        return Err($crate::diag::Error::boxed($span, $message).into())
    };

    ($span:expr, $fmt:expr, $($arg:expr),+ $(,)?) => {
        bail!($span, format!($fmt, $($arg),+))
    };
}

/// The result type for typesetting and all its subpasses.
pub type TypResult<T> = Result<T, TypError>;

/// The error type for typesetting and all its subpasses.
pub type TypError = Box<Vec<Error>>;

/// A result type with a string error message.
pub type StrResult<T> = Result<T, String>;

/// An error in a source file.
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct Error {
    /// The erroneous location in the source code.
    pub span: Span,
    /// A diagnostic message describing the problem.
    pub message: String,
    /// The trace of function calls leading to the error.
    pub trace: Vec<Spanned<Tracepoint>>,
}

impl Error {
    /// Create a new, bare error.
    pub fn new(span: Span, message: impl Into<String>) -> Self {
        Self {
            span,
            trace: vec![],
            message: message.into(),
        }
    }

    /// Create a boxed vector containing one error. The return value is suitable
    /// as the `Err` variant of a [`TypResult`].
    pub fn boxed(span: Span, message: impl Into<String>) -> Box<Vec<Self>> {
        Box::new(vec![Self::new(span, message)])
    }
}

/// A part of an error's [trace](Error::trace).
#[derive(Debug, Clone, Eq, PartialEq, Ord, PartialOrd)]
pub enum Tracepoint {
    /// A function call.
    Call(Option<String>),
    /// A module import.
    Import,
}

impl Display for Tracepoint {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        match self {
            Tracepoint::Call(Some(name)) => {
                write!(f, "error occured in this call of function `{}`", name)
            }
            Tracepoint::Call(None) => f.pad("error occured in this function call"),
            Tracepoint::Import => f.pad("error occured while importing this module"),
        }
    }
}

/// Convert a [`StrResult`] to a [`TypResult`] by adding span information.
pub trait At<T> {
    /// Add the span information.
    fn at(self, span: Span) -> TypResult<T>;
}

impl<T, S> At<T> for Result<T, S>
where
    S: Into<String>,
{
    fn at(self, span: Span) -> TypResult<T> {
        self.map_err(|message| Error::boxed(span, message))
    }
}

/// Enrich a [`TypResult`] with a tracepoint.
pub trait Trace<T> {
    /// Add the tracepoint to all errors that lie outside the `span`.
    fn trace<F>(self, make_point: F, span: Span) -> Self
    where
        F: Fn() -> Tracepoint;
}

impl<T> Trace<T> for TypResult<T> {
    fn trace<F>(self, make_point: F, span: Span) -> Self
    where
        F: Fn() -> Tracepoint,
    {
        self.map_err(|mut errors| {
            for error in errors.iter_mut() {
                if !span.surrounds(error.span) {
                    error.trace.push(Spanned::new(make_point(), span));
                }
            }
            errors
        })
    }
}

/// Transform `expected X, found Y` into `expected X or A, found Y`.
pub fn with_alternative(msg: String, alt: &str) -> String {
    let mut parts = msg.split(", found ");
    if let (Some(a), Some(b)) = (parts.next(), parts.next()) {
        format!("{} or {}, found {}", a, alt, b)
    } else {
        msg
    }
}
