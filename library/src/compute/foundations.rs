use crate::prelude::*;

use comemo::Track;
use typst::model;
use typst::syntax::Source;

/// The name of a value's type.
///
/// # Parameters
/// - value: Value (positional, required)
///   The value whose type's to determine.
///
/// # Tags
/// - foundations
#[func]
pub fn type_(args: &mut Args) -> SourceResult<Value> {
    Ok(args.expect::<Value>("value")?.type_name().into())
}

/// The string representation of a value.
///
/// # Parameters
/// - value: Value (positional, required)
///   The value whose string representation to produce.
///
/// # Tags
/// - foundations
#[func]
pub fn repr(args: &mut Args) -> SourceResult<Value> {
    Ok(args.expect::<Value>("value")?.repr().into())
}

/// Ensure that a condition is fulfilled.
///
/// # Parameters
/// - condition: bool (positional, required)
///   The condition that must be true for the assertion to pass.
///
/// # Tags
/// - foundations
#[func]
pub fn assert(args: &mut Args) -> SourceResult<Value> {
    let Spanned { v, span } = args.expect::<Spanned<bool>>("condition")?;
    if !v {
        bail!(span, "assertion failed");
    }
    Ok(Value::None)
}

/// Evaluate a string as Typst markup.
///
/// # Parameters
/// - source: String (positional, required)
///   A string of Typst markup to evaluate.
///
/// # Tags
/// - foundations
#[func]
pub fn eval(vm: &Vm, args: &mut Args) -> SourceResult<Value> {
    let Spanned { v: text, span } = args.expect::<Spanned<String>>("source")?;
    let source = Source::synthesized(text, span);
    let route = model::Route::default();
    let module = model::eval(vm.world(), route.track(), &source)?;
    Ok(Value::Content(module.content))
}
