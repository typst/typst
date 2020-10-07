//! Computational values.

use std::fmt::{self, Debug, Formatter};
use std::ops::Deref;
use std::rc::Rc;

use super::{Args, Dict, Eval, EvalContext, SpannedEntry};
use crate::color::RgbaColor;
use crate::geom::Linear;
use crate::syntax::{Ident, SynTree};

/// A computational value.
#[derive(Clone, PartialEq)]
pub enum Value {
    /// The value that indicates the absence of a meaningful value.
    None,
    /// An identifier: `ident`.
    Ident(Ident),
    /// A boolean: `true, false`.
    Bool(bool),
    /// An integer: `120`.
    Int(i64),
    /// A floating-point number: `1.2, 200%`.
    Float(f64),
    /// A length: `2cm, 5.2in`.
    Length(f64),
    /// A relative value: `50%`.
    ///
    /// _Note_: `50%` is represented as `0.5` here, but as `50.0` in the
    /// corresponding [literal].
    ///
    /// [literal]: ../syntax/ast/enum.Lit.html#variant.Percent
    Relative(f64),
    /// A combination of an absolute length and a relative value: `20% + 5cm`.
    Linear(Linear),
    /// A color value with alpha channel: `#f79143ff`.
    Color(RgbaColor),
    /// A string: `"string"`.
    Str(String),
    /// A dictionary value: `(false, 12cm, greeting="hi")`.
    Dict(ValueDict),
    /// A content value: `{*Hi* there}`.
    Content(SynTree),
    /// An executable function.
    Func(ValueFunc),
    /// The result of invalid operations.
    Error,
}

impl Value {
    /// The natural-language name of this value's type for use in error
    /// messages.
    pub fn ty(&self) -> &'static str {
        match self {
            Self::None => "none",
            Self::Ident(_) => "identifier",
            Self::Bool(_) => "bool",
            Self::Int(_) => "integer",
            Self::Float(_) => "float",
            Self::Relative(_) => "relative",
            Self::Length(_) => "length",
            Self::Linear(_) => "linear",
            Self::Color(_) => "color",
            Self::Str(_) => "string",
            Self::Dict(_) => "dict",
            Self::Content(_) => "content",
            Self::Func(_) => "function",
            Self::Error => "error",
        }
    }
}

impl Eval for Value {
    type Output = ();

    /// Evaluate everything contained in this value.
    fn eval(&self, ctx: &mut EvalContext) -> Self::Output {
        match self {
            // Don't print out none values.
            Value::None => {}

            // Pass through.
            Value::Content(tree) => tree.eval(ctx),

            // Forward to each dictionary entry.
            Value::Dict(dict) => {
                for entry in dict.values() {
                    entry.value.v.eval(ctx);
                }
            }

            // Format with debug.
            val => ctx.push(ctx.make_text_node(format!("{:?}", val))),
        }
    }
}

impl Default for Value {
    fn default() -> Self {
        Value::None
    }
}

impl Debug for Value {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        match self {
            Self::None => f.pad("none"),
            Self::Ident(v) => v.fmt(f),
            Self::Bool(v) => v.fmt(f),
            Self::Int(v) => v.fmt(f),
            Self::Float(v) => v.fmt(f),
            Self::Length(v) => v.fmt(f),
            Self::Relative(v) => v.fmt(f),
            Self::Linear(v) => v.fmt(f),
            Self::Color(v) => v.fmt(f),
            Self::Str(v) => v.fmt(f),
            Self::Dict(v) => v.fmt(f),
            Self::Content(v) => v.fmt(f),
            Self::Func(v) => v.fmt(f),
            Self::Error => f.pad("<error>"),
        }
    }
}

/// A dictionary of values.
///
/// # Example
/// ```typst
/// (false, 12cm, greeting="hi")
/// ```
pub type ValueDict = Dict<SpannedEntry<Value>>;

/// An wrapper around a reference-counted executable function value.
///
/// The dynamic function object is wrapped in an `Rc` to keep [`Value`]
/// clonable.
///
/// _Note_: This is needed because the compiler can't `derive(PartialEq)` for
///         [`Value`] when directly putting the boxed function in there, see the
///         [Rust Issue].
///
/// [`Value`]: enum.Value.html
/// [Rust Issue]: https://github.com/rust-lang/rust/issues/31740
#[derive(Clone)]
pub struct ValueFunc(pub Rc<Func>);

/// The signature of executable functions.
type Func = dyn Fn(Args, &mut EvalContext) -> Value;

impl ValueFunc {
    /// Create a new function value from a rust function or closure.
    pub fn new<F>(f: F) -> Self
    where
        F: Fn(Args, &mut EvalContext) -> Value + 'static,
    {
        Self(Rc::new(f))
    }
}

impl Eq for ValueFunc {}

impl PartialEq for ValueFunc {
    fn eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.0, &other.0)
    }
}

impl Deref for ValueFunc {
    type Target = Func;

    fn deref(&self) -> &Self::Target {
        self.0.as_ref()
    }
}

impl Debug for ValueFunc {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        f.pad("<function>")
    }
}
