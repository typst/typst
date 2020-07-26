//! Problems (errors / warnings) in _Typst_ documents.
//!
//! There are no fatal errors in _Typst_. The document will always compile and
//! yield a layout. However, this is a best effort process and bad things will
//! still generate errors and warnings.

use serde::Serialize;
use crate::syntax::span::SpanVec;


/// A list of spanned problems.
pub type Problems = SpanVec<Problem>;

/// A problem that arose in parsing or layouting.
#[derive(Debug, Clone, Eq, PartialEq, Serialize)]
pub struct Problem {
    /// How severe / important the problem is.
    pub severity: Severity,
    /// A message describing the problem.
    pub message: String,
}

/// How severe / important a problem is.
#[derive(Debug, Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum Severity {
    /// Something in the code is not good.
    Warning,
    /// Something in the code is wrong!
    Error,
}

impl Problem {
    /// Create a new problem from message and severity.
    pub fn new(message: impl Into<String>, severity: Severity) -> Self {
        Self { message: message.into(), severity }
    }
}

/// Construct a problem with `Error` severity.
///
/// ```
/// # use typstc::error;
/// # use typstc::syntax::span::Span;
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
        $crate::__impl_problem!($crate::problem::Severity::Error; $($tts)*)
    };
}

/// Construct a problem with `Warning` severity.
///
/// This works exactly like `error!`. See its documentation for more
/// information.
#[macro_export]
macro_rules! warning {
    ($($tts:tt)*) => {
        $crate::__impl_problem!($crate::problem::Severity::Warning; $($tts)*)
    };
}

/// Backs the `error!` and `warning!` macros.
#[macro_export]
#[doc(hidden)]
macro_rules! __impl_problem {
    ($severity:expr; @$feedback:expr, $($tts:tt)*) => {
        $feedback.problems.push($crate::__impl_problem!($severity; $($tts)*));
    };

    ($severity:expr; $fmt:literal $($tts:tt)*) => {
        $crate::problem::Problem::new(format!($fmt $($tts)*), $severity)
    };

    ($severity:expr; $span:expr, $fmt:literal $($tts:tt)*) => {
        $crate::syntax::span::Spanned::new(
            $crate::__impl_problem!($severity; $fmt $($tts)*),
            $span,
        )
    };
}
