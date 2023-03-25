pub use typst_macros::{cast_from_value, cast_to_value, Cast};

use std::num::{NonZeroI64, NonZeroUsize};
use std::ops::Add;

use ecow::EcoString;

use super::{Array, Str, Value};
use crate::diag::StrResult;
use crate::syntax::Spanned;
use crate::util::separated_list;

/// Cast from a value to a specific type.
pub trait Cast<V = Value>: Sized {
    /// Check whether the value is castable to `Self`.
    fn is(value: &V) -> bool;

    /// Try to cast the value into an instance of `Self`.
    fn cast(value: V) -> StrResult<Self>;

    /// Describe the acceptable values.
    fn describe() -> CastInfo;

    /// Produce an error for an inacceptable value.
    fn error(value: Value) -> StrResult<Self> {
        Err(Self::describe().error(&value))
    }
}

impl Cast for Value {
    fn is(_: &Value) -> bool {
        true
    }

    fn cast(value: Value) -> StrResult<Self> {
        Ok(value)
    }

    fn describe() -> CastInfo {
        CastInfo::Any
    }
}

impl<T: Cast> Cast<Spanned<Value>> for T {
    fn is(value: &Spanned<Value>) -> bool {
        T::is(&value.v)
    }

    fn cast(value: Spanned<Value>) -> StrResult<Self> {
        T::cast(value.v)
    }

    fn describe() -> CastInfo {
        T::describe()
    }
}

impl<T: Cast> Cast<Spanned<Value>> for Spanned<T> {
    fn is(value: &Spanned<Value>) -> bool {
        T::is(&value.v)
    }

    fn cast(value: Spanned<Value>) -> StrResult<Self> {
        let span = value.span;
        T::cast(value.v).map(|t| Spanned::new(t, span))
    }

    fn describe() -> CastInfo {
        T::describe()
    }
}

cast_to_value! {
    v: u8 => Value::Int(i64::from(v))
}

cast_to_value! {
    v: u16 => Value::Int(i64::from(v))
}

cast_from_value! {
    u32,
    int: i64 => int.try_into().map_err(|_| {
        if int < 0 {
            "number must be at least zero"
        } else {
            "number too large"
        }
    })?,
}

cast_to_value! {
    v: u32 => Value::Int(i64::from(v))
}

cast_to_value! {
    v: i32 => Value::Int(i64::from(v))
}

cast_from_value! {
    usize,
    int: i64 => int.try_into().map_err(|_| {
        if int < 0 {
            "number must be at least zero"
        } else {
            "number too large"
        }
    })?,
}

cast_to_value! {
    v: usize => Value::Int(v as i64)
}

cast_from_value! {
    NonZeroUsize,
    int: i64 => int
        .try_into()
        .and_then(usize::try_into)
        .map_err(|_| if int <= 0 {
            "number must be positive"
        } else {
            "number too large"
        })?,
}

cast_to_value! {
    v: NonZeroUsize => Value::Int(v.get() as i64)
}

cast_from_value! {
    NonZeroI64,
    int: i64 => int.try_into()
        .map_err(|_| if int <= 0 {
            "number must be positive"
        } else {
            "number too large"
        })?,
}

cast_to_value! {
    v: NonZeroI64 => Value::Int(v.get())
}

cast_from_value! {
    char,
    string: Str => {
        let mut chars = string.chars();
        match (chars.next(), chars.next()) {
            (Some(c), None) => c,
            _ => Err("expected exactly one character")?,
        }
    },
}

cast_to_value! {
    v: char => Value::Str(v.into())
}

cast_to_value! {
    v: &str => Value::Str(v.into())
}

cast_from_value! {
    EcoString,
    v: Str => v.into(),
}

cast_to_value! {
    v: EcoString => Value::Str(v.into())
}

cast_from_value! {
    String,
    v: Str => v.into(),
}

cast_to_value! {
    v: String => Value::Str(v.into())
}

impl<T: Cast> Cast for Option<T> {
    fn is(value: &Value) -> bool {
        matches!(value, Value::None) || T::is(value)
    }

    fn cast(value: Value) -> StrResult<Self> {
        match value {
            Value::None => Ok(None),
            v if T::is(&v) => Ok(Some(T::cast(v)?)),
            _ => <Self as Cast>::error(value),
        }
    }

    fn describe() -> CastInfo {
        T::describe() + CastInfo::Type("none")
    }
}

impl<T: Into<Value>> From<Option<T>> for Value {
    fn from(v: Option<T>) -> Self {
        match v {
            Some(v) => v.into(),
            None => Value::None,
        }
    }
}

impl<T: Cast> Cast for Vec<T> {
    fn is(value: &Value) -> bool {
        Array::is(value)
    }

    fn cast(value: Value) -> StrResult<Self> {
        value.cast::<Array>()?.into_iter().map(Value::cast).collect()
    }

    fn describe() -> CastInfo {
        <Array as Cast>::describe()
    }
}

impl<T: Into<Value>> From<Vec<T>> for Value {
    fn from(v: Vec<T>) -> Self {
        Value::Array(v.into_iter().map(Into::into).collect())
    }
}

/// A container for a variadic argument.
pub trait Variadics {
    /// The contained type.
    type Inner;
}

impl<T> Variadics for Vec<T> {
    type Inner = T;
}

/// Describes a possible value for a cast.
#[derive(Debug, Clone, Hash)]
pub enum CastInfo {
    /// Any value is okay.
    Any,
    /// A specific value, plus short documentation for that value.
    Value(Value, &'static str),
    /// Any value of a type.
    Type(&'static str),
    /// Multiple alternatives.
    Union(Vec<Self>),
}

impl CastInfo {
    /// Produce an error message describing what was expected and what was
    /// found.
    pub fn error(&self, found: &Value) -> EcoString {
        fn accumulate(
            info: &CastInfo,
            found: &Value,
            parts: &mut Vec<EcoString>,
            matching_type: &mut bool,
        ) {
            match info {
                CastInfo::Any => parts.push("anything".into()),
                CastInfo::Value(value, _) => {
                    parts.push(value.repr().into());
                    if value.type_name() == found.type_name() {
                        *matching_type = true;
                    }
                }
                CastInfo::Type(ty) => parts.push((*ty).into()),
                CastInfo::Union(options) => {
                    for option in options {
                        accumulate(option, found, parts, matching_type);
                    }
                }
            }
        }

        let mut matching_type = false;
        let mut parts = vec![];
        accumulate(self, found, &mut parts, &mut matching_type);

        let mut msg = String::from("expected ");
        if parts.is_empty() {
            msg.push_str(" nothing");
        }

        msg.push_str(&separated_list(&parts, "or"));

        if !matching_type {
            msg.push_str(", found ");
            msg.push_str(found.type_name());
        }

        msg.into()
    }
}

impl Add for CastInfo {
    type Output = Self;

    fn add(self, rhs: Self) -> Self {
        Self::Union(match (self, rhs) {
            (Self::Union(mut lhs), Self::Union(rhs)) => {
                lhs.extend(rhs);
                lhs
            }
            (Self::Union(mut lhs), rhs) => {
                lhs.push(rhs);
                lhs
            }
            (lhs, Self::Union(mut rhs)) => {
                rhs.insert(0, lhs);
                rhs
            }
            (lhs, rhs) => vec![lhs, rhs],
        })
    }
}

/// Castable from nothing.
pub enum Never {}

impl Cast for Never {
    fn is(_: &Value) -> bool {
        false
    }

    fn cast(value: Value) -> StrResult<Self> {
        <Self as Cast>::error(value)
    }

    fn describe() -> CastInfo {
        CastInfo::Union(vec![])
    }
}
