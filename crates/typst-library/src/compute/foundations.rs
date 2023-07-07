use crate::prelude::*;

/// Determines the type of a value.
///
/// Returns the name of the value's type.
///
/// ## Example { #example }
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
#[func]
pub fn type_(
    /// The value whose type's to determine.
    value: Value,
) -> Str {
    value.type_name().into()
}

/// Returns the string representation of a value.
///
/// When inserted into content, most values are displayed as this representation
/// in monospace with syntax-highlighting. The exceptions are `{none}`,
/// integers, floats, strings, content, and functions.
///
/// **Note:** This function is for debugging purposes. Its output should not be
/// considered stable and may change at any time!
///
/// ## Example { #example }
/// ```example
/// #none vs #repr(none) \
/// #"hello" vs #repr("hello") \
/// #(1, 2) vs #repr((1, 2)) \
/// #[*Hi*] vs #repr([*Hi*])
/// ```
///
/// Display: Representation
/// Category: foundations
#[func]
pub fn repr(
    /// The value whose string representation to produce.
    value: Value,
) -> Str {
    value.repr()
}

/// Fails with an error.
///
/// ## Example { #example }
/// The code below produces the error `panicked with: "this is wrong"`.
/// ```typ
/// #panic("this is wrong")
/// ```
///
/// Display: Panic
/// Category: foundations
#[func]
pub fn panic(
    /// The values to panic with.
    #[variadic]
    values: Vec<Value>,
) -> StrResult<Never> {
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
    Err(msg)
}

/// Ensures that a condition is fulfilled.
///
/// Fails with an error if the condition is not fulfilled. Does not
/// produce any output in the document.
///
/// If you wish to test equality between two values, see
/// [`assert.eq`]($func/assert.eq) and [`assert.ne`]($func/assert.ne).
///
/// ## Example { #example }
/// ```typ
/// #assert(1 < 2, message: "math broke")
/// ```
///
/// Display: Assert
/// Category: foundations
#[func]
#[scope(
    scope.define("eq", assert_eq_func());
    scope.define("ne", assert_ne_func());
    scope
)]
pub fn assert(
    /// The condition that must be true for the assertion to pass.
    condition: bool,
    /// The error message when the assertion fails.
    #[named]
    message: Option<EcoString>,
) -> StrResult<NoneValue> {
    if !condition {
        if let Some(message) = message {
            bail!("assertion failed: {message}");
        } else {
            bail!("assertion failed");
        }
    }
    Ok(NoneValue)
}

/// Ensures that two values are equal.
///
/// Fails with an error if the first value is not equal to the second. Does not
/// produce any output in the document.
///
/// ## Example { #example }
/// ```typ
/// #assert.eq(10, 10)
/// ```
///
/// Display: Assert Equals
/// Category: foundations
#[func]
pub fn assert_eq(
    /// The first value to compare.
    left: Value,

    /// The second value to compare.
    right: Value,

    /// An optional message to display on error instead of the representations
    /// of the compared values.
    #[named]
    message: Option<EcoString>,
) -> StrResult<NoneValue> {
    if left != right {
        if let Some(message) = message {
            bail!("equality assertion failed: {message}");
        } else {
            bail!("equality assertion failed: value {left:?} was not equal to {right:?}");
        }
    }
    Ok(NoneValue)
}

/// Ensures that two values are not equal.
///
/// Fails with an error if the first value is equal to the second. Does not
/// produce any output in the document.
///
/// ## Example { #example }
/// ```typ
/// #assert.ne(3, 4)
/// ```
///
/// Display: Assert Not Equals
/// Category: foundations
#[func]
pub fn assert_ne(
    /// The first value to compare.
    left: Value,

    /// The second value to compare.
    right: Value,

    /// An optional message to display on error instead of the representations
    /// of the compared values.
    #[named]
    message: Option<EcoString>,
) -> StrResult<NoneValue> {
    if left == right {
        if let Some(message) = message {
            bail!("inequality assertion failed: {message}");
        } else {
            bail!("inequality assertion failed: value {left:?} was equal to {right:?}");
        }
    }
    Ok(NoneValue)
}

/// Evaluates a string as Typst code.
///
/// This function should only be used as a last resort.
///
/// ## Example { #example }
/// ```example
/// #eval("1 + 1") \
/// #eval("(1, 2, 3, 4)").len() \
/// #eval("[*Strong text*]")
/// ```
///
/// Display: Evaluate
/// Category: foundations
#[func]
pub fn eval(
    /// A string of Typst code to evaluate.
    ///
    /// The code in the string cannot interact with the file system.
    source: Spanned<String>,
    /// The virtual machine.
    vm: &mut Vm,
) -> SourceResult<Value> {
    let Spanned { v: text, span } = source;
    typst::eval::eval_string(vm.world(), &text, span)
}
