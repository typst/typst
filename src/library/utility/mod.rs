//! Computational utility functions.

mod blind;
mod color;
mod math;
mod string;

pub use blind::*;
pub use color::*;
pub use math::*;
pub use string::*;

use crate::library::prelude::*;

/// The name of a value's type.
pub fn type_(_: &mut Context, args: &mut Args) -> TypResult<Value> {
    Ok(args.expect::<Value>("value")?.type_name().into())
}

/// Ensure that a condition is fulfilled.
pub fn assert(_: &mut Context, args: &mut Args) -> TypResult<Value> {
    let Spanned { v, span } = args.expect::<Spanned<bool>>("condition")?;
    if !v {
        bail!(span, "assertion failed");
    }
    Ok(Value::None)
}
