use tracing_error::SpanTrace;

use crate::prelude::*;

/// Prints a formatted backtrace of the current call stack.
///
/// This is a debugging function that can be used to print the current
/// internal call stack. It is useful for debugging, but should not be
/// used in production code.
///
/// Display: Backtrace
/// Category: debugging
/// Returns: Value
#[func]
pub fn backtrace() -> Value {
    let span_trace = SpanTrace::capture();
    println!("{}", color_spantrace::colorize(&span_trace));

    Value::None
}
