//! Foundational functions.

mod calc;
mod create;
mod data;
mod numbering;

pub use self::calc::*;
pub use self::create::*;
pub use self::data::*;
pub use self::numbering::*;

use comemo::Track;
use typst::model::{self, Route, Vm};
use typst::syntax::Source;

use crate::prelude::*;

/// The name of a value's type.
pub fn type_(_: &Vm, args: &mut Args) -> SourceResult<Value> {
    Ok(args.expect::<Value>("value")?.type_name().into())
}

/// The string representation of a value.
pub fn repr(_: &Vm, args: &mut Args) -> SourceResult<Value> {
    Ok(args.expect::<Value>("value")?.repr().into())
}

/// Ensure that a condition is fulfilled.
pub fn assert(_: &Vm, args: &mut Args) -> SourceResult<Value> {
    let Spanned { v, span } = args.expect::<Spanned<bool>>("condition")?;
    if !v {
        bail!(span, "assertion failed");
    }
    Ok(Value::None)
}

/// Evaluate a string as Typst markup.
pub fn eval(vm: &Vm, args: &mut Args) -> SourceResult<Value> {
    let Spanned { v: text, span } = args.expect::<Spanned<String>>("source")?;
    let source = Source::synthesized(text, span);
    let route = Route::default();
    let module = model::eval(vm.world(), route.track(), &source)?;
    Ok(Value::Content(module.content))
}
