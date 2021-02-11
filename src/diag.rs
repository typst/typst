//! Diagnostics for source code.
//!
//! Errors are never fatal, the document will always compile and yield a layout.

use std::collections::BTreeSet;
use std::fmt::{self, Display, Formatter};

use crate::syntax::Span;

/// The result of some pass: Some output `T` and diagnostics.
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct Pass<T> {
    /// The output of this compilation pass.
    pub output: T,
    /// User diagnostics accumulated in this pass.
    pub diags: DiagSet,
}

impl<T> Pass<T> {
    /// Create a new pass from output and diagnostics.
    pub fn new(output: T, diags: DiagSet) -> Self {
        Self { output, diags }
    }
}

/// A set of diagnostics.
///
/// Since this is a [`BTreeSet`], there cannot be two equal (up to span)
/// diagnostics and you can quickly iterate diagnostics in source location
/// order.
pub type DiagSet = BTreeSet<Diag>;

/// A diagnostic with severity level and message.
#[derive(Debug, Clone, Eq, PartialEq, Ord, PartialOrd)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub struct Diag {
    /// The source code location.
    pub span: Span,
    /// How severe / important the diagnostic is.
    pub level: Level,
    /// A message describing the diagnostic.
    pub message: String,
}

impl Diag {
    /// Create a new diagnostic from message and level.
    pub fn new(span: impl Into<Span>, level: Level, message: impl Into<String>) -> Self {
        Self {
            span: span.into(),
            level,
            message: message.into(),
        }
    }
}

impl Display for Diag {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        write!(f, "{}: {}", self.level, self.message)
    }
}

/// How severe / important a diagnostic is.
#[derive(Debug, Copy, Clone, Eq, PartialEq, Ord, PartialOrd)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
pub enum Level {
    Warning,
    Error,
}

impl Display for Level {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        f.pad(match self {
            Self::Warning => "warning",
            Self::Error => "error",
        })
    }
}

/// Construct a diagnostic with [`Error`](Level::Error) level.
///
/// ```
/// # use typst::error;
/// # use typst::syntax::Span;
/// # let span = Span::ZERO;
/// # let name = "";
/// let error = error!(span, "there is an error with {}", name);
/// ```
#[macro_export]
macro_rules! error {
    ($span:expr, $($tts:tt)*) => {
        $crate::diag::Diag::new(
            $span,
            $crate::diag::Level::Error,
            format!($($tts)*),
        )
    };
}

/// Construct a diagnostic with [`Warning`](Level::Warning) level.
///
/// This works exactly like `error!`. See its documentation for more
/// information.
#[macro_export]
macro_rules! warning {
    ($span:expr, $($tts:tt)*) => {
        $crate::diag::Diag::new(
            $span,
            $crate::diag::Level::Warning,
            format!($($tts)*),
        )
    };
}
