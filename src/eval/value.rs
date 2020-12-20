//! Computational values.

use std::fmt::{self, Debug, Formatter};
use std::ops::Deref;
use std::rc::Rc;

use fontdock::{FontStretch, FontStyle, FontWeight};

use super::{Args, Dict, Eval, EvalContext, SpannedEntry};
use crate::color::RgbaColor;
use crate::diag::Diag;
use crate::geom::{Dir, Length, Linear, Relative};
use crate::paper::Paper;
use crate::syntax::{Ident, SpanWith, Spanned, SynTree};

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
    Length(Length),
    /// A relative value: `50%`.
    Relative(Relative),
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

/// An wrapper around a reference-counted function trait object.
///
/// The dynamic function object is wrapped in an `Rc` to keep [`Value`]
/// cloneable.
///
/// _Note_: This is needed because the compiler can't `derive(PartialEq)` for
///         [`Value`] when directly putting the `Rc` in there, see the [Rust
///         Issue].
///
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

impl PartialEq for ValueFunc {
    fn eq(&self, _: &Self) -> bool {
        false
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

/// Try to convert a value into a more specific type.
pub trait TryFromValue: Sized {
    /// Try to convert the value into yourself.
    fn try_from_value(value: Spanned<Value>) -> Conv<Self>;
}

/// The result of a conversion.
#[derive(Debug, Clone, PartialEq)]
pub enum Conv<T> {
    /// Success conversion.
    Ok(T),
    /// Sucessful conversion with a warning.
    Warn(T, Diag),
    /// Unsucessful conversion, gives back the value alongside the error.
    Err(Value, Diag),
}

impl<T> Conv<T> {
    /// Map the conversion result.
    pub fn map<U>(self, f: impl FnOnce(T) -> U) -> Conv<U> {
        match self {
            Conv::Ok(t) => Conv::Ok(f(t)),
            Conv::Warn(t, warn) => Conv::Warn(f(t), warn),
            Conv::Err(v, err) => Conv::Err(v, err),
        }
    }
}

impl<T: TryFromValue> TryFromValue for Spanned<T> {
    fn try_from_value(value: Spanned<Value>) -> Conv<Self> {
        let span = value.span;
        T::try_from_value(value).map(|v| v.span_with(span))
    }
}

/// A value type that matches [identifier](Value::Ident) and [string](Value::Str) values.
pub struct StringLike(pub String);

impl From<StringLike> for String {
    fn from(like: StringLike) -> String {
        like.0
    }
}

impl Deref for StringLike {
    type Target = str;

    fn deref(&self) -> &str {
        self.0.as_str()
    }
}

/// Implement [`TryFromValue`] through a match.
macro_rules! try_from_match {
    ($type:ty[$name:literal] $(@ $span:ident)?: $($pattern:pat => $output:expr),* $(,)?) => {
        impl $crate::eval::TryFromValue for $type {
            fn try_from_value(value: Spanned<Value>) -> $crate::eval::Conv<Self> {
                use $crate::eval::Conv;
                #[allow(unused)]
                $(let $span = value.span;)?
                #[allow(unreachable_patterns)]
                match value.v {
                    $($pattern => Conv::Ok($output)),*,
                    v => {
                        let e = error!("expected {}, found {}", $name, v.ty());
                        Conv::Err(v, e)
                    }
                }
            }
        }
    };
}

/// Implement [`TryFromValue`] through a function parsing an identifier.
macro_rules! try_from_id {
    ($type:ty[$name:literal]: $from_str:expr) => {
        impl $crate::eval::TryFromValue for $type {
            fn try_from_value(value: Spanned<Value>) -> $crate::eval::Conv<Self> {
                use $crate::eval::Conv;
                let v = value.v;
                if let Value::Ident(id) = v {
                    if let Some(v) = $from_str(&id) {
                        Conv::Ok(v)
                    } else {
                        Conv::Err(Value::Ident(id), error!("invalid {}", $name))
                    }
                } else {
                    let e = error!("expected identifier, found {}", v.ty());
                    Conv::Err(v, e)
                }
            }
        }
    };
}

try_from_match!(Value["value"]: v => v);
try_from_match!(Ident["identifier"]: Value::Ident(v) => v);
try_from_match!(bool["bool"]: Value::Bool(v) => v);
try_from_match!(i64["integer"]: Value::Int(v) => v);
try_from_match!(f64["float"]:
    Value::Int(v) => v as f64,
    Value::Float(v) => v,
);
try_from_match!(Length["length"]: Value::Length(v) => v);
try_from_match!(Relative["relative"]: Value::Relative(v) => v);
try_from_match!(Linear["linear"]:
    Value::Linear(v) => v,
    Value::Length(v) => v.into(),
    Value::Relative(v) => v.into(),
);
try_from_match!(String["string"]: Value::Str(v) => v);
try_from_match!(SynTree["tree"]: Value::Content(v) => v);
try_from_match!(ValueDict["dictionary"]: Value::Dict(v) => v);
try_from_match!(ValueFunc["function"]: Value::Func(v) => v);
try_from_match!(StringLike["identifier or string"]:
    Value::Ident(Ident(v)) => Self(v),
    Value::Str(v) => Self(v),
);
try_from_id!(Dir["direction"]: |v| match v {
    "ltr" | "left-to-right" => Some(Self::LTR),
    "rtl" | "right-to-left" => Some(Self::RTL),
    "ttb" | "top-to-bottom" => Some(Self::TTB),
    "btt" | "bottom-to-top" => Some(Self::BTT),
    _ => None,
});
try_from_id!(FontStyle["font style"]: Self::from_str);
try_from_id!(FontStretch["font stretch"]: Self::from_str);
try_from_id!(Paper["paper"]: Self::from_name);

impl TryFromValue for FontWeight {
    fn try_from_value(value: Spanned<Value>) -> Conv<Self> {
        match value.v {
            Value::Int(number) => {
                let [min, max] = [Self::THIN, Self::BLACK];
                if number < i64::from(min.to_number()) {
                    Conv::Warn(min, warning!("the minimum font weight is {:#?}", min))
                } else if number > i64::from(max.to_number()) {
                    Conv::Warn(max, warning!("the maximum font weight is {:#?}", max))
                } else {
                    Conv::Ok(Self::from_number(number as u16))
                }
            }
            Value::Ident(id) => {
                if let Some(weight) = Self::from_str(&id) {
                    Conv::Ok(weight)
                } else {
                    Conv::Err(Value::Ident(id), error!("invalid font weight"))
                }
            }
            v => {
                let e = error!("expected font weight, found {}", v.ty());
                Conv::Err(v, e)
            }
        }
    }
}
