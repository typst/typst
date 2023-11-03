//! Diagnostics.

use std::fmt::{self, Display, Formatter};
use std::io;
use std::path::{Path, PathBuf};
use std::str::Utf8Error;
use std::string::FromUtf8Error;

use comemo::Tracked;

use crate::syntax::{PackageSpec, Span, Spanned, SyntaxError};
use crate::{World, WorldExt};

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
        return Err(Box::new(vec![$crate::diag::SourceDiagnostic::error(
            $span,
            $crate::diag::eco_format!($fmt, $($arg),*),
        )]))
    };
}

#[doc(inline)]
pub use crate::{__bail as bail, __error as error, __warning as warning};

#[doc(hidden)]
pub use ecow::{eco_format, EcoString};

/// Construct an [`EcoString`] or [`SourceDiagnostic`] with severity `Error`.
#[macro_export]
#[doc(hidden)]
macro_rules! __error {
    ($fmt:literal $(, $arg:expr)* $(,)?) => {
        $crate::diag::eco_format!($fmt, $($arg),*)
    };

    ($span:expr, $fmt:literal $(, $arg:expr)* $(,)?) => {
        $crate::diag::SourceDiagnostic::error(
            $span,
            $crate::diag::eco_format!($fmt, $($arg),*),
        )
    };
}

/// Construct a [`SourceDiagnostic`] with severity `Warning`.
#[macro_export]
#[doc(hidden)]
macro_rules! __warning {
    ($span:expr, $fmt:literal $(, $arg:expr)* $(,)?) => {
        $crate::diag::SourceDiagnostic::warning(
            $span,
            $crate::diag::eco_format!($fmt, $($arg),*),
        )
    };
}

/// A result that can carry multiple source errors.
pub type SourceResult<T> = Result<T, Box<Vec<SourceDiagnostic>>>;

/// An error or warning in a source file.
///
/// The contained spans will only be detached if any of the input source files
/// were detached.
#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub struct SourceDiagnostic {
    /// Whether the diagnostic is an error or a warning.
    pub severity: Severity,
    /// The span of the relevant node in the source code.
    pub span: Span,
    /// A diagnostic message describing the problem.
    pub message: EcoString,
    /// The trace of function calls leading to the problem.
    pub trace: Vec<Spanned<Tracepoint>>,
    /// Additonal hints to the user, indicating how this problem could be avoided
    /// or worked around.
    pub hints: Vec<EcoString>,
}

/// The severity of a [`SourceDiagnostic`].
#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash)]
pub enum Severity {
    /// A fatal error.
    Error,
    /// A non-fatal warning.
    Warning,
}

impl SourceDiagnostic {
    /// Create a new, bare error.
    pub fn error(span: Span, message: impl Into<EcoString>) -> Self {
        Self {
            severity: Severity::Error,
            span,
            trace: vec![],
            message: message.into(),
            hints: vec![],
        }
    }

    /// Create a new, bare warning.
    pub fn warning(span: Span, message: impl Into<EcoString>) -> Self {
        Self {
            severity: Severity::Warning,
            span,
            trace: vec![],
            message: message.into(),
            hints: vec![],
        }
    }

    /// Adds a single hint to the diagnostic.
    pub fn hint(&mut self, hint: impl Into<EcoString>) {
        self.hints.push(hint.into());
    }

    /// Adds a single hint to the diagnostic.
    pub fn with_hint(mut self, hint: impl Into<EcoString>) -> Self {
        self.hint(hint);
        self
    }

    /// Adds user-facing hints to the diagnostic.
    pub fn with_hints(mut self, hints: impl IntoIterator<Item = EcoString>) -> Self {
        self.hints.extend(hints);
        self
    }
}

impl From<SyntaxError> for SourceDiagnostic {
    fn from(error: SyntaxError) -> Self {
        Self {
            severity: Severity::Error,
            span: error.span,
            message: error.message,
            trace: vec![],
            hints: error.hints,
        }
    }
}

/// A part of a diagnostic's [trace](SourceDiagnostic::trace).
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
            let Some(trace_range) = world.range(span) else { return errors };
            for error in errors.iter_mut() {
                // Skip traces that surround the error.
                if let Some(error_range) = world.range(error.span) {
                    if error.span.id() == span.id()
                        && trace_range.start <= error_range.start
                        && trace_range.end >= error_range.end
                    {
                        continue;
                    }
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
        self.map_err(|message| {
            let mut diagnostic = SourceDiagnostic::error(span, message);
            if diagnostic.message.contains("(access denied)") {
                diagnostic.hint("cannot read file outside of project root");
                diagnostic
                    .hint("you can adjust the project root with the --root argument");
            }
            Box::new(vec![diagnostic])
        })
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
            Box::new(vec![
                SourceDiagnostic::error(span, diags.message).with_hints(diags.hints)
            ])
        })
    }
}

/// Enrich a [`StrResult`] or [`HintedStrResult`] with a hint.
pub trait Hint<T> {
    /// Add the hint.
    fn hint(self, hint: impl Into<EcoString>) -> HintedStrResult<T>;
}

impl<T, S> Hint<T> for Result<T, S>
where
    S: Into<EcoString>,
{
    fn hint(self, hint: impl Into<EcoString>) -> HintedStrResult<T> {
        self.map_err(|message| HintedString {
            message: message.into(),
            hints: vec![hint.into()],
        })
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
    /// The package the file is part of could not be loaded.
    Package(PackageError),
    /// Another error.
    ///
    /// The optional string can give more details, if available.
    Other(Option<EcoString>),
}

impl FileError {
    /// Create a file error from an I/O error.
    pub fn from_io(err: io::Error, path: &Path) -> Self {
        match err.kind() {
            io::ErrorKind::NotFound => Self::NotFound(path.into()),
            io::ErrorKind::PermissionDenied => Self::AccessDenied,
            io::ErrorKind::InvalidData
                if err.to_string().contains("stream did not contain valid UTF-8") =>
            {
                Self::InvalidUtf8
            }
            _ => Self::Other(Some(eco_format!("{err}"))),
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
            Self::Package(error) => error.fmt(f),
            Self::Other(Some(err)) => write!(f, "failed to load file ({err})"),
            Self::Other(None) => f.pad("failed to load file"),
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

impl From<PackageError> for FileError {
    fn from(err: PackageError) -> Self {
        Self::Package(err)
    }
}

impl From<FileError> for EcoString {
    fn from(err: FileError) -> Self {
        eco_format!("{err}")
    }
}

/// A result type with a package-related error.
pub type PackageResult<T> = Result<T, PackageError>;

/// An error that occured while trying to load a package.
///
/// Some variants have an optional string can give more details, if available.
#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub enum PackageError {
    /// The specified package does not exist.
    NotFound(PackageSpec),
    /// Failed to retrieve the package through the network.
    NetworkFailed(Option<EcoString>),
    /// The package archive was malformed.
    MalformedArchive(Option<EcoString>),
    /// Another error.
    Other(Option<EcoString>),
}

impl std::error::Error for PackageError {}

impl Display for PackageError {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        match self {
            Self::NotFound(spec) => {
                write!(f, "package not found (searched for {spec})",)
            }
            Self::NetworkFailed(Some(err)) => {
                write!(f, "failed to download package ({err})")
            }
            Self::NetworkFailed(None) => f.pad("failed to download package"),
            Self::MalformedArchive(Some(err)) => {
                write!(f, "failed to decompress package ({err})")
            }
            Self::MalformedArchive(None) => {
                f.pad("failed to decompress package (archive malformed)")
            }
            Self::Other(Some(err)) => write!(f, "failed to load package ({err})"),
            Self::Other(None) => f.pad("failed to load package"),
        }
    }
}

impl From<PackageError> for EcoString {
    fn from(err: PackageError) -> Self {
        eco_format!("{err}")
    }
}

/// Format a user-facing error message for an XML-like file format.
pub fn format_xml_like_error(format: &str, error: roxmltree::Error) -> EcoString {
    match error {
        roxmltree::Error::UnexpectedCloseTag { expected, actual, pos } => {
            eco_format!(
                "failed to parse {format} (found closing tag '{actual}' \
                 instead of '{expected}' in line {})",
                pos.row
            )
        }
        roxmltree::Error::UnknownEntityReference(entity, pos) => {
            eco_format!(
                "failed to parse {format} (unknown entity '{entity}' in line {})",
                pos.row
            )
        }
        roxmltree::Error::DuplicatedAttribute(attr, pos) => {
            eco_format!(
                "failed to parse {format}: (duplicate attribute '{attr}' in line {})",
                pos.row
            )
        }
        roxmltree::Error::NoRootNode => {
            eco_format!("failed to parse {format} (missing root node)")
        }
        err => eco_format!("failed to parse {format} ({err})"),
    }
}
