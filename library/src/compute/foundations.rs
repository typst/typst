use crate::prelude::*;

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
/// Display: Type
/// Category: foundations
/// Returns: string
#[func]
pub fn type_(
    /// The value whose type's to determine.
    value: Value,
) -> Value {
    value.type_name().into()
}

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
/// Display: Representation
/// Category: foundations
/// Returns: string
#[func]
pub fn repr(
    /// The value whose string representation to produce.
    value: Value,
) -> Value {
    value.repr().into()
}

/// Fail with an error.
///
/// ## Example
/// The code below produces the error `panicked with: "this is wrong"`.
/// ```typ
/// #panic("this is wrong")
/// ```
///
/// Display: Panic
/// Category: foundations
/// Returns:
#[func]
pub fn panic(
    /// The values to panic with.
    #[variadic]
    values: Vec<Value>,
) -> Value {
    let mut msg = EcoString::from("panicked");
    if !values.is_empty() {
        msg.push_str(" with: ");
        for (i, value) in values.iter().enumerate() {
            if i > 0 {
                msg.push_str(", ");
            }
            msg.push_str(&value.repr());
        }
    }
    bail!(args.span, msg);
}

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
/// Display: Assert
/// Category: foundations
/// Returns:
#[func]
pub fn assert(
    /// The condition that must be true for the assertion to pass.
    condition: bool,
    /// The error message when the assertion fails.
    #[named]
    #[default]
    message: Option<EcoString>,
) -> Value {
    if !condition {
        if let Some(message) = message {
            bail!(args.span, "assertion failed: {}", message);
        } else {
            bail!(args.span, "assertion failed");
        }
    }
    Value::None
}

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
/// Display: Evaluate
/// Category: foundations
/// Returns: any
#[func]
pub fn eval(
    /// A string of Typst code to evaluate.
    ///
    /// The code in the string cannot interact with the file system.
    source: Spanned<String>,
) -> Value {
    let Spanned { v: text, span } = source;
    typst::eval::eval_string(vm.world(), &text, span)?
}
