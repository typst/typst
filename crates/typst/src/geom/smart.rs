use super::*;
use crate::eval::{AutoValue, CastInfo, FromValue, IntoValue, Reflect};

/// A value that can be automatically determined.
#[derive(Debug, Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub enum Smart<T> {
    /// The value should be determined smartly based on the circumstances.
    Auto,
    /// A specific value.
    Custom(T),
}

impl<T> Smart<T> {
    /// Whether the value is `Auto`.
    pub fn is_auto(&self) -> bool {
        matches!(self, Self::Auto)
    }

    /// Whether this holds a custom value.
    pub fn is_custom(&self) -> bool {
        matches!(self, Self::Custom(_))
    }

    /// Returns a `Smart<&T>` borrowing the inner `T`.
    pub fn as_ref(&self) -> Smart<&T> {
        match self {
            Smart::Auto => Smart::Auto,
            Smart::Custom(v) => Smart::Custom(v),
        }
    }

    /// Returns a reference the contained custom value.
    /// If the value is [`Smart::Auto`], `None` is returned.
    pub fn as_custom(self) -> Option<T> {
        match self {
            Self::Auto => None,
            Self::Custom(x) => Some(x),
        }
    }

    /// Map the contained custom value with `f`.
    pub fn map<F, U>(self, f: F) -> Smart<U>
    where
        F: FnOnce(T) -> U,
    {
        match self {
            Self::Auto => Smart::Auto,
            Self::Custom(x) => Smart::Custom(f(x)),
        }
    }

    /// Map the contained custom value with `f` if it contains a custom value,
    /// otherwise returns `default`.
    pub fn map_or<F, U>(self, default: U, f: F) -> U
    where
        F: FnOnce(T) -> U,
    {
        match self {
            Self::Auto => default,
            Self::Custom(x) => f(x),
        }
    }

    /// Keeps `self` if it contains a custom value, otherwise returns `other`.
    pub fn or(self, other: Smart<T>) -> Self {
        match self {
            Self::Custom(x) => Self::Custom(x),
            Self::Auto => other,
        }
    }

    /// Retusn `Auto` if `self` is `Auto`, otherwise calls the provided function onthe contained
    /// value and returns the result.
    pub fn and_then<F, U>(self, f: F) -> Smart<U>
    where
        F: FnOnce(T) -> Smart<U>,
    {
        match self {
            Smart::Auto => Smart::Auto,
            Smart::Custom(x) => f(x),
        }
    }

    /// Returns the contained custom value or a provided default value.
    pub fn unwrap_or(self, default: T) -> T {
        match self {
            Self::Auto => default,
            Self::Custom(x) => x,
        }
    }

    /// Returns the contained custom value or computes a default value.
    pub fn unwrap_or_else<F>(self, f: F) -> T
    where
        F: FnOnce() -> T,
    {
        match self {
            Self::Auto => f(),
            Self::Custom(x) => x,
        }
    }

    /// Returns the contained custom value or the default value.
    pub fn unwrap_or_default(self) -> T
    where
        T: Default,
    {
        // we want to do this; the Clippy lint is not type-aware
        #[allow(clippy::unwrap_or_default)]
        self.unwrap_or_else(T::default)
    }
}

impl<T> Smart<Smart<T>> {
    /// Removes a single level of nesting, returns `Auto` if the inner or outer value is `Auto`.
    pub fn flatten(self) -> Smart<T> {
        match self {
            Smart::Custom(Smart::Auto) | Smart::Auto => Smart::Auto,
            Smart::Custom(Smart::Custom(v)) => Smart::Custom(v),
        }
    }
}

impl<T> Default for Smart<T> {
    fn default() -> Self {
        Self::Auto
    }
}

impl<T: Reflect> Reflect for Smart<T> {
    fn input() -> CastInfo {
        T::input() + AutoValue::input()
    }

    fn output() -> CastInfo {
        T::output() + AutoValue::output()
    }

    fn castable(value: &Value) -> bool {
        AutoValue::castable(value) || T::castable(value)
    }
}

impl<T: IntoValue> IntoValue for Smart<T> {
    fn into_value(self) -> Value {
        match self {
            Smart::Custom(v) => v.into_value(),
            Smart::Auto => Value::Auto,
        }
    }
}

impl<T: FromValue> FromValue for Smart<T> {
    fn from_value(value: Value) -> StrResult<Self> {
        match value {
            Value::Auto => Ok(Self::Auto),
            v if T::castable(&v) => Ok(Self::Custom(T::from_value(v)?)),
            _ => Err(Self::error(&value)),
        }
    }
}

impl<T: Resolve> Resolve for Smart<T> {
    type Output = Smart<T::Output>;

    fn resolve(self, styles: StyleChain) -> Self::Output {
        self.map(|v| v.resolve(styles))
    }
}

impl<T> Fold for Smart<T>
where
    T: Fold,
    T::Output: Default,
{
    type Output = Smart<T::Output>;

    fn fold(self, outer: Self::Output) -> Self::Output {
        self.map(|inner| inner.fold(outer.unwrap_or_default()))
    }
}
