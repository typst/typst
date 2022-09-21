//! Diagnostics.

use std::fmt::{self, Display, Formatter};
use std::io;
use std::path::{Path, PathBuf};
use std::string::FromUtf8Error;

use comemo::Tracked;

use crate::syntax::{Span, Spanned};
use crate::World;

/// Early-return with a [`SourceError`].
#[macro_export]
macro_rules! bail {
    ($error:expr) => {
        return Err(Box::new(vec![$error]))
    };

    ($($tts:tt)*) => {
        $crate::bail!($crate::error!($($tts)*))
    };
}

/// Construct a [`SourceError`].
#[macro_export]
macro_rules! error {
    ($span:expr, $message:expr $(,)?) => {
        $crate::diag::SourceError::new($span, $message)
    };

    ($span:expr, $fmt:expr, $($arg:expr),+ $(,)?) => {
        $crate::error!($span, format!($fmt, $($arg),+))
    };
}

/// A result that can carry multiple source errors.
pub type SourceResult<T> = Result<T, Box<Vec<SourceError>>>;

/// An error in a source file.
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct SourceError {
    /// The erroneous node in the source code.
    pub span: Span,
    /// A diagnostic message describing the problem.
    pub message: String,
    /// The trace of function calls leading to the error.
    pub trace: Vec<Spanned<Tracepoint>>,
}

impl SourceError {
    /// Create a new, bare error.
    pub fn new(span: Span, message: impl Into<String>) -> Self {
        Self {
            span,
            trace: vec![],
            message: message.into(),
        }
    }
}

/// A part of an error's [trace](SourceError::trace).
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

/// Enrich a [`SourceResult`] with a tracepoint.
pub trait Trace<T> {
    /// Add the tracepoint to all errors that lie outside the `span`.
    fn trace<F>(self, world: Tracked<dyn World>, make_point: F, span: Span) -> Self
    where
        F: Fn() -> Tracepoint;
}

impl<T> Trace<T> for SourceResult<T> {
    fn trace<F>(self, world: Tracked<dyn World>, make_point: F, span: Span) -> Self
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

/// A result type with a string error message.
pub type StrResult<T> = Result<T, String>;

/// Transform `expected X, found Y` into `expected X or A, found Y`.
pub fn with_alternative(msg: String, alt: &str) -> String {
    let mut parts = msg.split(", found ");
    if let (Some(a), Some(b)) = (parts.next(), parts.next()) {
        format!("{} or {}, found {}", a, alt, b)
    } else {
        msg
    }
}

/// Convert a [`StrResult`] to a [`SourceResult`] by adding span information.
pub trait At<T> {
    /// Add the span information.
    fn at(self, span: Span) -> SourceResult<T>;
}

impl<T, S> At<T> for Result<T, S>
where
    S: Into<String>,
{
    fn at(self, span: Span) -> SourceResult<T> {
        self.map_err(|message| Box::new(vec![error!(span, message)]))
    }
}

/// A result type with a file-related error.
pub type FileResult<T> = Result<T, FileError>;

/// An error that occured while trying to load of a file.
#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub enum FileError {
    /// A file was not found at this path.
    NotFound(PathBuf),
    /// A directory was found, but a file was expected.
    IsDirectory,
    /// A file could not be accessed.
    AccessDenied,
    /// The file was not valid UTF-8, but should have been.
    InvalidUtf8,
    /// Another error.
    Other,
}

impl FileError {
    /// Create a file error from an I/O error.
    pub fn from_io(error: io::Error, path: &Path) -> Self {
        match error.kind() {
            io::ErrorKind::NotFound => Self::NotFound(path.into()),
            io::ErrorKind::PermissionDenied => Self::AccessDenied,
            io::ErrorKind::InvalidData
                if error.to_string().contains("stream did not contain valid UTF-8") =>
            {
                Self::InvalidUtf8
            }
            _ => Self::Other,
        }
    }
}

impl std::error::Error for FileError {}

impl Display for FileError {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        match self {
            Self::NotFound(path) => {
                write!(f, "file not found (searched at {})", path.display())
            }
            Self::IsDirectory => f.pad("failed to load file (is a directory)"),
            Self::AccessDenied => f.pad("failed to load file (access denied)"),
            Self::InvalidUtf8 => f.pad("file is not valid utf-8"),
            Self::Other => f.pad("failed to load file"),
        }
    }
}

impl From<FromUtf8Error> for FileError {
    fn from(_: FromUtf8Error) -> Self {
        Self::InvalidUtf8
    }
}

impl From<FileError> for String {
    fn from(error: FileError) -> Self {
        error.to_string()
    }
}
