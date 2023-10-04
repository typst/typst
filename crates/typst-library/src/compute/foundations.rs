use typst::eval::{
    Datetime, Duration, EvalMode, Module, Never, NoneValue, Plugin, Regex, Repr, Version,
};

use crate::prelude::*;

/// Hook up all foundational definitions.
pub(super) fn define(global: &mut Scope) {
    global.category("foundations");
    global.define_type::<bool>();
    global.define_type::<i64>();
    global.define_type::<f64>();
    global.define_type::<Str>();
    global.define_type::<Bytes>();
    global.define_type::<Content>();
    global.define_type::<Array>();
    global.define_type::<Dict>();
    global.define_type::<Func>();
    global.define_type::<Args>();
    global.define_type::<Type>();
    global.define_type::<Module>();
    global.define_type::<Regex>();
    global.define_type::<Datetime>();
    global.define_type::<Duration>();
    global.define_type::<Version>();
    global.define_type::<Plugin>();
    global.define_func::<repr>();
    global.define_func::<panic>();
    global.define_func::<assert>();
    global.define_func::<eval>();
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
/// # Example
/// ```example
/// #none vs #repr(none) \
/// #"hello" vs #repr("hello") \
/// #(1, 2) vs #repr((1, 2)) \
/// #[*Hi*] vs #repr([*Hi*])
/// ```
#[func(title = "Representation")]
pub fn repr(
    /// The value whose string representation to produce.
    value: Value,
) -> Str {
    value.repr().into()
}

/// Fails with an error.
///
/// # Example
/// The code below produces the error `panicked with: "this is wrong"`.
/// ```typ
/// #panic("this is wrong")
/// ```
#[func(keywords = ["error"])]
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
/// [`assert.eq`]($assert.eq) and [`assert.ne`]($assert.ne).
///
/// # Example
/// ```typ
/// #assert(1 < 2, message: "math broke")
/// ```
#[func(scope)]
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

#[scope]
impl assert {
    /// Ensures that two values are equal.
    ///
    /// Fails with an error if the first value is not equal to the second. Does not
    /// produce any output in the document.
    ///
    /// ```typ
    /// #assert.eq(10, 10)
    /// ```
    #[func(title = "Assert Equal")]
    pub fn eq(
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
                bail!(
                    "equality assertion failed: value {} was not equal to {}",
                    left.repr(),
                    right.repr()
                );
            }
        }
        Ok(NoneValue)
    }

    /// Ensures that two values are not equal.
    ///
    /// Fails with an error if the first value is equal to the second. Does not
    /// produce any output in the document.
    ///
    /// ```typ
    /// #assert.ne(3, 4)
    /// ```
    #[func(title = "Assert Not Equal")]
    pub fn ne(
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
                bail!(
                    "inequality assertion failed: value {} was equal to {}",
                    left.repr(),
                    right.repr()
                );
            }
        }
        Ok(NoneValue)
    }
}

/// Evaluates a string as Typst code.
///
/// This function should only be used as a last resort.
///
/// # Example
/// ```example
/// #eval("1 + 1") \
/// #eval("(1, 2, 3, 4)").len() \
/// #eval("*Markup!*", mode: "markup") \
/// ```
#[func(title = "Evaluate")]
pub fn eval(
    /// The virtual machine.
    vm: &mut Vm,
    /// A string of Typst code to evaluate.
    ///
    /// The code in the string cannot interact with the file system.
    source: Spanned<String>,
    /// The syntactical mode in which the string is parsed.
    ///
    /// ```example
    /// #eval("= Heading", mode: "markup")
    /// #eval("1_2^3", mode: "math")
    /// ```
    #[named]
    #[default(EvalMode::Code)]
    mode: EvalMode,
    /// A scope of definitions that are made available.
    ///
    /// ```example
    /// #eval("x + 1", scope: (x: 2)) \
    /// #eval(
    ///   "abc/xyz",
    ///   mode: "math",
    ///   scope: (
    ///     abc: $a + b + c$,
    ///     xyz: $x + y + z$,
    ///   ),
    /// )
    /// ```
    #[named]
    #[default]
    scope: Dict,
) -> SourceResult<Value> {
    let Spanned { v: text, span } = source;
    let dict = scope;
    let mut scope = Scope::new();
    for (key, value) in dict {
        scope.define(key, value);
    }
    typst::eval::eval_string(vm.world(), &text, span, mode, scope)
}
