//! Diagnostics.

use std::fmt::{self, Display, Formatter};
use std::io;
use std::ops::Range;
use std::path::{Path, PathBuf};
use std::str::Utf8Error;
use std::string::FromUtf8Error;

use comemo::Tracked;

use crate::syntax::{ErrorPos, Span, Spanned};
use crate::World;

/// Early-return with a [`StrResult`] or [`SourceResult`].
///
/// If called with just a string and format args, returns with a
/// `StrResult`. If called with a span, a string and format args, returns
/// a `SourceResult`.
///
/// ```
/// bail!("bailing with a {}", "string result");
/// bail!(span, "bailing with a {}", "source result");
/// ```
#[macro_export]
#[doc(hidden)]
macro_rules! __bail {
    ($fmt:literal $(, $arg:expr)* $(,)?) => {
        return Err($crate::diag::eco_format!($fmt, $($arg),*))
    };

    ($error:expr) => {
        return Err(Box::new(vec![$error]))
    };

    ($span:expr, $fmt:literal $(, $arg:expr)* $(,)?) => {
        return Err(Box::new(vec![$crate::diag::SourceError::new(
            $span,
            $crate::diag::eco_format!($fmt, $($arg),*),
        )]))
    };
}

#[doc(inline)]
pub use crate::__bail as bail;

/// Construct an [`EcoString`] or [`SourceError`].
#[macro_export]
#[doc(hidden)]
macro_rules! __error {
    ($fmt:literal $(, $arg:expr)* $(,)?) => {
        $crate::diag::eco_format!($fmt, $($arg),*)
    };

    ($span:expr, $fmt:literal $(, $arg:expr)* $(,)?) => {
        $crate::diag::SourceError::new(
            $span,
            $crate::diag::eco_format!($fmt, $($arg),*),
        )
    };
}

#[doc(inline)]
pub use crate::__error as error;
#[doc(hidden)]
pub use ecow::{eco_format, EcoString};

/// A result that can carry multiple source errors.
pub type SourceResult<T> = Result<T, Box<Vec<SourceError>>>;

/// An error in a source file.
///
/// The contained spans will only be detached if any of the input source files
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
    /// Additonal hints to the user, indicating how this error could be avoided
    /// or worked around.
    pub hints: Vec<EcoString>,
}

impl SourceError {
    /// Create a new, bare error.
    pub fn new(span: Span, message: impl Into<EcoString>) -> Self {
        Self {
            span,
            pos: ErrorPos::Full,
            trace: vec![],
            message: message.into(),
            hints: vec![],
        }
    }

    /// Adjust the position in the node where the error should be annotated.
    pub fn with_pos(mut self, pos: ErrorPos) -> Self {
        self.pos = pos;
        self
    }

    /// Adds user-facing hints to the error.
    pub fn with_hints(mut self, hints: impl IntoIterator<Item = EcoString>) -> Self {
        self.hints.extend(hints);
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
                write!(f, "error occurred in this call of function `{}`", name)
            }
            Tracepoint::Call(None) => {
                write!(f, "error occurred in this function call")
            }
            Tracepoint::Show(name) => {
                write!(f, "error occurred while applying show rule to this {name}")
            }
            Tracepoint::Import => {
                write!(f, "error occurred while importing this module")
            }
        }
    }
}

/// Enrich a [`SourceResult`] with a tracepoint.
pub trait Trace<T> {
    /// Add the tracepoint to all errors that lie outside the `span`.
    fn trace<F>(self, world: Tracked<dyn World + '_>, make_point: F, span: Span) -> Self
    where
        F: Fn() -> Tracepoint;
}

impl<T> Trace<T> for SourceResult<T> {
    fn trace<F>(self, world: Tracked<dyn World + '_>, make_point: F, span: Span) -> Self
    where
        F: Fn() -> Tracepoint,
    {
        self.map_err(|mut errors| {
            if span.is_detached() {
                return errors;
            }
            let range = world.source(span.source()).range(span);
            for error in errors.iter_mut().filter(|e| !e.span.is_detached()) {
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
        self.map_err(|message| Box::new(vec![SourceError::new(span, message)]))
    }
}

/// A result type with a string error message and hints.
pub type HintedStrResult<T> = Result<T, HintedString>;

/// A string message with hints.
#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub struct HintedString {
    /// A diagnostic message describing the problem.
    pub message: EcoString,
    /// Additonal hints to the user, indicating how this error could be avoided
    /// or worked around.
    pub hints: Vec<EcoString>,
}

impl<T> At<T> for Result<T, HintedString> {
    fn at(self, span: Span) -> SourceResult<T> {
        self.map_err(|diags| {
            Box::new(vec![SourceError::new(span, diags.message).with_hints(diags.hints)])
        })
    }
}

/// Enrich a [`StrResult`] or [`HintedStrResult`] with a hint.
pub trait Hint<T> {
    /// Add the hint.
    fn hint(self, hint: impl Into<EcoString>) -> HintedStrResult<T>;
}

impl<T> Hint<T> for StrResult<T> {
    fn hint(self, hint: impl Into<EcoString>) -> HintedStrResult<T> {
        self.map_err(|message| HintedString { message, hints: vec![hint.into()] })
    }
}

impl<T> Hint<T> for HintedStrResult<T> {
    fn hint(self, hint: impl Into<EcoString>) -> HintedStrResult<T> {
        self.map_err(|mut error| {
            error.hints.push(hint.into());
            error
        })
    }
}

/// A result type with a file-related error.
pub type FileResult<T> = Result<T, FileError>;

/// An error that occurred while trying to load of a file.
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
        eco_format!("{error}")
    }
}

/// Format a user-facing error message for an XML-like file format.
pub fn format_xml_like_error(format: &str, error: roxmltree::Error) -> EcoString {
    match error {
        roxmltree::Error::UnexpectedCloseTag { expected, actual, pos } => {
            eco_format!(
                "failed to parse {format}: found closing tag '{actual}' \
                 instead of '{expected}' in line {}",
                pos.row
            )
        }
        roxmltree::Error::UnknownEntityReference(entity, pos) => {
            eco_format!(
                "failed to parse {format}: unknown entity '{entity}' in line {}",
                pos.row
            )
        }
        roxmltree::Error::DuplicatedAttribute(attr, pos) => {
            eco_format!(
                "failed to parse {format}: duplicate attribute '{attr}' in line {}",
                pos.row
            )
        }
        roxmltree::Error::NoRootNode => {
            eco_format!("failed to parse {format}: missing root node")
        }
        _ => eco_format!("failed to parse {format}"),
    }
}
