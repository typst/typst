//! Conversion from values into other types.

use std::ops::Deref;

use fontdock::{FontStretch, FontStyle, FontWeight};

use super::{Value, ValueDict, ValueFunc};
use crate::diag::Diag;
use crate::geom::Linear;
use crate::layout::{Dir, SpecAlign};
use crate::paper::Paper;
use crate::syntax::{Ident, SpanWith, Spanned, SynTree};

/// Types that values can be converted into.
pub trait Convert: Sized {
    /// Convert a value into `Self`.
    ///
    /// If the conversion works out, this should return `Ok(...)` with an
    /// instance of `Self`. If it doesn't, it should return `Err(...)` giving
    /// back the original value.
    ///
    /// In addition to the result, the method can return an optional diagnostic
    /// to warn even when the conversion suceeded or to explain the problem when
    /// the conversion failed.
    ///
    /// The function takes a `Spanned<Value>` instead of just a `Value` so that
    /// this trait can be blanket implemented for `Spanned<T>` where `T:
    /// Convert`.
    fn convert(value: Spanned<Value>) -> (Result<Self, Value>, Option<Diag>);
}

impl<T: Convert> Convert for Spanned<T> {
    fn convert(value: Spanned<Value>) -> (Result<Self, Value>, Option<Diag>) {
        let span = value.span;
        let (result, diag) = T::convert(value);
        (result.map(|v| v.span_with(span)), diag)
    }
}

/// A value type that matches [length] values.
///
/// [length]: enum.Value.html#variant.Length
pub struct Absolute(pub f64);

impl From<Absolute> for f64 {
    fn from(abs: Absolute) -> f64 {
        abs.0
    }
}

/// A value type that matches [relative] values.
///
/// [relative]: enum.Value.html#variant.Relative
pub struct Relative(pub f64);

impl From<Relative> for f64 {
    fn from(rel: Relative) -> f64 {
        rel.0
    }
}

/// A value type that matches [identifier] and [string] values.
///
/// [identifier]: enum.Value.html#variant.Ident
/// [string]: enum.Value.html#variant.Str
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

macro_rules! impl_match {
    ($type:ty, $name:expr, $($p:pat => $r:expr),* $(,)?) => {
        impl Convert for $type {
            fn convert(value: Spanned<Value>) -> (Result<Self, Value>, Option<Diag>) {
                #[allow(unreachable_patterns)]
                match value.v {
                    $($p => (Ok($r), None)),*,
                    v => {
                        let err = error!("expected {}, found {}", $name, v.ty());
                        (Err(v), Some(err))
                    },
                }
            }
        }
    };
}

impl_match!(Value, "value", v => v);
impl_match!(Ident, "identifier", Value::Ident(v) => v);
impl_match!(bool, "bool", Value::Bool(v) => v);
impl_match!(i64, "integer", Value::Int(v) => v);
impl_match!(f64, "float",
    Value::Int(v) => v as f64,
    Value::Float(v) => v,
);
impl_match!(Absolute, "length", Value::Length(v) => Absolute(v));
impl_match!(Relative, "relative", Value::Relative(v) => Relative(v));
impl_match!(Linear, "linear",
    Value::Linear(v) => v,
    Value::Length(v) => Linear::abs(v),
    Value::Relative(v) => Linear::rel(v),
);
impl_match!(String, "string", Value::Str(v) => v);
impl_match!(SynTree, "tree", Value::Content(v) => v);
impl_match!(ValueDict, "dictionary", Value::Dict(v) => v);
impl_match!(ValueFunc, "function", Value::Func(v) => v);
impl_match!(StringLike, "identifier or string",
    Value::Ident(Ident(v)) => StringLike(v),
    Value::Str(v) => StringLike(v),
);

macro_rules! impl_ident {
    ($type:ty, $name:expr, $parse:expr) => {
        impl Convert for $type {
            fn convert(value: Spanned<Value>) -> (Result<Self, Value>, Option<Diag>) {
                match value.v {
                    Value::Ident(id) => {
                        if let Some(thing) = $parse(&id) {
                            (Ok(thing), None)
                        } else {
                            (Err(Value::Ident(id)), Some(error!("invalid {}", $name)))
                        }
                    }
                    v => {
                        let err = error!("expected {}, found {}", $name, v.ty());
                        (Err(v), Some(err))
                    }
                }
            }
        }
    };
}

impl_ident!(Dir, "direction", |v| match v {
    "ltr" => Some(Self::LTR),
    "rtl" => Some(Self::RTL),
    "ttb" => Some(Self::TTB),
    "btt" => Some(Self::BTT),
    _ => None,
});

impl_ident!(SpecAlign, "alignment", |v| match v {
    "left" => Some(Self::Left),
    "right" => Some(Self::Right),
    "top" => Some(Self::Top),
    "bottom" => Some(Self::Bottom),
    "center" => Some(Self::Center),
    _ => None,
});

impl_ident!(FontStyle, "font style", Self::from_str);
impl_ident!(FontStretch, "font stretch", Self::from_str);
impl_ident!(Paper, "paper", Self::from_name);

impl Convert for FontWeight {
    fn convert(value: Spanned<Value>) -> (Result<Self, Value>, Option<Diag>) {
        match value.v {
            Value::Int(number) => {
                let [min, max] = [100, 900];
                let warning = if number < min {
                    Some(warning!("the minimum font weight is {}", min))
                } else if number > max {
                    Some(warning!("the maximum font weight is {}", max))
                } else {
                    None
                };
                let weight = Self::from_number(number.min(max).max(min) as u16);
                (Ok(weight), warning)
            }
            Value::Ident(id) => {
                if let Some(thing) = FontWeight::from_str(&id) {
                    (Ok(thing), None)
                } else {
                    (Err(Value::Ident(id)), Some(error!("invalid font weight")))
                }
            }
            v => {
                let err =
                    error!("expected font weight (name or number), found {}", v.ty());
                (Err(v), Some(err))
            }
        }
    }
}
