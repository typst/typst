use crate::prelude::*;

use comemo::Track;
use typst::model;
use typst::syntax::Source;

/// # Type
/// Determine a value's type.
///
/// Returns the name of the value's type.
///
/// ## Example
/// ```
/// #type(12) \
/// #type(14.7) \
/// #type("hello") \
/// #type(none) \
/// #type([Hi]) \
/// #type(x => x + 1)
/// ```
///
/// ## Parameters
/// - value: Value (positional, required)
///   The value whose type's to determine.
///
/// - returns: string
///
/// ## Category
/// foundations
#[func]
pub fn type_(args: &mut Args) -> SourceResult<Value> {
    Ok(args.expect::<Value>("value")?.type_name().into())
}

/// # Representation
/// The string representation of a value.
///
/// When inserted into content, most values are displayed as this representation
/// in monospace with syntax-highlighting. The exceptions are `{none}`,
/// integers, floats, strings, content, and functions.
///
/// ## Example
/// ```
/// { none } vs #repr(none) \
/// { "hello" } vs #repr("hello") \
/// { (1, 2) } vs #repr((1, 2)) \
/// { [*Hi*] } vs #repr([*Hi*])
/// ```
///
/// ## Parameters
/// - value: Value (positional, required)
///   The value whose string representation to produce.
///
/// - returns: string
///
/// ## Category
/// foundations
#[func]
pub fn repr(args: &mut Args) -> SourceResult<Value> {
    Ok(args.expect::<Value>("value")?.repr().into())
}

/// # Assert
/// Ensure that a condition is fulfilled.
///
/// Fails with an error if the condition is not fulfilled. Does not
/// produce any output in the document.
///
/// ## Example
/// ```
/// #assert(1 < 2)
/// ```
///
/// ## Parameters
/// - condition: bool (positional, required)
///   The condition that must be true for the assertion to pass.
///
/// ## Category
/// foundations
#[func]
pub fn assert(args: &mut Args) -> SourceResult<Value> {
    let Spanned { v, span } = args.expect::<Spanned<bool>>("condition")?;
    if !v {
        bail!(span, "assertion failed");
    }
    Ok(Value::None)
}

/// # Evaluate
/// Evaluate a string as Typst markup.
///
/// You shouldn't typically need this function, but it is there if you do.
///
/// ## Example
/// ```
/// #let markup = "= Heading\n _Emphasis_"
/// #eval(markup)
/// ```
///
/// ## Parameters
/// - source: String (positional, required)
///   A string of Typst markup to evaluate.
///
///   The markup and code in the string cannot interact with the file system.
///
/// - returns: content
///
/// ## Category
/// foundations
#[func]
pub fn eval(vm: &Vm, args: &mut Args) -> SourceResult<Value> {
    let Spanned { v: text, span } = args.expect::<Spanned<String>>("source")?;
    let source = Source::synthesized(text, span);
    let route = model::Route::default();
    let module = model::eval(vm.world(), route.track(), &source)?;
    Ok(Value::Content(module.content()))
}
