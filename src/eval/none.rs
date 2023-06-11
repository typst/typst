use std::fmt::{self, Debug, Formatter};

use super::{cast, CastInfo, FromValue, IntoValue, Reflect, Value};
use crate::diag::StrResult;

/// A value that indicates the absence of any other value.
#[derive(Default, Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct NoneValue;

impl Reflect for NoneValue {
    fn describe() -> CastInfo {
        CastInfo::Type("none")
    }

    fn castable(value: &Value) -> bool {
        matches!(value, Value::None)
    }
}

impl IntoValue for NoneValue {
    fn into_value(self) -> Value {
        Value::None
    }
}

impl FromValue for NoneValue {
    fn from_value(value: Value) -> StrResult<Self> {
        match value {
            Value::None => Ok(Self),
            _ => Err(Self::error(&value)),
        }
    }
}

impl Debug for NoneValue {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        f.pad("none")
    }
}

cast! {
    (),
    self => Value::None,
    _: NoneValue => (),
}

impl<T: Reflect> Reflect for Option<T> {
    fn describe() -> CastInfo {
        T::describe() + NoneValue::describe()
    }

    fn castable(value: &Value) -> bool {
        NoneValue::castable(value) || T::castable(value)
    }
}

impl<T: IntoValue> IntoValue for Option<T> {
    fn into_value(self) -> Value {
        match self {
            Some(v) => v.into_value(),
            None => Value::None,
        }
    }
}

impl<T: FromValue> FromValue for Option<T> {
    fn from_value(value: Value) -> StrResult<Self> {
        match value {
            Value::None => Ok(None),
            v if T::castable(&v) => Ok(Some(T::from_value(v)?)),
            _ => Err(Self::error(&value)),
        }
    }
}
