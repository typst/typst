use crate::eval::{CastInfo, FromValue, IntoValue, Reflect};

use super::*;

/// A container with first, middle, last and single components.
#[derive(Debug, Default, Copy, Clone, Eq, PartialEq, Hash)]
pub struct BrokenParts<T> {
    /// The value for the first of multiple parts.
    pub first: T,
    /// The value for the parts that are neither first nor last.
    pub middle: T,
    /// The value for the last of multiple parts.
    pub last: T,
    /// The value for a singular part.
    pub single: T,
}

impl<T> BrokenParts<T> {
    /// Create a new instance from the four components.
    pub const fn new(first: T, middle: T, last: T, single: T) -> Self {
        Self { first, middle, last, single }
    }

    /// Create an instance with four equal components.
    pub fn splat(value: T) -> Self
    where
        T: Clone,
    {
        Self {
            first: value.clone(),
            middle: value.clone(),
            last: value.clone(),
            single: value,
        }
    }

    /// Map the individual fields with `f`.
    pub fn map<F, U>(self, mut f: F) -> BrokenParts<U>
    where
        F: FnMut(T) -> U,
    {
        BrokenParts {
            first: f(self.first),
            middle: f(self.middle),
            last: f(self.last),
            single: f(self.single),
        }
    }

    /// Zip two instances into one.
    pub fn zip<U>(self, other: BrokenParts<U>) -> BrokenParts<(T, U)> {
        BrokenParts {
            first: (self.first, other.first),
            middle: (self.middle, other.middle),
            last: (self.last, other.last),
            single: (self.single, other.single),
        }
    }

    /// An iterator over the values for multiple parts.
    pub fn iter_multiple(&self) -> impl Iterator<Item = &T> {
        [&self.first, &self.middle, &self.last].into_iter()
    }

    /// Whether all parts are equal.
    pub fn is_uniform(&self) -> bool
    where
        T: PartialEq,
    {
        self.first == self.middle && self.middle == self.last && self.last == self.single
    }
}

impl<T> Get<BrokenPart> for BrokenParts<T> {
    type Component = T;

    fn get_ref(&self, part: BrokenPart) -> &T {
        match part {
            BrokenPart::First => &self.first,
            BrokenPart::Middle => &self.middle,
            BrokenPart::Last => &self.last,
            BrokenPart::Single => &self.single,
        }
    }

    fn get_mut(&mut self, side: BrokenPart) -> &mut T {
        match side {
            BrokenPart::First => &mut self.first,
            BrokenPart::Middle => &mut self.middle,
            BrokenPart::Last => &mut self.last,
            BrokenPart::Single => &mut self.single,
        }
    }
}

/// The four kinds of parts that can arise from breaking.
#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
pub enum BrokenPart {
    /// The first of multiple parts.
    First,
    /// The parts that are neither first nor last.
    Middle,
    /// The last of multiple parts.
    Last,
    /// A singular part.
    Single,
}

impl<T: Reflect> Reflect for BrokenParts<T> {
    fn describe() -> CastInfo {
        T::describe() + Dict::describe()
    }

    fn castable(value: &Value) -> bool {
        Dict::castable(value) || T::castable(value)
    }
}

impl<T> IntoValue for BrokenParts<T>
where
    T: PartialEq + IntoValue,
{
    fn into_value(self) -> Value {
        if self.is_uniform() {
            return self.first.into_value();
        }

        let mut dict = Dict::new();
        let mut handle = |key: &str, component: T| {
            let value = component.into_value();
            if value != Value::None {
                dict.insert(key.into(), value);
            }
        };

        handle("first", self.first);
        handle("middle", self.middle);
        handle("last", self.last);
        handle("single", self.single);

        Value::Dict(dict)
    }
}

impl<T> FromValue for BrokenParts<T>
where
    T: Default + FromValue + Clone,
{
    fn from_value(mut value: Value) -> StrResult<Self> {
        let keys = ["first", "middle", "last", "single", "rest"];
        if let Value::Dict(dict) = &mut value {
            if dict.iter().any(|(key, _)| keys.contains(&key.as_str())) {
                let mut take = |key| dict.take(key).ok().map(T::from_value).transpose();
                let rest = take("rest")?;

                // Make sure that either `rest` or all other keys have to be given.
                let mut take_or_rest = |key| match dict.take(key) {
                    Ok(val) => T::from_value(val),
                    Err(e) => rest.clone().ok_or(e),
                };

                let parts = BrokenParts {
                    first: take_or_rest("first")?,
                    middle: take_or_rest("middle")?,
                    last: take_or_rest("last")?,
                    single: take_or_rest("single")?,
                };

                dict.finish(&keys)?;
                return Ok(parts);
            }
        }

        if T::castable(&value) {
            Ok(Self::splat(T::from_value(value)?))
        } else {
            Err(Self::error(&value))
        }
    }
}

impl<T: Resolve> Resolve for BrokenParts<T> {
    type Output = BrokenParts<T::Output>;

    fn resolve(self, styles: StyleChain) -> Self::Output {
        self.map(|v| v.resolve(styles))
    }
}

impl<T: Fold> Fold for BrokenParts<T> {
    type Output = BrokenParts<T::Output>;

    fn fold(self, outer: Self::Output) -> Self::Output {
        self.zip(outer).map(|(inner, outer)| inner.fold(outer))
    }
}
