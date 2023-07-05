use std::fmt::{self, Debug, Formatter};

use super::{CastInfo, FromValue, IntoValue, Reflect, Value};
use crate::diag::StrResult;

/// A value that indicates a smart default.
#[derive(Default, Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
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
    fn describe() -> CastInfo {
        CastInfo::Type("auto")
    }

    fn castable(value: &Value) -> bool {
        matches!(value, Value::Auto)
    }
}

impl Debug for AutoValue {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        f.pad("auto")
    }
}
