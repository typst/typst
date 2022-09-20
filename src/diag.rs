//! Diagnostics.

use std::fmt::{self, Display, Formatter};
use std::io;
use std::path::Path;

use crate::syntax::{Span, Spanned};
use crate::World;

/// Early-return with a [`TypError`].
#[macro_export]
macro_rules! bail {
    ($($tts:tt)*) => {
        return Err($crate::error!($($tts)*).into())
    };
}

/// Construct a [`TypError`].
#[macro_export]
macro_rules! error {
    ($span:expr, $message:expr $(,)?) => {
        Box::new(vec![$crate::diag::Error::new($span, $message)])
    };

    ($span:expr, $fmt:expr, $($arg:expr),+ $(,)?) => {
        $crate::error!($span, format!($fmt, $($arg),+))
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
    /// The erroneous node in the source code.
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
            Tracepoint::Call(None) => {
                write!(f, "error occured in this function call")
            }
            Tracepoint::Import => {
                write!(f, "error occured while importing this module")
            }
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
        self.map_err(|message| error!(span, message))
    }
}

/// Enrich a [`TypResult`] with a tracepoint.
pub trait Trace<T> {
    /// Add the tracepoint to all errors that lie outside the `span`.
    fn trace<F>(self, world: &dyn World, make_point: F, span: Span) -> Self
    where
        F: Fn() -> Tracepoint;
}

impl<T> Trace<T> for TypResult<T> {
    fn trace<F>(self, world: &dyn World, make_point: F, span: Span) -> Self
    where
        F: Fn() -> Tracepoint,
    {
        self.map_err(|mut errors| {
            let range = world.source(span.source()).range(span);
            for error in errors.iter_mut() {
                // Skip traces that surround the error.
                let error_range = world.source(error.span.source()).range(error.span);
                if range.start <= error_range.start && range.end >= error_range.end {
                    continue;
                }

                error.trace.push(Spanned::new(make_point(), span));
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

/// Format a file loading failure.
pub fn failed_to_load(target: &str, path: &Path, error: io::Error) -> String {
    match error.kind() {
        io::ErrorKind::NotFound => {
            format!("file not found (searched at {})", path.display())
        }
        _ => format!("failed to load {target} ({error})"),
    }
}
