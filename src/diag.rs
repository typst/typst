//! Diagnostics.

use std::fmt::{self, Display, Formatter};
use std::io;
use std::ops::Range;
use std::path::{Path, PathBuf};
use std::str::Utf8Error;
use std::string::FromUtf8Error;

use comemo::Tracked;

use crate::syntax::{ErrorPos, Span, Spanned};
use crate::util::{format_eco, EcoString};
use crate::World;

/// Early-return with a [`SourceError`].
#[macro_export]
#[doc(hidden)]
macro_rules! __bail {
    ($error:expr) => {
        return Err(Box::new(vec![$error]))
    };

    ($($tts:tt)*) => {
        $crate::diag::bail!($crate::diag::error!($($tts)*))
    };
}

#[doc(inline)]
pub use crate::__bail as bail;

/// Construct a [`SourceError`].
#[macro_export]
#[doc(hidden)]
macro_rules! __error {
    ($span:expr, $message:expr $(,)?) => {
        $crate::diag::SourceError::new($span, $message)
    };

    ($span:expr, $fmt:expr, $($arg:expr),+ $(,)?) => {
        $crate::diag::error!($span, $crate::util::format_eco!($fmt, $($arg),+))
    };
}

#[doc(inline)]
pub use crate::__error as error;

/// A result that can carry multiple source errors.
pub type SourceResult<T> = Result<T, Box<Vec<SourceError>>>;

/// An error in a source file.
///
/// This contained spans will only be detached if any of the input source files
/// were detached.
#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub struct SourceError {
    /// The span of the erroneous node in the source code.
    pub span: Span,
    /// The position in the node where the error should be annotated.
    pub pos: ErrorPos,
    /// A diagnostic message describing the problem.
    pub message: EcoString,
    /// The trace of function calls leading to the error.
    pub trace: Vec<Spanned<Tracepoint>>,
}

impl SourceError {
    /// Create a new, bare error.
    #[track_caller]
    pub fn new(span: Span, message: impl Into<EcoString>) -> Self {
        assert!(!span.is_detached());
        Self {
            span,
            pos: ErrorPos::Full,
            trace: vec![],
            message: message.into(),
        }
    }

    /// Adjust the position in the node where the error should be annotated.
    pub fn with_pos(mut self, pos: ErrorPos) -> Self {
        self.pos = pos;
        self
    }

    /// The range in the source file identified by
    /// [`self.span.source()`](Span::source) where the error should be
    /// annotated.
    pub fn range(&self, world: &dyn World) -> Range<usize> {
        let full = world.source(self.span.source()).range(self.span);
        match self.pos {
            ErrorPos::Full => full,
            ErrorPos::Start => full.start..full.start,
            ErrorPos::End => full.end..full.end,
        }
    }
}

/// A part of an error's [trace](SourceError::trace).
#[derive(Debug, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub enum Tracepoint {
    /// A function call.
    Call(Option<EcoString>),
    /// A show rule application.
    Show(EcoString),
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
            Tracepoint::Show(name) => {
                write!(f, "error occured while applying show rule to this {name}")
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
pub type StrResult<T> = Result<T, EcoString>;

/// Convert a [`StrResult`] to a [`SourceResult`] by adding span information.
pub trait At<T> {
    /// Add the span information.
    fn at(self, span: Span) -> SourceResult<T>;
}

impl<T, S> At<T> for Result<T, S>
where
    S: Into<EcoString>,
{
    fn at(self, span: Span) -> SourceResult<T> {
        self.map_err(|message| Box::new(vec![error!(span, message)]))
    }
}

/// Format the parts separated with commas and a final "and" or "or".
pub(crate) fn comma_list<S>(buf: &mut String, parts: &[S], last: &str)
where
    S: AsRef<str>,
{
    for (i, part) in parts.iter().enumerate() {
        match i {
            0 => {}
            1 if parts.len() == 2 => {
                buf.push(' ');
                buf.push_str(last);
                buf.push(' ');
            }
            i if i + 1 == parts.len() => {
                buf.push_str(", ");
                buf.push_str(last);
                buf.push(' ');
            }
            _ => buf.push_str(", "),
        }
        buf.push_str(part.as_ref());
    }
}

/// A result type with a file-related error.
pub type FileResult<T> = Result<T, FileError>;

/// An error that occured while trying to load of a file.
#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub enum FileError {
    /// A file was not found at this path.
    NotFound(PathBuf),
    /// A file could not be accessed.
    AccessDenied,
    /// A directory was found, but a file was expected.
    IsDirectory,
    /// The file is not a Typst source file, but should have been.
    NotSource,
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
            Self::AccessDenied => f.pad("failed to load file (access denied)"),
            Self::IsDirectory => f.pad("failed to load file (is a directory)"),
            Self::NotSource => f.pad("not a typst source file"),
            Self::InvalidUtf8 => f.pad("file is not valid utf-8"),
            Self::Other => f.pad("failed to load file"),
        }
    }
}

impl From<Utf8Error> for FileError {
    fn from(_: Utf8Error) -> Self {
        Self::InvalidUtf8
    }
}

impl From<FromUtf8Error> for FileError {
    fn from(_: FromUtf8Error) -> Self {
        Self::InvalidUtf8
    }
}

impl From<FileError> for EcoString {
    fn from(error: FileError) -> Self {
        format_eco!("{error}")
    }
}

/// Format a user-facing error message for an XML-like file format.
pub fn format_xml_like_error(format: &str, error: roxmltree::Error) -> String {
    match error {
        roxmltree::Error::UnexpectedCloseTag { expected, actual, pos } => {
            format!(
                "failed to parse {format}: found closing tag '{actual}' \
                 instead of '{expected}' in line {}",
                pos.row
            )
        }
        roxmltree::Error::UnknownEntityReference(entity, pos) => {
            format!(
                "failed to parse {format}: unknown entity '{entity}' in line {}",
                pos.row
            )
        }
        roxmltree::Error::DuplicatedAttribute(attr, pos) => {
            format!(
                "failed to parse {format}: duplicate attribute '{attr}' in line {}",
                pos.row
            )
        }
        roxmltree::Error::NoRootNode => {
            format!("failed to parse {format}: missing root node")
        }
        roxmltree::Error::SizeLimit => "file is too large".into(),
        _ => format!("failed to parse {format}"),
    }
}
