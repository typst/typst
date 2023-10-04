use ecow::EcoString;
use std::fmt::Debug;

use super::{ty, CastInfo, FromValue, IntoValue, Reflect, Repr, Type, Value};
use crate::diag::StrResult;

/// A value that indicates a smart default.
///
/// The auto type has exactly one value: `{auto}`.
///
/// Parameters that support the `{auto}` value have some smart default or
/// contextual behaviour. A good example is the [text direction]($text.dir)
/// parameter. Setting it to `{auto}` lets Typst automatically determine the
/// direction from the [text language]($text.lang).
#[ty(name = "auto")]
#[derive(Debug, Default, Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct AutoValue;

impl IntoValue for AutoValue {
    fn into_value(self) -> Value {
        Value::Auto
    }
}

impl FromValue for AutoValue {
    fn from_value(value: Value) -> StrResult<Self> {
        match value {
            Value::Auto => Ok(Self),
            _ => Err(Self::error(&value)),
        }
    }
}

impl Reflect for AutoValue {
    fn input() -> CastInfo {
        CastInfo::Type(Type::of::<Self>())
    }

    fn output() -> CastInfo {
        CastInfo::Type(Type::of::<Self>())
    }

    fn castable(value: &Value) -> bool {
        matches!(value, Value::Auto)
    }
}

impl Repr for AutoValue {
    fn repr(&self) -> EcoString {
        "auto".into()
    }
}
