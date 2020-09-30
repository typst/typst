//! Diagnostics for source code.
//!
//! There are no fatal errors. The document will always compile and yield a
//! layout on a best effort process, generating diagnostics for incorrect
//! things.

#[cfg(feature = "serialize")]
use serde::Serialize;

use crate::syntax::SpanVec;

/// A list of spanned diagnostics.
pub type Diagnostics = SpanVec<Diagnostic>;

/// A diagnostic that arose in parsing or layouting.
#[derive(Debug, Clone, Eq, PartialEq, Ord, PartialOrd)]
#[cfg_attr(feature = "serialize", derive(Serialize))]
pub struct Diagnostic {
    /// How severe / important the diagnostic is.
    pub level: Level,
    /// A message describing the diagnostic.
    pub message: String,
}

/// How severe / important a diagnostic is.
#[derive(Debug, Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
#[cfg_attr(feature = "serialize", derive(Serialize))]
#[cfg_attr(feature = "serialize", serde(rename_all = "camelCase"))]
pub enum Level {
    Warning,
    Error,
}

impl Diagnostic {
    /// Create a new diagnostic from message and level.
    pub fn new(message: impl Into<String>, level: Level) -> Self {
        Self { message: message.into(), level }
    }
}

/// Construct a diagnostic with `Error` level.
///
/// ```
/// # use typstc::error;
/// # use typstc::syntax::Span;
/// # use typstc::Feedback;
/// # let span = Span::ZERO;
/// # let mut feedback = Feedback::new();
/// # let name = "";
/// // Create formatted error values.
/// let error = error!("expected {}", name);
///
/// // Create spanned errors.
/// let spanned = error!(span, "there is an error here");
///
/// // Create an error and directly add it to existing feedback.
/// error!(@feedback, span, "oh no!");
/// ```
#[macro_export]
macro_rules! error {
    ($($tts:tt)*) => {
        $crate::__impl_diagnostic!($crate::diagnostic::Level::Error; $($tts)*)
    };
}

/// Construct a diagnostic with `Warning` level.
///
/// This works exactly like `error!`. See its documentation for more
/// information.
#[macro_export]
macro_rules! warning {
    ($($tts:tt)*) => {
        $crate::__impl_diagnostic!($crate::diagnostic::Level::Warning; $($tts)*)
    };
}

/// Backs the `error!` and `warning!` macros.
#[macro_export]
#[doc(hidden)]
macro_rules! __impl_diagnostic {
    ($level:expr; @$feedback:expr, $($tts:tt)*) => {
        $feedback.diagnostics.push($crate::__impl_diagnostic!($level; $($tts)*));
    };

    ($level:expr; $fmt:literal $($tts:tt)*) => {
        $crate::diagnostic::Diagnostic::new(format!($fmt $($tts)*), $level)
    };

    ($level:expr; $span:expr, $fmt:literal $($tts:tt)*) => {
        $crate::syntax::Spanned::new(
            $crate::__impl_diagnostic!($level; $fmt $($tts)*),
            $span,
        )
    };
}
