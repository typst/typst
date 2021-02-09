use crate::prelude::*;

/// `type`: Find out the name of a value's type.
///
/// # Positional arguments
/// - Any value.
///
/// # Return value
/// The name of the value's type as a string.
pub fn type_(ctx: &mut EvalContext, args: &mut ValueArgs) -> Value {
    if let Some(value) = args.require::<Value>(ctx, "value") {
        value.type_name().into()
    } else {
        Value::Error
    }
}
