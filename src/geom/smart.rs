use super::*;

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
        self.unwrap_or_else(T::default)
    }
}

impl<T> Default for Smart<T> {
    fn default() -> Self {
        Self::Auto
    }
}

impl<T: Cast> Cast for Smart<T> {
    fn is(value: &Value) -> bool {
        matches!(value, Value::Auto) || T::is(value)
    }

    fn cast(value: Value) -> StrResult<Self> {
        match value {
            Value::Auto => Ok(Self::Auto),
            v if T::is(&v) => Ok(Self::Custom(T::cast(v)?)),
            _ => <Self as Cast>::error(value),
        }
    }

    fn describe() -> CastInfo {
        T::describe() + CastInfo::Type("auto")
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

impl<T: Into<Value>> From<Smart<T>> for Value {
    fn from(v: Smart<T>) -> Self {
        match v {
            Smart::Custom(v) => v.into(),
            Smart::Auto => Value::Auto,
        }
    }
}
