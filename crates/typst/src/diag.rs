//! Diagnostics.

use std::collections::HashSet;
use std::fmt::{self, Display, Formatter};
use std::io;
use std::path::{Path, PathBuf};
use std::str::Utf8Error;
use std::string::FromUtf8Error;

use comemo::Tracked;
use ecow::{eco_vec, EcoVec};

use crate::syntax::package::{PackageSpec, PackageVersion};
use crate::syntax::{ast, Span, Spanned, SyntaxError};
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
        $id:ident,
        $fmt:literal $(, $arg:expr)*
        $(; hint: $hint:literal $(, $hint_arg:expr)*)*
        $(,)?
    ) => {
        $crate::diag::SourceDiagnostic::warning(
            $span,
            ::std::option::Option::Some($crate::diag::CompilerWarning::$id),
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
    /// The identifier for this diagnostic.
    pub identifier: Option<Identifier>,
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
            identifier: None,
            trace: eco_vec![],
            message: message.into(),
            hints: eco_vec![],
        }
    }

    /// Create a new, bare warning.
    pub fn warning(
        span: Span,
        identifier: Option<CompilerWarning>,
        message: impl Into<EcoString>,
    ) -> Self {
        Self {
            severity: Severity::Warning,
            span,
            identifier: identifier.map(Identifier::Warn),
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
            identifier: None,
            span: error.span,
            message: error.message,
            trace: eco_vec![],
            hints: error.hints,
        }
    }
}

/// The identifier of a [`SourceDiagnostic`].
#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub enum Identifier {
    /// Identifier for a built-in compiler warning.
    Warn(CompilerWarning),
    /// Identifier for a warning raised by a package.
    User(EcoString),
}

/// Built-in compiler warnings.
#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub enum CompilerWarning {
    UnnecessaryImportRenaming,
    UnnecessaryStars,
    UnnecessaryUnderscores,
    NonConvergingLayout,
    UnknownFontFamilies,
}

impl CompilerWarning {
    /// The name of the warning as a string, following the format of diagnostic
    /// identifiers.
    pub const fn name(&self) -> &'static str {
        match self {
            CompilerWarning::UnnecessaryImportRenaming => "unnecessary-import-renaming",
            CompilerWarning::UnnecessaryStars => "unnecessary-stars",
            CompilerWarning::UnnecessaryUnderscores => "unnecessary-underscores",
            CompilerWarning::NonConvergingLayout => "non-converging-layout",
            CompilerWarning::UnknownFontFamilies => "unknown-font-families",
        }
    }
}

impl Identifier {
    /// The identifier's name, e.g. 'unnecessary-stars'.
    pub fn name(&self) -> &str {
        match self {
            Self::Warn(warning_identifier) => warning_identifier.name(),
            Self::User(user_identifier) => user_identifier,
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

/// Enrich diagnostics with a tracepoint.
pub trait Trace {
    /// Add the tracepoint to all diagnostics that lie outside the `span`.
    fn trace<F>(self, world: Tracked<dyn World + '_>, make_point: F, span: Span) -> Self
    where
        F: Fn() -> Tracepoint;
}

impl Trace for EcoVec<SourceDiagnostic> {
    fn trace<F>(
        mut self,
        world: Tracked<dyn World + '_>,
        make_point: F,
        span: Span,
    ) -> Self
    where
        F: Fn() -> Tracepoint,
    {
        let Some(trace_range) = world.range(span) else { return self };
        for error in self.make_mut().iter_mut() {
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
        self
    }
}

impl<T> Trace for SourceResult<T> {
    fn trace<F>(self, world: Tracked<dyn World + '_>, make_point: F, span: Span) -> Self
    where
        F: Fn() -> Tracepoint,
    {
        self.map_err(|errors| errors.trace(world, make_point, span))
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

/// Deduplicate errors based on their spans and messages.
pub fn deduplicate_errors(
    mut errors: EcoVec<SourceDiagnostic>,
) -> EcoVec<SourceDiagnostic> {
    let mut unique = HashSet::new();
    errors.retain(|error| {
        debug_assert!(error.severity == Severity::Error);
        let hash = crate::utils::hash128(&(&error.span, &error.message));
        unique.insert(hash)
    });
    errors
}

/// Apply warning suppression, deduplication, and return the remaining
/// warnings.
///
/// We deduplicate warnings which are identical modulo tracepoints.
/// This is so we can attempt to suppress each tracepoint separately,
/// without having one suppression discard all other attempts of raising
/// this same warning through a different set of tracepoints.
///
/// For example, calling the same function twice but suppressing a warning
/// on the first call shouldn't suppress on the second, so each set of
/// tracepoints for a particular warning matters. If at least one instance
/// of a warning isn't suppressed, the warning will be returned, and the
/// remaining duplicates are discarded.
pub fn deduplicate_and_suppress_warnings(
    warnings: &mut EcoVec<SourceDiagnostic>,
    world: &dyn World,
) {
    let mut unsuppressed = HashSet::<u128>::default();

    // Only retain warnings which weren't locally suppressed where they
    // were emitted or at any of their tracepoints.
    warnings.retain(|diag| {
        debug_assert!(diag.severity == Severity::Warning);
        let hash = crate::utils::hash128(&(&diag.span, &diag.identifier, &diag.message));
        if unsuppressed.contains(&hash) {
            // This warning - with the same span, identifier and message -
            // was already raised and not suppressed before, with a
            // different set of tracepoints. Therefore, we should not raise
            // it again, and checking for suppression is unnecessary.
            return false;
        }

        let Some(identifier) = &diag.identifier else {
            // Can't suppress without an identifier. Therefore, retain the
            // warning. It is not a duplicate due to the check above.
            unsuppressed.insert(hash);
            return true;
        };

        let should_raise = !is_warning_suppressed(diag.span, world, identifier)
            && !diag.trace.iter().any(|tracepoint| {
                is_warning_suppressed(tracepoint.span, world, identifier)
            });

        // If this warning wasn't suppressed, any further duplicates (with
        // different tracepoints) should be removed.
        should_raise && unsuppressed.insert(hash)
    });
}

/// Checks if a given warning is suppressed given one span it has a tracepoint
/// in. If one of the ancestors of the node where the warning occurred has a
/// warning suppression decorator sibling right before it suppressing this
/// particular warning, the warning is considered suppressed.
fn is_warning_suppressed(span: Span, world: &dyn World, identifier: &Identifier) -> bool {
    // Don't suppress detached warnings.
    let Some(source) = span.id().and_then(|file| world.source(file).ok()) else {
        return false;
    };

    let search_root = source.find(span);
    let mut searched_node = search_root.as_ref();

    // Walk the parent nodes to check for a warning suppression in the
    // previous line.
    while let Some(node) = searched_node {
        let mut searched_decorator = node.prev_attached_decorator();
        while let Some(sibling) = searched_decorator {
            let decorator = sibling.cast::<ast::Decorator>().unwrap();
            if check_decorator_suppresses_warning(decorator, identifier) {
                return true;
            }
            searched_decorator = sibling.prev_attached_decorator();
        }
        searched_node = node.parent();
    }

    false
}

/// Checks if an 'allow' decorator would cause a warning with a particular
/// identifier to be suppressed.
fn check_decorator_suppresses_warning(
    decorator: ast::Decorator,
    warning: &Identifier,
) -> bool {
    if decorator.name().as_str() != "allow" {
        return false;
    }

    for argument in decorator.arguments() {
        if warning.name() == argument.get() {
            return true;
        }
    }

    false
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

/// Format a user-facing error message for an XML-like file format.
pub fn format_xml_like_error(format: &str, error: roxmltree::Error) -> EcoString {
    match error {
        roxmltree::Error::UnexpectedCloseTag(expected, actual, pos) => {
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
                "failed to parse {format} (duplicate attribute '{attr}' in line {})",
                pos.row
            )
        }
        roxmltree::Error::NoRootNode => {
            eco_format!("failed to parse {format} (missing root node)")
        }
        err => eco_format!("failed to parse {format} ({err})"),
    }
}
