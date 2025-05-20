//! Diagnostics.

use std::fmt::{self, Display, Formatter};
use std::io;
use std::path::{Path, PathBuf};
use std::str::Utf8Error;
use std::string::FromUtf8Error;

use comemo::Tracked;
use ecow::{eco_vec, EcoVec};
use typst_syntax::package::{PackageSpec, PackageVersion};
use typst_syntax::{Lines, Span, Spanned, SyntaxError};
use utf8_iter::ErrorReportingUtf8Chars;

use crate::engine::Engine;
use crate::loading::{LoadSource, Loaded};
use crate::{World, WorldExt};

/// Early-return with a [`StrResult`] or [`SourceResult`].
///
/// If called with just a string and format args, returns with a
/// `StrResult`. If called with a span, a string and format args, returns
/// a `SourceResult`.
///
/// You can also emit hints with the `; hint: "..."` syntax.
///
/// ```ignore
/// bail!("bailing with a {}", "string result");
/// bail!(span, "bailing with a {}", "source result");
/// bail!(
///     span, "bailing with a {}", "source result";
///     hint: "hint 1"
/// );
/// bail!(
///     span, "bailing with a {}", "source result";
///     hint: "hint 1";
///     hint: "hint 2";
/// );
/// ```
#[macro_export]
#[doc(hidden)]
macro_rules! __bail {
    // For bail!("just a {}", "string")
    (
        $fmt:literal $(, $arg:expr)*
        $(; hint: $hint:literal $(, $hint_arg:expr)*)*
        $(,)?
    ) => {
        return Err($crate::diag::error!(
            $fmt $(, $arg)*
            $(; hint: $hint $(, $hint_arg)*)*
        ))
    };

    // For bail!(error!(..))
    ($error:expr) => {
        return Err(::ecow::eco_vec![$error])
    };

    // For bail(span, ...)
    ($($tts:tt)*) => {
        return Err(::ecow::eco_vec![$crate::diag::error!($($tts)*)])
    };
}

/// Construct an [`EcoString`], [`HintedString`] or [`SourceDiagnostic`] with
/// severity `Error`.
#[macro_export]
#[doc(hidden)]
macro_rules! __error {
    // For bail!("just a {}", "string").
    ($fmt:literal $(, $arg:expr)* $(,)?) => {
        $crate::diag::eco_format!($fmt, $($arg),*).into()
    };

    // For bail!("a hinted {}", "string"; hint: "some hint"; hint: "...")
    (
        $fmt:literal $(, $arg:expr)*
        $(; hint: $hint:literal $(, $hint_arg:expr)*)*
        $(,)?
    ) => {
        $crate::diag::HintedString::new(
            $crate::diag::eco_format!($fmt, $($arg),*)
        ) $(.with_hint($crate::diag::eco_format!($hint, $($hint_arg),*)))*
    };

    // For bail!(span, ...)
    (
        $span:expr, $fmt:literal $(, $arg:expr)*
        $(; hint: $hint:literal $(, $hint_arg:expr)*)*
        $(,)?
    ) => {
        $crate::diag::SourceDiagnostic::error(
            $span,
            $crate::diag::eco_format!($fmt, $($arg),*),
        )  $(.with_hint($crate::diag::eco_format!($hint, $($hint_arg),*)))*
    };
}

/// Construct a [`SourceDiagnostic`] with severity `Warning`.
///
/// You can also emit hints with the `; hint: "..."` syntax.
///
/// ```ignore
/// warning!(span, "warning with a {}", "source result");
/// warning!(
///     span, "warning with a {}", "source result";
///     hint: "hint 1"
/// );
/// warning!(
///     span, "warning with a {}", "source result";
///     hint: "hint 1";
///     hint: "hint 2";
/// );
/// ```
#[macro_export]
#[doc(hidden)]
macro_rules! __warning {
    (
        $span:expr,
        $fmt:literal $(, $arg:expr)*
        $(; hint: $hint:literal $(, $hint_arg:expr)*)*
        $(,)?
    ) => {
        $crate::diag::SourceDiagnostic::warning(
            $span,
            $crate::diag::eco_format!($fmt, $($arg),*),
        ) $(.with_hint($crate::diag::eco_format!($hint, $($hint_arg),*)))*
    };
}

#[rustfmt::skip]
#[doc(inline)]
pub use {
    crate::__bail as bail,
    crate::__error as error,
    crate::__warning as warning,
    ecow::{eco_format, EcoString},
};

/// A result that can carry multiple source errors.
pub type SourceResult<T> = Result<T, EcoVec<SourceDiagnostic>>;

/// An output alongside warnings generated while producing it.
#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub struct Warned<T> {
    /// The produced output.
    pub output: T,
    /// Warnings generated while producing the output.
    pub warnings: EcoVec<SourceDiagnostic>,
}

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
    pub trace: EcoVec<Spanned<Tracepoint>>,
    /// Additional hints to the user, indicating how this problem could be avoided
    /// or worked around.
    pub hints: EcoVec<EcoString>,
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
            trace: eco_vec![],
            message: message.into(),
            hints: eco_vec![],
        }
    }

    /// Create a new, bare warning.
    pub fn warning(span: Span, message: impl Into<EcoString>) -> Self {
        Self {
            severity: Severity::Warning,
            span,
            trace: eco_vec![],
            message: message.into(),
            hints: eco_vec![],
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
            trace: eco_vec![],
            hints: error.hints,
        }
    }
}

/// Destination for a deprecation message when accessing a deprecated value.
pub trait DeprecationSink {
    /// Emits the given deprecation message into this sink.
    fn emit(self, message: &str);
}

impl DeprecationSink for () {
    fn emit(self, _: &str) {}
}

impl DeprecationSink for (&mut Engine<'_>, Span) {
    /// Emits the deprecation message as a warning.
    fn emit(self, message: &str) {
        self.0.sink.warn(SourceDiagnostic::warning(self.1, message));
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
                write!(f, "error occurred in this call of function `{name}`")
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
            for error in errors.make_mut().iter_mut() {
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

/// Convert a [`StrResult`] or [`HintedStrResult`] to a [`SourceResult`] by
/// adding span information.
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
            eco_vec![diagnostic]
        })
    }
}

/// A result type with a string error message and hints.
pub type HintedStrResult<T> = Result<T, HintedString>;

/// A string message with hints.
///
/// This is internally represented by a vector of strings.
/// The first element of the vector contains the message.
/// The remaining elements are the hints.
/// This is done to reduce the size of a HintedString.
/// The vector is guaranteed to not be empty.
#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub struct HintedString(EcoVec<EcoString>);

impl HintedString {
    /// Creates a new hinted string with the given message.
    pub fn new(message: EcoString) -> Self {
        Self(eco_vec![message])
    }

    /// A diagnostic message describing the problem.
    pub fn message(&self) -> &EcoString {
        self.0.first().unwrap()
    }

    /// Additional hints to the user, indicating how this error could be avoided
    /// or worked around.
    pub fn hints(&self) -> &[EcoString] {
        self.0.get(1..).unwrap_or(&[])
    }

    /// Adds a single hint to the hinted string.
    pub fn hint(&mut self, hint: impl Into<EcoString>) {
        self.0.push(hint.into());
    }

    /// Adds a single hint to the hinted string.
    pub fn with_hint(mut self, hint: impl Into<EcoString>) -> Self {
        self.hint(hint);
        self
    }

    /// Adds user-facing hints to the hinted string.
    pub fn with_hints(mut self, hints: impl IntoIterator<Item = EcoString>) -> Self {
        self.0.extend(hints);
        self
    }
}

impl<S> From<S> for HintedString
where
    S: Into<EcoString>,
{
    fn from(value: S) -> Self {
        Self::new(value.into())
    }
}

impl<T> At<T> for HintedStrResult<T> {
    fn at(self, span: Span) -> SourceResult<T> {
        self.map_err(|err| {
            let mut components = err.0.into_iter();
            let message = components.next().unwrap();
            let diag = SourceDiagnostic::error(span, message).with_hints(components);
            eco_vec![diag]
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
        self.map_err(|message| HintedString::new(message.into()).with_hint(hint))
    }
}

impl<T> Hint<T> for HintedStrResult<T> {
    fn hint(self, hint: impl Into<EcoString>) -> HintedStrResult<T> {
        self.map_err(|mut error| {
            error.hint(hint.into());
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

/// An error that occurred while trying to load a package.
///
/// Some variants have an optional string can give more details, if available.
#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub enum PackageError {
    /// The specified package does not exist.
    NotFound(PackageSpec),
    /// The specified package found, but the version does not exist.
    VersionNotFound(PackageSpec, PackageVersion),
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
            Self::VersionNotFound(spec, latest) => {
                write!(
                    f,
                    "package found, but version {} does not exist (latest is {})",
                    spec.version, latest,
                )
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

pub type LoadResult<T> = Result<T, LoadError>;

/// A callsite independent error that occurred during data loading.
/// Can be turned into a [`SourceDiagnostic`] using the [`LoadedAt::in_text`]
/// or [`LoadedAt::in_invalid_text`] methods available on [`LoadResult`].
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct LoadError {
    pub pos: ReportPos,
    pub message: EcoString,
    pub error: EcoString,
}

impl LoadError {
    pub fn new(
        pos: impl Into<ReportPos>,
        message: impl std::fmt::Display,
        error: impl std::fmt::Display,
    ) -> Self {
        Self {
            pos: pos.into(),
            message: eco_format!("{message}"),
            error: eco_format!("{error}"),
        }
    }
}

/// Convert a [`LoadResult`] to a [`SourceResult`] by adding the [`Loaded`] context.
pub trait LoadedAt<T> {
    /// Add the span information.
    fn in_text(self, data: &Loaded) -> SourceResult<T>;

    /// Add the span information.
    fn in_invalid_text(self, data: &Loaded) -> SourceResult<T>;
}

impl<T> LoadedAt<T> for Result<T, LoadError> {
    /// Report an error, possibly in an external file.
    fn in_text(self, data: &Loaded) -> SourceResult<T> {
        self.map_err(|err| data.err_in_text(err.pos, err.message, err.error))
    }

    /// Report an error in invalid text.
    fn in_invalid_text(self, data: &Loaded) -> SourceResult<T> {
        self.map_err(|err| data.err_in_invalid_text(err.pos, err.message, err.error))
    }
}

impl Loaded {
    pub fn err_in_text(
        &self,
        pos: impl Into<ReportPos>,
        msg: impl std::fmt::Display,
        error: impl std::fmt::Display,
    ) -> EcoVec<SourceDiagnostic> {
        let pos = pos.into();
        let lines = Lines::from_bytes(&self.bytes);
        match (self.source.v, lines) {
            // Only report an error in an external file,
            // if it is human readable (valid utf-8).
            (LoadSource::Path(file_id), Ok(lines)) => {
                if let Some(range) = pos.range(&lines) {
                    let span = Span::from_range(file_id, range);
                    return eco_vec!(error!(span, "{msg} ({error})"));
                }

                // Either `ReportPos::None` was provided, or resolving the range
                // from the line/column failed. If present report the possibly
                // wrong line/column in the error message anyway.
                let span = Span::from_range(file_id, 0..self.bytes.len());
                let error = if let Some(pair) = pos.line_col(&lines) {
                    let (line, col) = pair.numbers();
                    error!(span, "{msg} ({error} at {line}:{col})")
                } else {
                    error!(span, "{msg} ({error})")
                };
                eco_vec![error]
            }
            (_, Ok(lines)) => {
                let error = if let Some(pair) = pos.line_col(&lines) {
                    let (line, col) = pair.numbers();
                    error!(self.source.span, "{msg} ({error} at {line}:{col})")
                } else {
                    error!(self.source.span, "{msg} ({error})")
                };
                eco_vec![error]
            }
            _ => self.err_in_invalid_text(pos, msg, error),
        }
    }

    /// Report an error, possibly in an external file.
    pub fn err_in_invalid_text(
        &self,
        pos: impl Into<ReportPos>,
        msg: impl std::fmt::Display,
        error: impl std::fmt::Display,
    ) -> EcoVec<SourceDiagnostic> {
        let pos = pos.into();
        let error = if let Some(pair) = pos.try_line_col(&self.bytes) {
            let (line, col) = pair.numbers();
            error!(self.source.span, "{msg} ({error} at {line}:{col})")
        } else {
            error!(self.source.span, "{msg} ({error})")
        };
        eco_vec![error]
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Hash)]
pub enum ReportPos {
    /// Contains the range, and the 0-based line/column.
    Full(std::ops::Range<usize>, LineCol),
    /// Contains the range.
    Range(std::ops::Range<usize>),
    /// Contains the 0-based line/column.
    LineCol(LineCol),
    #[default]
    None,
}

impl From<std::ops::Range<usize>> for ReportPos {
    fn from(value: std::ops::Range<usize>) -> Self {
        Self::Range(value)
    }
}

impl From<LineCol> for ReportPos {
    fn from(value: LineCol) -> Self {
        Self::LineCol(value)
    }
}

impl ReportPos {
    fn range(&self, lines: &Lines<String>) -> Option<std::ops::Range<usize>> {
        match self {
            ReportPos::Full(range, _) => Some(range.clone()),
            ReportPos::Range(range) => Some(range.clone()),
            &ReportPos::LineCol(pair) => {
                let i = lines.line_column_to_byte(pair.line, pair.col)?;
                Some(i..i)
            }
            ReportPos::None => None,
        }
    }

    fn line_col(&self, lines: &Lines<String>) -> Option<LineCol> {
        match self {
            &ReportPos::Full(_, pair) => Some(pair),
            ReportPos::Range(range) => {
                let (line, col) = lines.byte_to_line_column(range.start)?;
                Some(LineCol::zero_based(line, col))
            }
            &ReportPos::LineCol(pair) => Some(pair),
            ReportPos::None => None,
        }
    }

    /// Either get the the line/column pair, or try to compute it from possibly
    /// invalid utf-8 data.
    fn try_line_col(&self, bytes: &[u8]) -> Option<LineCol> {
        match self {
            &ReportPos::Full(_, pair) => Some(pair),
            ReportPos::Range(range) => LineCol::try_from_byte_pos(range.start, bytes),
            &ReportPos::LineCol(pair) => Some(pair),
            ReportPos::None => None,
        }
    }
}

/// A line/column pair.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct LineCol {
    /// The 0-based line.
    line: usize,
    /// The 0-based column.
    col: usize,
}

impl LineCol {
    /// Constructs the line/column pair from 0-based indices.
    pub fn zero_based(line: usize, col: usize) -> Self {
        Self { line, col }
    }

    /// Constructs the line/column pair from 1-based numbers.
    pub fn one_based(line: usize, col: usize) -> Self {
        Self {
            line: line.saturating_sub(1),
            col: col.saturating_sub(1),
        }
    }

    /// Try to compute a line/column pair from possibly invalid utf-8 data.
    pub fn try_from_byte_pos(pos: usize, bytes: &[u8]) -> Option<Self> {
        let bytes = &bytes[..pos];
        let mut line = 0;
        let line_start = memchr::memchr_iter(b'\n', bytes)
            .inspect(|_| line += 1)
            .last()
            .map(|i| i + 1)
            .unwrap_or(bytes.len());

        let col = ErrorReportingUtf8Chars::new(&bytes[line_start..]).count();
        Some(LineCol::zero_based(line, col))
    }

    /// Returns the 0-based line/column indices.
    pub fn indices(&self) -> (usize, usize) {
        (self.line as usize, self.col as usize)
    }

    /// Returns the 1-based line/column numbers.
    pub fn numbers(&self) -> (usize, usize) {
        (self.line as usize + 1, self.col as usize + 1)
    }
}

/// Format a user-facing error message for an XML-like file format.
pub fn format_xml_like_error(format: &str, error: roxmltree::Error) -> LoadError {
    let pos = LineCol::one_based(error.pos().row as usize, error.pos().col as usize);
    let msg = eco_format!("failed to parse {format}");
    let err = match error {
        roxmltree::Error::UnexpectedCloseTag(expected, actual, _) => {
            eco_format!("found closing tag '{actual}' instead of '{expected}'")
        }
        roxmltree::Error::UnknownEntityReference(entity, _) => {
            eco_format!("unknown entity '{entity}'")
        }
        roxmltree::Error::DuplicatedAttribute(attr, _) => {
            eco_format!("duplicate attribute '{attr}'")
        }
        roxmltree::Error::NoRootNode => {
            eco_format!("missing root node")
        }
        err => eco_format!("{err}"),
    };

    LoadError::new(pos, msg, err)
}
