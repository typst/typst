use crate::prelude::*;

/// # Type
/// Determine a value's type.
///
/// Returns the name of the value's type.
///
/// ## Example
/// ```example
/// #type(12) \
/// #type(14.7) \
/// #type("hello") \
/// #type(none) \
/// #type([Hi]) \
/// #type(x => x + 1)
/// ```
///
/// ## Parameters
/// - value: `Value` (positional, required)
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
/// ```example
/// #none vs #repr(none) \
/// #"hello" vs #repr("hello") \
/// #(1, 2) vs #repr((1, 2)) \
/// #[*Hi*] vs #repr([*Hi*])
/// ```
///
/// ## Parameters
/// - value: `Value` (positional, required)
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

/// # Panic
/// Fail with an error.
///
/// ## Example
/// The code below produces the error `panicked at: "this is wrong"`.
/// ```typ
/// #panic("this is wrong")
/// ```
///
/// ## Parameters
/// - payload: `Value` (positional)
///   The value (or message) to panic with.
///
/// ## Category
/// foundations
#[func]
pub fn panic(args: &mut Args) -> SourceResult<Value> {
    match args.eat::<Value>()? {
        Some(v) => bail!(args.span, "panicked with: {}", v.repr()),
        None => bail!(args.span, "panicked"),
    }
}

/// # Assert
/// Ensure that a condition is fulfilled.
///
/// Fails with an error if the condition is not fulfilled. Does not
/// produce any output in the document.
///
/// ## Example
/// ```typ
/// #assert(1 < 2, message: "math broke")
/// ```
///
/// ## Parameters
/// - condition: `bool` (positional, required)
///   The condition that must be true for the assertion to pass.
/// - message: `EcoString` (named)
///   The error message when the assertion fails.
///
/// ## Category
/// foundations
#[func]
pub fn assert(args: &mut Args) -> SourceResult<Value> {
    let check = args.expect::<bool>("condition")?;
    let message = args.named::<EcoString>("message")?;
    if !check {
        if let Some(message) = message {
            bail!(args.span, "assertion failed: {}", message);
        } else {
            bail!(args.span, "assertion failed");
        }
    }
    Ok(Value::None)
}

/// # Evaluate
/// Evaluate a string as Typst code.
///
/// This function should only be used as a last resort.
///
/// ## Example
/// ```example
/// #eval("1 + 1") \
/// #eval("(1, 2, 3, 4)").len() \
/// #eval("[*Strong text*]")
/// ```
///
/// ## Parameters
/// - source: `String` (positional, required)
///   A string of Typst code to evaluate.
///
///   The code in the string cannot interact with the file system.
///
/// - returns: any
///
/// ## Category
/// foundations
#[func]
pub fn eval(vm: &Vm, args: &mut Args) -> SourceResult<Value> {
    let Spanned { v: text, span } = args.expect::<Spanned<String>>("source")?;
    typst::model::eval_code_str(vm.world(), &text, span)
}
