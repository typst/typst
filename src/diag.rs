//! Diagnostics for source code.
//!
//! There are no fatal errors. The document will always compile and yield a
//! layout on a best effort process, but diagnostics are nevertheless generated
//! for incorrect things.

use std::fmt::{self, Display, Formatter};

/// A diagnostic that arose in parsing or layouting.
#[derive(Debug, Clone, Eq, PartialEq, Ord, PartialOrd)]
#[cfg_attr(feature = "serialize", derive(serde::Serialize))]
pub struct Diag {
    /// How severe / important the diagnostic is.
    pub level: Level,
    /// A message describing the diagnostic.
    pub message: String,
}

/// How severe / important a diagnostic is.
#[derive(Debug, Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
#[cfg_attr(feature = "serialize", derive(serde::Serialize))]
#[cfg_attr(feature = "serialize", serde(rename_all = "camelCase"))]
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

/// Construct a diagnostic with [`Error`] level.
///
/// ```
/// # use typstc::error;
/// # use typstc::syntax::Span;
/// # let span = Span::ZERO;
/// # let name = "";
/// // Create formatted error values.
/// let error = error!("expected {}", name);
///
/// // Create spanned errors.
/// let spanned = error!(span, "there is an error here");
/// ```
///
/// [`Error`]: diagnostic/enum.Level.html#variant.Error
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
/// [`Warning`]: diagnostic/enum.Level.html#variant.Warning
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
