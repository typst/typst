use ecow::EcoString;
use std::fmt::{self, Debug, Formatter};

use crate::diag::HintedStrResult;
use crate::foundations::{
    ty, CastInfo, Fold, FromValue, IntoValue, Reflect, Repr, Resolve, StyleChain, Type,
    Value,
};

/// A value that indicates a smart default.
///
/// The auto type has exactly one value: `{auto}`.
///
/// Parameters that support the `{auto}` value have some smart default or
/// contextual behaviour. A good example is the [text direction]($text.dir)
/// parameter. Setting it to `{auto}` lets Typst automatically determine the
/// direction from the [text language]($text.lang).
#[ty(cast, name = "auto")]
#[derive(Default, Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct AutoValue;

impl IntoValue for AutoValue {
    fn into_value(self) -> Value {
        Value::Auto
    }
}

impl FromValue for AutoValue {
    fn from_value(value: Value) -> HintedStrResult<Self> {
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

impl Debug for AutoValue {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        f.write_str("Auto")
    }
}

impl Repr for AutoValue {
    fn repr(&self) -> EcoString {
        "auto".into()
    }
}

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

    /// Whether this is a `Smart::Custom(x)` and `f(x)` is true.
    pub fn is_custom_and<F>(self, f: F) -> bool
    where
        F: Fn(T) -> bool,
    {
        match self {
            Self::Auto => false,
            Self::Custom(x) => f(x),
        }
    }

    /// Returns a `Smart<&T>` borrowing the inner `T`.
    pub fn as_ref(&self) -> Smart<&T> {
        match self {
            Smart::Auto => Smart::Auto,
            Smart::Custom(v) => Smart::Custom(v),
        }
    }

    /// Returns the contained custom value.
    ///
    /// If the value is [`Smart::Auto`], returns `None`.
    ///
    /// Equivalently, this just converts `Smart` to `Option`.
    pub fn custom(self) -> Option<T> {
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

    /// Keeps `self` if it contains a custom value, otherwise returns the
    /// output of the given function.
    pub fn or_else<F>(self, f: F) -> Self
    where
        F: FnOnce() -> Self,
    {
        match self {
            Self::Custom(x) => Self::Custom(x),
            Self::Auto => f(),
        }
    }

    /// Returns `Auto` if `self` is `Auto`, otherwise calls the provided
    /// function on the contained value and returns the result.
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
    fn from_value(value: Value) -> HintedStrResult<Self> {
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

impl<T: Fold> Fold for Smart<T> {
    fn fold(self, outer: Self) -> Self {
        use Smart::Custom;
        match (self, outer) {
            (Custom(inner), Custom(outer)) => Custom(inner.fold(outer)),
            // An explicit `auto` should be respected, thus we don't do
            // `inner.or(outer)`.
            (inner, _) => inner,
        }
    }
}
