//! Foundational types and functions.

pub mod calc;
pub mod ops;
pub mod repr;
pub mod sys;

mod args;
mod array;
mod auto;
mod bool;
mod bytes;
mod cast;
mod content;
mod context;
mod datetime;
mod decimal;
mod dict;
mod duration;
mod element;
mod fields;
mod float;
mod func;
mod int;
mod label;
mod module;
mod none;
#[path = "plugin.rs"]
mod plugin_;
mod scope;
mod selector;
mod str;
mod styles;
mod symbol;
#[path = "target.rs"]
mod target_;
mod ty;
mod value;
mod version;

pub use self::args::*;
pub use self::array::*;
pub use self::auto::*;
pub use self::bytes::*;
pub use self::cast::*;
pub use self::content::*;
pub use self::context::*;
pub use self::datetime::*;
pub use self::decimal::*;
pub use self::dict::*;
pub use self::duration::*;
pub use self::element::*;
pub use self::fields::*;
pub use self::float::*;
pub use self::func::*;
pub use self::int::*;
pub use self::label::*;
pub use self::module::*;
pub use self::none::*;
pub use self::plugin_::*;
pub use self::repr::Repr;
pub use self::scope::*;
pub use self::selector::*;
pub use self::str::*;
pub use self::styles::*;
pub use self::symbol::*;
pub use self::target_::*;
pub use self::ty::*;
pub use self::value::*;
pub use self::version::*;
pub use typst_macros::{scope, ty};

#[rustfmt::skip]
#[doc(hidden)]
pub use {
    ecow::{eco_format, eco_vec},
    indexmap::IndexMap,
};

use ecow::EcoString;
use typst_syntax::Spanned;

use crate::diag::{bail, SourceResult, StrResult};
use crate::engine::Engine;
use crate::routines::EvalMode;
use crate::{Feature, Features};

/// Hook up all `foundations` definitions.
pub(super) fn define(
    global: &mut Scope,
    inputs: Dict,
    input_files: Dict,
    features: &Features,
) {
    global.start_category(crate::Category::Foundations);
    global.define_type::<bool>();
    global.define_type::<i64>();
    global.define_type::<f64>();
    global.define_type::<Str>();
    global.define_type::<Label>();
    global.define_type::<Bytes>();
    global.define_type::<Content>();
    global.define_type::<Array>();
    global.define_type::<Dict>();
    global.define_type::<Func>();
    global.define_type::<Args>();
    global.define_type::<Type>();
    global.define_type::<Module>();
    global.define_type::<Regex>();
    global.define_type::<Selector>();
    global.define_type::<Datetime>();
    global.define_type::<Decimal>();
    global.define_type::<Symbol>();
    global.define_type::<Duration>();
    global.define_type::<Version>();
    global.define_func::<repr::repr>();
    global.define_func::<panic>();
    global.define_func::<assert>();
    global.define_func::<eval>();
    global.define_func::<plugin>();
    if features.is_enabled(Feature::Html) {
        global.define_func::<target>();
    }
    global.define("calc", calc::module());
    global.define("sys", sys::module(inputs, input_files));
    global.reset_category();
}

/// Fails with an error.
///
/// Arguments are displayed to the user (not rendered in the document) as
/// strings, converting with `repr` if necessary.
///
/// # Example
/// The code below produces the error `panicked with: "this is wrong"`.
/// ```typ
/// #panic("this is wrong")
/// ```
#[func(keywords = ["error"])]
pub fn panic(
    /// The values to panic with and display to the user.
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
    engine: &mut Engine,
    /// A string of Typst code to evaluate.
    source: Spanned<String>,
    /// The [syntactical mode]($reference/syntax/#modes) in which the string is
    /// parsed.
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
        scope.bind(key.into(), Binding::new(value, span));
    }
    (engine.routines.eval_string)(engine.routines, engine.world, &text, span, mode, scope)
}
