use std::any::Any;
use std::cmp::Ordering;
use std::fmt::{self, Debug, Display, Formatter};
use std::rc::Rc;

use super::{ops, Array, Dict, Function, Str, Template};
use crate::color::{Color, RgbaColor};
use crate::diag::StrResult;
use crate::geom::{Angle, Fractional, Length, Linear, Relative};
use crate::syntax::Spanned;
use crate::util::EcoString;

/// A computational value.
#[derive(Debug, Clone)]
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
    Str(Str),
    /// An array of values: `(1, "hi", 12cm)`.
    Array(Array),
    /// A dictionary value: `(color: #f79143, pattern: dashed)`.
    Dict(Dict),
    /// A template value: `[*Hi* there]`.
    Template(Template),
    /// An executable function.
    Func(Function),
    /// A dynamic value.
    Dyn(Dynamic),
}

impl Value {
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
            Self::Str(_) => Str::TYPE_NAME,
            Self::Array(_) => Array::TYPE_NAME,
            Self::Dict(_) => Dict::TYPE_NAME,
            Self::Template(_) => Template::TYPE_NAME,
            Self::Func(_) => Function::TYPE_NAME,
            Self::Dyn(v) => v.type_name(),
        }
    }

    /// Check whether the value is castable into a specific type.
    pub fn is<T>(&self) -> bool
    where
        T: Cast<Value>,
    {
        T::is(self)
    }

    /// Try to cast the value into a specific type.
    pub fn cast<T>(self) -> StrResult<T>
    where
        T: Cast<Value>,
    {
        T::cast(self)
    }

    /// Join the value with another value.
    pub fn join(self, rhs: Self) -> StrResult<Self> {
        ops::join(self, rhs)
    }
}

impl Default for Value {
    fn default() -> Self {
        Value::None
    }
}

impl Display for Value {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        match self {
            Self::None => f.pad("none"),
            Self::Auto => f.pad("auto"),
            Self::Bool(v) => Display::fmt(v, f),
            Self::Int(v) => Display::fmt(v, f),
            Self::Float(v) => Display::fmt(v, f),
            Self::Length(v) => Display::fmt(v, f),
            Self::Angle(v) => Display::fmt(v, f),
            Self::Relative(v) => Display::fmt(v, f),
            Self::Linear(v) => Display::fmt(v, f),
            Self::Fractional(v) => Display::fmt(v, f),
            Self::Color(v) => Display::fmt(v, f),
            Self::Str(v) => Display::fmt(v, f),
            Self::Array(v) => Display::fmt(v, f),
            Self::Dict(v) => Display::fmt(v, f),
            Self::Template(v) => Display::fmt(v, f),
            Self::Func(v) => Display::fmt(v, f),
            Self::Dyn(v) => Display::fmt(v, f),
        }
    }
}

impl PartialEq for Value {
    fn eq(&self, other: &Self) -> bool {
        ops::equal(self, other)
    }
}

impl PartialOrd for Value {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        ops::compare(self, other)
    }
}

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

impl From<RgbaColor> for Value {
    fn from(v: RgbaColor) -> Self {
        Self::Color(Color::Rgba(v))
    }
}

impl From<&str> for Value {
    fn from(v: &str) -> Self {
        Self::Str(v.into())
    }
}

impl From<String> for Value {
    fn from(v: String) -> Self {
        Self::Str(v.into())
    }
}

impl From<EcoString> for Value {
    fn from(v: EcoString) -> Self {
        Self::Str(v.into())
    }
}

impl From<Dynamic> for Value {
    fn from(v: Dynamic) -> Self {
        Self::Dyn(v)
    }
}
/// A dynamic value.
#[derive(Clone)]
pub struct Dynamic(Rc<dyn Bounds>);

impl Dynamic {
    /// Create a new instance from any value that satisifies the required bounds.
    pub fn new<T>(any: T) -> Self
    where
        T: Type + Debug + Display + PartialEq + 'static,
    {
        Self(Rc::new(any))
    }

    /// Whether the wrapped type is `T`.
    pub fn is<T: 'static>(&self) -> bool {
        self.0.as_any().is::<T>()
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

impl Debug for Dynamic {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        Debug::fmt(&self.0, f)
    }
}

impl Display for Dynamic {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        Display::fmt(&self.0, f)
    }
}

impl PartialEq for Dynamic {
    fn eq(&self, other: &Self) -> bool {
        self.0.dyn_eq(other)
    }
}

trait Bounds: Debug + Display + 'static {
    fn as_any(&self) -> &dyn Any;
    fn dyn_eq(&self, other: &Dynamic) -> bool;
    fn dyn_type_name(&self) -> &'static str;
}

impl<T> Bounds for T
where
    T: Type + Debug + Display + PartialEq + 'static,
{
    fn as_any(&self) -> &dyn Any {
        self
    }

    fn dyn_eq(&self, other: &Dynamic) -> bool {
        if let Some(other) = other.downcast_ref::<Self>() {
            self == other
        } else {
            false
        }
    }

    fn dyn_type_name(&self) -> &'static str {
        T::TYPE_NAME
    }
}

/// The type of a value.
pub trait Type {
    /// The name of the type.
    const TYPE_NAME: &'static str;
}

/// Cast from a value to a specific type.
pub trait Cast<V>: Sized {
    /// Check whether the value is castable to `Self`.
    fn is(value: &V) -> bool;

    /// Try to cast the value into an instance of `Self`.
    fn cast(value: V) -> StrResult<Self>;
}

impl Cast<Value> for Value {
    fn is(_: &Value) -> bool {
        true
    }

    fn cast(value: Value) -> StrResult<Self> {
        Ok(value)
    }
}

impl<T> Cast<Spanned<Value>> for T
where
    T: Cast<Value>,
{
    fn is(value: &Spanned<Value>) -> bool {
        T::is(&value.v)
    }

    fn cast(value: Spanned<Value>) -> StrResult<Self> {
        T::cast(value.v)
    }
}

impl<T> Cast<Spanned<Value>> for Spanned<T>
where
    T: Cast<Value>,
{
    fn is(value: &Spanned<Value>) -> bool {
        T::is(&value.v)
    }

    fn cast(value: Spanned<Value>) -> StrResult<Self> {
        let span = value.span;
        T::cast(value.v).map(|t| Spanned::new(t, span))
    }
}

/// Implement traits for primitives.
macro_rules! primitive {
    (
        $type:ty: $name:literal, $variant:ident
        $(, $other:ident($binding:ident) => $out:expr)*
    ) => {
        impl Type for $type {
            const TYPE_NAME: &'static str = $name;
        }

        impl Cast<Value> for $type {
            fn is(value: &Value) -> bool {
                matches!(value, Value::$variant(_) $(| Value::$other(_))*)
            }

            fn cast(value: Value) -> StrResult<Self> {
                match value {
                    Value::$variant(v) => Ok(v),
                    $(Value::$other($binding) => Ok($out),)*
                    v => Err(format!(
                        "expected {}, found {}",
                        Self::TYPE_NAME,
                        v.type_name(),
                    )),
                }
            }
        }

        impl From<$type> for Value {
            fn from(v: $type) -> Self {
                Value::$variant(v)
            }
        }
    };
}

/// Implement traits for dynamic types.
macro_rules! dynamic {
    ($type:ty: $name:literal, $($tts:tt)*) => {
        impl $crate::eval::Type for $type {
            const TYPE_NAME: &'static str = $name;
        }

        castable! {
            $type: <Self as $crate::eval::Type>::TYPE_NAME,
            $($tts)*
            @this: Self => this.clone(),
        }

        impl From<$type> for $crate::eval::Value {
            fn from(v: $type) -> Self {
                $crate::eval::Value::Dyn($crate::eval::Dynamic::new(v))
            }
        }
    };
}

/// Make a type castable from a value.
macro_rules! castable {
    (
        $type:ty:
        $expected:expr,
        $($pattern:pat => $out:expr,)*
        $(@$dyn_in:ident: $dyn_type:ty => $dyn_out:expr,)*
    ) => {
        impl $crate::eval::Cast<$crate::eval::Value> for $type {
            fn is(value: &Value) -> bool {
                #[allow(unused_variables)]
                match value {
                    $($pattern => true,)*
                    $crate::eval::Value::Dyn(dynamic) => {
                        false $(|| dynamic.is::<$dyn_type>())*
                    }
                    _ => false,
                }
            }

            fn cast(value: $crate::eval::Value) -> $crate::diag::StrResult<Self> {
                let found = match value {
                    $($pattern => return Ok($out),)*
                    $crate::eval::Value::Dyn(dynamic) => {
                        $(if let Some($dyn_in) = dynamic.downcast_ref::<$dyn_type>() {
                            return Ok($dyn_out);
                        })*
                        dynamic.type_name()
                    }
                    v => v.type_name(),
                };

                Err(format!("expected {}, found {}", $expected, found))
            }
        }
    };
}

primitive! { bool: "boolean", Bool }
primitive! { i64: "integer", Int }
primitive! { f64: "float", Float, Int(v) => v as f64 }
primitive! { Length: "length", Length }
primitive! { Angle: "angle", Angle }
primitive! { Relative: "relative", Relative }
primitive! { Linear: "linear", Linear, Length(v) => v.into(), Relative(v) => v.into() }
primitive! { Fractional: "fractional", Fractional }
primitive! { Color: "color", Color }
primitive! { Str: "string", Str }
primitive! { Array: "array", Array }
primitive! { Dict: "dictionary", Dict }
primitive! { Template: "template", Template }
primitive! { Function: "function", Func }

#[cfg(test)]
mod tests {
    use super::*;

    #[track_caller]
    fn test(value: impl Into<Value>, exp: &str) {
        assert_eq!(value.into().to_string(), exp);
    }

    #[test]
    fn test_value_to_string() {
        // Primitives.
        test(Value::None, "none");
        test(false, "false");
        test(12i64, "12");
        test(3.14, "3.14");
        test(Length::pt(5.5), "5.5pt");
        test(Angle::deg(90.0), "90deg");
        test(Relative::one() / 2.0, "50%");
        test(Relative::new(0.3) + Length::cm(2.0), "30% + 2cm");
        test(Fractional::one() * 7.55, "7.55fr");
        test(Color::Rgba(RgbaColor::new(1, 1, 1, 0xff)), "#010101");

        // Collections.
        test("hello", r#""hello""#);
        test("\n", r#""\n""#);
        test("\\", r#""\\""#);
        test("\"", r#""\"""#);
        test(array![], "()");
        test(array![Value::None], "(none,)");
        test(array![1, 2], "(1, 2)");
        test(dict![], "(:)");
        test(dict!["one" => 1], "(one: 1)");
        test(dict!["two" => false, "one" => 1], "(one: 1, two: false)");

        // Functions.
        test(Function::new(None, |_, _| Ok(Value::None)), "<function>");
        test(
            Function::new(Some("nil".into()), |_, _| Ok(Value::None)),
            "<function nil>",
        );

        // Dynamics.
        test(Dynamic::new(1), "1");
    }
}
