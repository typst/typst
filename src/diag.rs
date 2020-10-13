//! Diagnostics and decorations for source code.
//!
//! There are no fatal errors. The document will always compile and yield a
//! layout on a best effort process, but diagnostics are nevertheless generated
//! for incorrect things.

use crate::syntax::SpanVec;
use std::fmt::{self, Display, Formatter};

/// The result of some pass: Some output `T` and [feedback] data.
///
/// [feedback]: struct.Feedback.html
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct Pass<T> {
    /// The output of this compilation pass.
    pub output: T,
    /// User feedback data accumulated in this pass.
    pub feedback: Feedback,
}

impl<T> Pass<T> {
    /// Create a new pass from output and feedback data.
    pub fn new(output: T, feedback: Feedback) -> Self {
        Self { output, feedback }
    }

    /// Create a new pass with empty feedback.
    pub fn okay(output: T) -> Self {
        Self { output, feedback: Feedback::new() }
    }

    /// Map the output type and keep the feedback data.
    pub fn map<U>(self, f: impl FnOnce(T) -> U) -> Pass<U> {
        Pass {
            output: f(self.output),
            feedback: self.feedback,
        }
    }
}

/// Diagnostic and semantic syntax highlighting data.
#[derive(Debug, Default, Clone, Eq, PartialEq)]
pub struct Feedback {
    /// Diagnostics about the source code.
    pub diags: SpanVec<Diag>,
    /// Decorations of the source code for semantic syntax highlighting.
    pub decos: SpanVec<Deco>,
}

impl Feedback {
    /// Create a new feedback instance without errors and decos.
    pub fn new() -> Self {
        Self { diags: vec![], decos: vec![] }
    }

    /// Merge two feedbacks into one.
    pub fn join(mut a: Self, b: Self) -> Self {
        a.extend(b);
        a
    }

    /// Add other feedback data to this feedback.
    pub fn extend(&mut self, more: Self) {
        self.diags.extend(more.diags);
        self.decos.extend(more.decos);
    }
}

/// A diagnostic that arose in parsing or layouting.
#[derive(Debug, Clone, Eq, PartialEq, Ord, PartialOrd)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub struct Diag {
    /// How severe / important the diagnostic is.
    pub level: Level,
    /// A message describing the diagnostic.
    pub message: String,
}

/// How severe / important a diagnostic is.
#[derive(Debug, Copy, Clone, Eq, PartialEq, Ord, PartialOrd)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
pub enum Level {
    Warning,
    Error,
}

impl Diag {
    /// Create a new diagnostic from message and level.
    pub fn new(level: Level, message: impl Into<String>) -> Self {
        Self { level, message: message.into() }
    }
}

impl Display for Level {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        f.pad(match self {
            Self::Warning => "warning",
            Self::Error => "error",
        })
    }
}

/// Decorations for semantic syntax highlighting.
#[derive(Debug, Copy, Clone, Eq, PartialEq, Ord, PartialOrd)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
pub enum Deco {
    /// Emphasized text.
    Emph,
    /// Strong text.
    Strong,
    /// A valid, successfully resolved name.
    Resolved,
    /// An invalid, unresolved name.
    Unresolved,
    /// A key in a dictionary.
    DictKey,
}

/// Construct a diagnostic with [`Error`] level.
///
/// ```
/// # use typst::error;
/// # use typst::syntax::Span;
/// # let span = Span::ZERO;
/// # let name = "";
/// // Create formatted error values.
/// let error = error!("expected {}", name);
///
/// // Create spanned errors.
/// let spanned = error!(span, "there is an error here");
/// ```
///
/// [`Error`]: diag/enum.Level.html#variant.Error
#[macro_export]
macro_rules! error {
    ($($tts:tt)*) => {
        $crate::__impl_diagnostic!($crate::diag::Level::Error; $($tts)*)
    };
}

/// Construct a diagnostic with [`Warning`] level.
///
/// This works exactly like `error!`. See its documentation for more
/// information.
///
/// [`Warning`]: diag/enum.Level.html#variant.Warning
#[macro_export]
macro_rules! warning {
    ($($tts:tt)*) => {
        $crate::__impl_diagnostic!($crate::diag::Level::Warning; $($tts)*)
    };
}

/// Backs the `error!` and `warning!` macros.
#[macro_export]
#[doc(hidden)]
macro_rules! __impl_diagnostic {
    ($level:expr; $fmt:literal $($tts:tt)*) => {
        $crate::diag::Diag::new($level, format!($fmt $($tts)*))
    };

    ($level:expr; $span:expr, $fmt:literal $($tts:tt)*) => {
        $crate::syntax::Spanned::new(
            $crate::__impl_diagnostic!($level; $fmt $($tts)*),
            $span,
        )
    };
}
