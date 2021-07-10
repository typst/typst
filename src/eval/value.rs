use std::any::Any;
use std::cmp::Ordering;
use std::fmt::{self, Debug, Display, Formatter};

use super::{ops, Array, Dict, EvalContext, Function, Template, TemplateFunc};
use crate::color::{Color, RgbaColor};
use crate::eco::EcoString;
use crate::exec::ExecContext;
use crate::geom::{Angle, Fractional, Length, Linear, Relative};
use crate::syntax::{Span, Spanned};

/// A computational value.
#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    /// The value that indicates the absence of a meaningful value.
    None,
    /// A value that indicates some smart default behaviour.
    Auto,
    /// A boolean: `true, false`.
    Bool(bool),
    /// An integer: `120`.
    Int(i64),
    /// A floating-point number: `1.2`, `10e-4`.
    Float(f64),
    /// A length: `12pt`, `3cm`.
    Length(Length),
    /// An angle:  `1.5rad`, `90deg`.
    Angle(Angle),
    /// A relative value: `50%`.
    Relative(Relative),
    /// A combination of an absolute length and a relative value: `20% + 5cm`.
    Linear(Linear),
    /// A fractional value: `1fr`.
    Fractional(Fractional),
    /// A color value: `#f79143ff`.
    Color(Color),
    /// A string: `"string"`.
    Str(EcoString),
    /// An array of values: `(1, "hi", 12cm)`.
    Array(Array),
    /// A dictionary value: `(color: #f79143, pattern: dashed)`.
    Dict(Dict),
    /// A template value: `[*Hi* there]`.
    Template(Template),
    /// An executable function.
    Func(Function),
    /// Any object.
    Any(AnyValue),
    /// The result of invalid operations.
    Error,
}

impl Value {
    /// Create a new template consisting of a single function node.
    pub fn template<F>(f: F) -> Self
    where
        F: Fn(&mut ExecContext) + 'static,
    {
        Self::Template(TemplateFunc::new(f).into())
    }

    /// The name of the stored value's type.
    pub fn type_name(&self) -> &'static str {
        match self {
            Self::None => "none",
            Self::Auto => "auto",
            Self::Bool(_) => bool::TYPE_NAME,
            Self::Int(_) => i64::TYPE_NAME,
            Self::Float(_) => f64::TYPE_NAME,
            Self::Length(_) => Length::TYPE_NAME,
            Self::Angle(_) => Angle::TYPE_NAME,
            Self::Relative(_) => Relative::TYPE_NAME,
            Self::Linear(_) => Linear::TYPE_NAME,
            Self::Fractional(_) => Fractional::TYPE_NAME,
            Self::Color(_) => Color::TYPE_NAME,
            Self::Str(_) => EcoString::TYPE_NAME,
            Self::Array(_) => Array::TYPE_NAME,
            Self::Dict(_) => Dict::TYPE_NAME,
            Self::Template(_) => Template::TYPE_NAME,
            Self::Func(_) => Function::TYPE_NAME,
            Self::Any(v) => v.type_name(),
            Self::Error => "error",
        }
    }

    /// Recursively compute whether two values are equal.
    pub fn eq(&self, rhs: &Self) -> bool {
        match (self, rhs) {
            (&Self::Int(a), &Self::Float(b)) => a as f64 == b,
            (&Self::Float(a), &Self::Int(b)) => a == b as f64,
            (&Self::Length(a), &Self::Linear(b)) => a == b.abs && b.rel.is_zero(),
            (&Self::Relative(a), &Self::Linear(b)) => a == b.rel && b.abs.is_zero(),
            (&Self::Linear(a), &Self::Length(b)) => a.abs == b && a.rel.is_zero(),
            (&Self::Linear(a), &Self::Relative(b)) => a.rel == b && a.abs.is_zero(),
            (Self::Array(a), Self::Array(b)) => {
                a.len() == b.len() && a.iter().zip(b).all(|(x, y)| x.eq(y))
            }
            (Self::Dict(a), Self::Dict(b)) => {
                a.len() == b.len()
                    && a.iter().all(|(k, x)| b.get(k).map_or(false, |y| x.eq(y)))
            }
            (a, b) => a == b,
        }
    }

    /// Compare a value with another value.
    pub fn cmp(&self, rhs: &Self) -> Option<Ordering> {
        match (self, rhs) {
            (Self::Int(a), Self::Int(b)) => a.partial_cmp(b),
            (Self::Int(a), Self::Float(b)) => (*a as f64).partial_cmp(b),
            (Self::Float(a), Self::Int(b)) => a.partial_cmp(&(*b as f64)),
            (Self::Float(a), Self::Float(b)) => a.partial_cmp(b),
            (Self::Angle(a), Self::Angle(b)) => a.partial_cmp(b),
            (Self::Length(a), Self::Length(b)) => a.partial_cmp(b),
            _ => None,
        }
    }

    /// Try to cast the value into a specific type.
    pub fn cast<T>(self) -> CastResult<T, Self>
    where
        T: Cast<Value>,
    {
        T::cast(self)
    }

    /// Join with another value.
    pub fn join(self, ctx: &mut EvalContext, other: Self, span: Span) -> Self {
        let (lhs, rhs) = (self.type_name(), other.type_name());
        match ops::join(self, other) {
            Ok(joined) => joined,
            Err(prev) => {
                ctx.diag(error!(span, "cannot join {} with {}", lhs, rhs));
                prev
            }
        }
    }
}

impl Default for Value {
    fn default() -> Self {
        Value::None
    }
}

/// A wrapper around a dynamic value.
pub struct AnyValue(Box<dyn Bounds>);

impl AnyValue {
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

impl Display for AnyValue {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        Display::fmt(&self.0, f)
    }
}

impl Debug for AnyValue {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        f.debug_tuple("ValueAny").field(&self.0).finish()
    }
}

impl Clone for AnyValue {
    fn clone(&self) -> Self {
        Self(self.0.dyn_clone())
    }
}

impl PartialEq for AnyValue {
    fn eq(&self, other: &Self) -> bool {
        self.0.dyn_eq(other)
    }
}

trait Bounds: Debug + Display + 'static {
    fn as_any(&self) -> &dyn Any;
    fn into_any(self: Box<Self>) -> Box<dyn Any>;
    fn dyn_eq(&self, other: &AnyValue) -> bool;
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

    fn dyn_eq(&self, other: &AnyValue) -> bool {
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
            CastResult::Err(v) => CastResult::Err(Spanned::new(v, span)),
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
            CastResult::Ok(t) => CastResult::Ok(Spanned::new(t, span)),
            CastResult::Warn(t, m) => CastResult::Warn(Spanned::new(t, span), m),
            CastResult::Err(v) => CastResult::Err(Spanned::new(v, span)),
        }
    }
}

macro_rules! primitive {
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

primitive! { bool: "boolean", Value::Bool }
primitive! { i64: "integer", Value::Int }
primitive! {
    f64: "float",
    Value::Float,
    Value::Int(v) => v as f64,
}
primitive! { Length: "length", Value::Length }
primitive! { Angle: "angle", Value::Angle }
primitive! { Relative: "relative", Value::Relative }
primitive! {
    Linear: "linear",
    Value::Linear,
    Value::Length(v) => v.into(),
    Value::Relative(v) => v.into(),
}
primitive! { Fractional: "fractional", Value::Fractional }
primitive! { Color: "color", Value::Color }
primitive! { EcoString: "string", Value::Str }
primitive! { Array: "array", Value::Array }
primitive! { Dict: "dictionary", Value::Dict }
primitive! {
    Template: "template",
    Value::Template,
    Value::Str(v) => v.into(),
}
primitive! { Function: "function", Value::Func }

impl From<i32> for Value {
    fn from(v: i32) -> Self {
        Self::Int(v as i64)
    }
}

impl From<usize> for Value {
    fn from(v: usize) -> Self {
        Self::Int(v as i64)
    }
}

impl From<String> for Value {
    fn from(v: String) -> Self {
        Self::Str(v.into())
    }
}

impl From<&str> for Value {
    fn from(v: &str) -> Self {
        Self::Str(v.into())
    }
}

impl From<RgbaColor> for Value {
    fn from(v: RgbaColor) -> Self {
        Self::Color(Color::Rgba(v))
    }
}

impl From<AnyValue> for Value {
    fn from(v: AnyValue) -> Self {
        Self::Any(v)
    }
}

/// Make a type castable from a value.
///
/// Given a type `T`, this implements the following traits:
/// - [`Type`] for `T`,
/// - [`Cast<Value>`](Cast) for `T`.
///
/// # Example
/// ```
/// # use typst::value;
/// enum FontFamily {
///     Serif,
///     Named(String),
/// }
///
/// value! {
///     FontFamily: "font family",
///     Value::Str(string) => Self::Named(string),
/// }
/// ```
/// This would allow the type `FontFamily` to be cast from:
/// - a [`Value::Any`] variant already containing a `FontFamily`,
/// - a string, producing a named font family.
macro_rules! castable {
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
