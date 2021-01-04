use std::any::Any;
use std::collections::HashMap;
use std::fmt::{self, Debug, Display, Formatter};
use std::ops::Deref;
use std::rc::Rc;

use super::{Args, Eval, EvalContext};
use crate::color::Color;
use crate::geom::{Length, Linear, Relative};
use crate::syntax::{Spanned, Tree, WithSpan};

/// A computational value.
#[derive(Clone, PartialEq)]
pub enum Value {
    /// The value that indicates the absence of a meaningful value.
    None,
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
    /// A color value: `#f79143ff`.
    Color(Color),
    /// A string: `"string"`.
    Str(String),
    /// An array value: `(1, "hi", 12cm)`.
    Array(ValueArray),
    /// A dictionary value: `(color: #f79143, pattern: dashed)`.
    Dict(ValueDict),
    /// A content value: `{*Hi* there}`.
    Content(ValueContent),
    /// An executable function.
    Func(ValueFunc),
    /// Any object.
    Any(ValueAny),
    /// The result of invalid operations.
    Error,
}

impl Value {
    /// Try to cast the value into a specific type.
    pub fn cast<T>(self) -> CastResult<T, Self>
    where
        T: Cast<Value>,
    {
        T::cast(self)
    }

    /// The name of the stored value's type.
    pub fn type_name(&self) -> &'static str {
        match self {
            Self::None => "none",
            Self::Bool(_) => bool::TYPE_NAME,
            Self::Int(_) => i64::TYPE_NAME,
            Self::Float(_) => f64::TYPE_NAME,
            Self::Length(_) => Length::TYPE_NAME,
            Self::Relative(_) => Relative::TYPE_NAME,
            Self::Linear(_) => Linear::TYPE_NAME,
            Self::Color(_) => Color::TYPE_NAME,
            Self::Str(_) => String::TYPE_NAME,
            Self::Array(_) => ValueArray::TYPE_NAME,
            Self::Dict(_) => ValueDict::TYPE_NAME,
            Self::Content(_) => ValueContent::TYPE_NAME,
            Self::Func(_) => ValueFunc::TYPE_NAME,
            Self::Any(v) => v.type_name(),
            Self::Error => "error",
        }
    }
}

impl Eval for &Value {
    type Output = ();

    /// Evaluate everything contained in this value.
    fn eval(self, ctx: &mut EvalContext) -> Self::Output {
        ctx.push(ctx.make_text_node(match self {
            Value::None => return,
            Value::Bool(v) => v.to_string(),
            Value::Int(v) => v.to_string(),
            Value::Float(v) => v.to_string(),
            Value::Length(v) => v.to_string(),
            Value::Relative(v) => v.to_string(),
            Value::Linear(v) => v.to_string(),
            Value::Color(v) => v.to_string(),
            Value::Str(v) => v.clone(),
            // TODO: Find good representation for composite types.
            Value::Array(_v) => "(array)".into(),
            Value::Dict(_v) => "(dictionary)".into(),
            Value::Content(tree) => return tree.eval(ctx),
            Value::Func(v) => v.to_string(),
            Value::Any(v) => v.to_string(),
            Value::Error => "(error)".into(),
        }));
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
            Self::None => f.pad("None"),
            Self::Bool(v) => Debug::fmt(v, f),
            Self::Int(v) => Debug::fmt(v, f),
            Self::Float(v) => Debug::fmt(v, f),
            Self::Length(v) => Debug::fmt(v, f),
            Self::Relative(v) => Debug::fmt(v, f),
            Self::Linear(v) => Debug::fmt(v, f),
            Self::Color(v) => Debug::fmt(v, f),
            Self::Str(v) => Debug::fmt(v, f),
            Self::Array(v) => Debug::fmt(v, f),
            Self::Dict(v) => Debug::fmt(v, f),
            Self::Content(v) => Debug::fmt(v, f),
            Self::Func(v) => Debug::fmt(v, f),
            Self::Any(v) => Debug::fmt(v, f),
            Self::Error => f.pad("Error"),
        }
    }
}

/// An array value: `(1, "hi", 12cm)`.
pub type ValueArray = Vec<Value>;

/// A dictionary value: `(color: #f79143, pattern: dashed)`.
pub type ValueDict = HashMap<String, Value>;

/// A content value: `{*Hi* there}`.
pub type ValueContent = Tree;

/// A wrapper around a reference-counted executable function.
#[derive(Clone)]
pub struct ValueFunc {
    name: String,
    f: Rc<dyn Fn(&mut EvalContext, &mut Args) -> Value>,
}

impl ValueFunc {
    /// Create a new function value from a rust function or closure.
    pub fn new<F>(name: impl Into<String>, f: F) -> Self
    where
        F: Fn(&mut EvalContext, &mut Args) -> Value + 'static,
    {
        Self { name: name.into(), f: Rc::new(f) }
    }
}

impl PartialEq for ValueFunc {
    fn eq(&self, _: &Self) -> bool {
        false
    }
}

impl Deref for ValueFunc {
    type Target = dyn Fn(&mut EvalContext, &mut Args) -> Value;

    fn deref(&self) -> &Self::Target {
        self.f.as_ref()
    }
}

impl Display for ValueFunc {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        write!(f, "<function {}>", self.name)
    }
}
impl Debug for ValueFunc {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        Display::fmt(self, f)
    }
}

/// A wrapper around a dynamic value.
pub struct ValueAny(Box<dyn Bounds>);

impl ValueAny {
    /// Create a new instance from any value that satisifies the required bounds.
    pub fn new<T>(any: T) -> Self
    where
        T: Type + Debug + Display + Clone + PartialEq + 'static,
    {
        Self(Box::new(any))
    }

    /// Whether the wrapped type is `T`.
    pub fn is<T: 'static>(&self) -> bool {
        self.0.as_any().is::<T>()
    }

    /// Try to downcast to a specific type.
    pub fn downcast<T: 'static>(self) -> Result<T, Self> {
        if self.is::<T>() {
            Ok(*self.0.into_any().downcast().unwrap())
        } else {
            Err(self)
        }
    }

    /// Try to downcast to a reference to a specific type.
    pub fn downcast_ref<T: 'static>(&self) -> Option<&T> {
        self.0.as_any().downcast_ref()
    }

    /// The name of the stored value's type.
    pub fn type_name(&self) -> &'static str {
        self.0.dyn_type_name()
    }
}

impl Clone for ValueAny {
    fn clone(&self) -> Self {
        Self(self.0.dyn_clone())
    }
}

impl PartialEq for ValueAny {
    fn eq(&self, other: &Self) -> bool {
        self.0.dyn_eq(other)
    }
}

impl Display for ValueAny {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        Display::fmt(&self.0, f)
    }
}

impl Debug for ValueAny {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        Debug::fmt(&self.0, f)
    }
}

trait Bounds: Debug + Display + 'static {
    fn as_any(&self) -> &dyn Any;
    fn into_any(self: Box<Self>) -> Box<dyn Any>;
    fn dyn_eq(&self, other: &ValueAny) -> bool;
    fn dyn_clone(&self) -> Box<dyn Bounds>;
    fn dyn_type_name(&self) -> &'static str;
}

impl<T> Bounds for T
where
    T: Type + Debug + Display + Clone + PartialEq + 'static,
{
    fn as_any(&self) -> &dyn Any {
        self
    }

    fn into_any(self: Box<Self>) -> Box<dyn Any> {
        self
    }

    fn dyn_eq(&self, other: &ValueAny) -> bool {
        if let Some(other) = other.downcast_ref::<Self>() {
            self == other
        } else {
            false
        }
    }

    fn dyn_clone(&self) -> Box<dyn Bounds> {
        Box::new(self.clone())
    }

    fn dyn_type_name(&self) -> &'static str {
        T::TYPE_NAME
    }
}

/// Types that can be stored in values.
pub trait Type {
    /// The name of the type.
    const TYPE_NAME: &'static str;
}

impl<T> Type for Spanned<T>
where
    T: Type,
{
    const TYPE_NAME: &'static str = T::TYPE_NAME;
}

/// Cast from a value to a specific type.
pub trait Cast<V>: Type + Sized {
    /// Try to cast the value into an instance of `Self`.
    fn cast(value: V) -> CastResult<Self, V>;
}

/// The result of casting a value to a specific type.
#[derive(Debug, Clone, Eq, PartialEq)]
pub enum CastResult<T, V> {
    /// The value was cast successfully.
    Ok(T),
    /// The value was cast successfully, but with a warning message.
    Warn(T, String),
    /// The value could not be cast into the specified type.
    Err(V),
}

impl<T, V> CastResult<T, V> {
    /// Access the conversion result, discarding a possibly existing warning.
    pub fn ok(self) -> Option<T> {
        match self {
            CastResult::Ok(t) | CastResult::Warn(t, _) => Some(t),
            CastResult::Err(_) => None,
        }
    }
}

impl Type for Value {
    const TYPE_NAME: &'static str = "value";
}

impl Cast<Value> for Value {
    fn cast(value: Value) -> CastResult<Self, Value> {
        CastResult::Ok(value)
    }
}

impl<T> Cast<Spanned<Value>> for T
where
    T: Cast<Value>,
{
    fn cast(value: Spanned<Value>) -> CastResult<Self, Spanned<Value>> {
        let span = value.span;
        match T::cast(value.v) {
            CastResult::Ok(t) => CastResult::Ok(t),
            CastResult::Warn(t, m) => CastResult::Warn(t, m),
            CastResult::Err(v) => CastResult::Err(v.with_span(span)),
        }
    }
}

impl<T> Cast<Spanned<Value>> for Spanned<T>
where
    T: Cast<Value>,
{
    fn cast(value: Spanned<Value>) -> CastResult<Self, Spanned<Value>> {
        let span = value.span;
        match T::cast(value.v) {
            CastResult::Ok(t) => CastResult::Ok(t.with_span(span)),
            CastResult::Warn(t, m) => CastResult::Warn(t.with_span(span), m),
            CastResult::Err(v) => CastResult::Err(v.with_span(span)),
        }
    }
}

macro_rules! impl_primitive {
    ($type:ty:
        $type_name:literal,
        $variant:path
        $(, $pattern:pat => $out:expr)* $(,)?
    ) => {
        impl Type for $type {
            const TYPE_NAME: &'static str = $type_name;
        }

        impl From<$type> for Value {
            fn from(v: $type) -> Self {
                $variant(v)
            }
        }

        impl Cast<Value> for $type {
            fn cast(value: Value) -> CastResult<Self, Value> {
                match value {
                    $variant(v) => CastResult::Ok(v),
                    $($pattern => CastResult::Ok($out),)*
                    v => CastResult::Err(v),
                }
            }
        }
    };
}

impl_primitive! { bool: "boolean", Value::Bool }
impl_primitive! { i64: "integer", Value::Int }
impl_primitive! { Length: "length", Value::Length }
impl_primitive! { Relative: "relative", Value::Relative }
impl_primitive! { Color: "color", Value::Color }
impl_primitive! { String: "string", Value::Str }
impl_primitive! { ValueArray: "array", Value::Array }
impl_primitive! { ValueDict: "dictionary", Value::Dict }
impl_primitive! { ValueContent: "content", Value::Content }
impl_primitive! { ValueFunc: "function", Value::Func }

impl_primitive! {
    f64: "float",
    Value::Float,
    Value::Int(v) => v as f64,
}

impl_primitive! {
    Linear: "linear",
    Value::Linear,
    Value::Length(v) => v.into(),
    Value::Relative(v) => v.into(),
}

impl From<&str> for Value {
    fn from(v: &str) -> Self {
        Self::Str(v.to_string())
    }
}

impl From<ValueAny> for Value {
    fn from(v: ValueAny) -> Self {
        Self::Any(v)
    }
}

/// Make a type usable as a [`Value`].
///
/// Given a type `T`, this always implements the following traits:
/// - [`Type`] for `T`,
/// - [`Cast<Value>`](Cast) for `T`.
#[macro_export]
macro_rules! impl_type {
    ($type:ty:
        $type_name:literal
        $(, $pattern:pat => $out:expr)*
        $(, #($anyvar:ident: $anytype:ty) => $anyout:expr)*
        $(,)?
    ) => {
        impl $crate::eval::Type for $type {
            const TYPE_NAME: &'static str = $type_name;
        }

        impl $crate::eval::Cast<$crate::eval::Value> for $type {
            fn cast(
                value: $crate::eval::Value,
            ) -> $crate::eval::CastResult<Self, $crate::eval::Value> {
                use $crate::eval::*;

                #[allow(unreachable_code)]
                match value {
                    $($pattern => CastResult::Ok($out),)*
                    Value::Any(mut any) => {
                        any = match any.downcast::<Self>() {
                            Ok(t) => return CastResult::Ok(t),
                            Err(any) => any,
                        };

                        $(any = match any.downcast::<$anytype>() {
                            Ok($anyvar) => return CastResult::Ok($anyout),
                            Err(any) => any,
                        };)*

                        CastResult::Err(Value::Any(any))
                    },
                    v => CastResult::Err(v),
                }
            }
        }
    };
}
