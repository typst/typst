use crate::prelude::*;
use crate::pretty::pretty;

/// `type`: Find out the name of a value's type.
///
/// # Positional arguments
/// - Any value.
///
/// # Return value
/// The name of the value's type as a string.
pub fn type_(ctx: &mut EvalContext, args: &mut ValueArgs) -> Value {
    match args.require::<Value>(ctx, "value") {
        Some(value) => value.type_name().into(),
        None => Value::Error,
    }
}

/// `repr`: Get the string representation of a value.
///
/// # Positional arguments
/// - Any value.
///
/// # Return value
/// The string representation of the value.
pub fn repr(ctx: &mut EvalContext, args: &mut ValueArgs) -> Value {
    match args.require::<Value>(ctx, "value") {
        Some(value) => pretty(&value).into(),
        None => Value::Error,
    }
}
