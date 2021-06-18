use std::cmp::Ordering;

use super::*;

/// `min`: The minimum of two values.
///
/// # Positional parameters
/// - Values: variadic, must be comparable.
///
/// # Return value
/// The minimum of the sequence of values. For equal elements, the first one is
/// returned.
pub fn min(ctx: &mut EvalContext, args: &mut FuncArgs) -> Value {
    minmax(ctx, args, Ordering::Less)
}

/// `max`: The maximum of two values.
///
/// # Positional parameters
/// - Values: variadic, must be comparable.
///
/// # Return value
/// The maximum of the sequence of values. For equal elements, the first one is
/// returned.
pub fn max(ctx: &mut EvalContext, args: &mut FuncArgs) -> Value {
    minmax(ctx, args, Ordering::Greater)
}

/// Find the minimum or maximum of a sequence of values.
fn minmax(ctx: &mut EvalContext, args: &mut FuncArgs, goal: Ordering) -> Value {
    let mut extremum = None;

    while let Some(value) = args.eat::<Value>(ctx) {
        if let Some(prev) = &extremum {
            match value.cmp(&prev) {
                Some(ordering) if ordering == goal => extremum = Some(value),
                Some(_) => {}
                None => {
                    ctx.diag(error!(
                        args.span,
                        "cannot compare {} with {}",
                        prev.type_name(),
                        value.type_name(),
                    ));
                    return Value::Error;
                }
            }
        } else {
            extremum = Some(value);
        }
    }

    extremum.unwrap_or_else(|| {
        args.expect::<Value>(ctx, "value");
        Value::Error
    })
}
