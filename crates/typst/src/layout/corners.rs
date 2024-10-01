use std::fmt::{self, Debug, Formatter};

use crate::diag::HintedStrResult;
use crate::foundations::{
    AlternativeFold, CastInfo, Dict, Fold, FromValue, IntoValue, Reflect, Resolve,
    StyleChain, Value,
};
use crate::layout::Side;
use crate::utils::Get;

/// A container with components for the four corners of a rectangle.
#[derive(Default, Copy, Clone, Eq, PartialEq, Hash)]
pub struct Corners<T> {
    /// The value for the top left corner.
    pub top_left: T,
    /// The value for the top right corner.
    pub top_right: T,
    /// The value for the bottom right corner.
    pub bottom_right: T,
    /// The value for the bottom left corner.
    pub bottom_left: T,
}

impl<T> Corners<T> {
    /// Create a new instance from the four components.
    pub const fn new(top_left: T, top_right: T, bottom_right: T, bottom_left: T) -> Self {
        Self { top_left, top_right, bottom_right, bottom_left }
    }

    /// Create an instance with four equal components.
    pub fn splat(value: T) -> Self
    where
        T: Clone,
    {
        Self {
            top_left: value.clone(),
            top_right: value.clone(),
            bottom_right: value.clone(),
            bottom_left: value,
        }
    }

    /// Map the individual fields with `f`.
    pub fn map<F, U>(self, mut f: F) -> Corners<U>
    where
        F: FnMut(T) -> U,
    {
        Corners {
            top_left: f(self.top_left),
            top_right: f(self.top_right),
            bottom_right: f(self.bottom_right),
            bottom_left: f(self.bottom_left),
        }
    }

    /// Zip two instances into one.
    pub fn zip<U>(self, other: Corners<U>) -> Corners<(T, U)> {
        Corners {
            top_left: (self.top_left, other.top_left),
            top_right: (self.top_right, other.top_right),
            bottom_right: (self.bottom_right, other.bottom_right),
            bottom_left: (self.bottom_left, other.bottom_left),
        }
    }

    /// An iterator over the corners, starting with the top left corner,
    /// clockwise.
    pub fn iter(&self) -> impl Iterator<Item = &T> {
        [&self.top_left, &self.top_right, &self.bottom_right, &self.bottom_left]
            .into_iter()
    }

    /// Whether all sides are equal.
    pub fn is_uniform(&self) -> bool
    where
        T: PartialEq,
    {
        self.top_left == self.top_right
            && self.top_right == self.bottom_right
            && self.bottom_right == self.bottom_left
    }
}

impl<T> Corners<Option<T>> {
    /// Unwrap-or-default the individual corners.
    pub fn unwrap_or_default(self) -> Corners<T>
    where
        T: Default,
    {
        self.map(Option::unwrap_or_default)
    }
}

impl<T> Get<Corner> for Corners<T> {
    type Component = T;

    fn get_ref(&self, corner: Corner) -> &T {
        match corner {
            Corner::TopLeft => &self.top_left,
            Corner::TopRight => &self.top_right,
            Corner::BottomRight => &self.bottom_right,
            Corner::BottomLeft => &self.bottom_left,
        }
    }

    fn get_mut(&mut self, corner: Corner) -> &mut T {
        match corner {
            Corner::TopLeft => &mut self.top_left,
            Corner::TopRight => &mut self.top_right,
            Corner::BottomRight => &mut self.bottom_right,
            Corner::BottomLeft => &mut self.bottom_left,
        }
    }
}

impl<T: Debug + PartialEq> Debug for Corners<T> {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        if self.is_uniform() {
            f.write_str("Corners::splat(")?;
            self.top_left.fmt(f)?;
            f.write_str(")")
        } else {
            f.debug_struct("Corners")
                .field("top_left", &self.top_left)
                .field("top_right", &self.top_right)
                .field("bottom_right", &self.bottom_right)
                .field("bottom_left", &self.bottom_left)
                .finish()
        }
    }
}

impl<T: Reflect> Reflect for Corners<Option<T>> {
    fn input() -> CastInfo {
        T::input() + Dict::input()
    }

    fn output() -> CastInfo {
        T::output() + Dict::output()
    }

    fn castable(value: &Value) -> bool {
        Dict::castable(value) || T::castable(value)
    }
}

impl<T> IntoValue for Corners<Option<T>>
where
    T: PartialEq + IntoValue,
{
    fn into_value(self) -> Value {
        if self.is_uniform() {
            if let Some(top_left) = self.top_left {
                return top_left.into_value();
            }
        }

        let mut dict = Dict::new();
        let mut handle = |key: &str, component: Option<T>| {
            if let Some(c) = component {
                dict.insert(key.into(), c.into_value());
            }
        };

        handle("top-left", self.top_left);
        handle("top-right", self.top_right);
        handle("bottom-right", self.bottom_right);
        handle("bottom-left", self.bottom_left);

        Value::Dict(dict)
    }
}

impl<T> FromValue for Corners<Option<T>>
where
    T: FromValue + Clone,
{
    fn from_value(mut value: Value) -> HintedStrResult<Self> {
        let expected_keys = [
            "top-left",
            "top-right",
            "bottom-right",
            "bottom-left",
            "left",
            "top",
            "right",
            "bottom",
            "rest",
        ];

        if let Value::Dict(dict) = &mut value {
            if dict.is_empty() {
                return Ok(Self::splat(None));
            } else if dict.iter().any(|(key, _)| expected_keys.contains(&key.as_str())) {
                let mut take = |key| dict.take(key).ok().map(T::from_value).transpose();
                let rest = take("rest")?;
                let left = take("left")?.or_else(|| rest.clone());
                let top = take("top")?.or_else(|| rest.clone());
                let right = take("right")?.or_else(|| rest.clone());
                let bottom = take("bottom")?.or_else(|| rest.clone());
                let corners = Corners {
                    top_left: take("top-left")?
                        .or_else(|| top.clone())
                        .or_else(|| left.clone()),
                    top_right: take("top-right")?
                        .or_else(|| top.clone())
                        .or_else(|| right.clone()),
                    bottom_right: take("bottom-right")?
                        .or_else(|| bottom.clone())
                        .or_else(|| right.clone()),
                    bottom_left: take("bottom-left")?
                        .or_else(|| bottom.clone())
                        .or_else(|| left.clone()),
                };

                dict.finish(&expected_keys)?;
                return Ok(corners);
            }
        }

        if T::castable(&value) {
            Ok(Self::splat(Some(T::from_value(value)?)))
        } else if let Value::Dict(dict) = &value {
            let keys = dict.iter().map(|kv| kv.0.as_str()).collect();
            // Do not hint at expected_keys, because T may be castable from Dict
            // objects with other sets of expected keys.
            Err(Dict::unexpected_keys(keys, None).into())
        } else {
            Err(Self::error(&value))
        }
    }
}

impl<T: Resolve> Resolve for Corners<T> {
    type Output = Corners<T::Output>;

    fn resolve(self, styles: StyleChain) -> Self::Output {
        self.map(|v| v.resolve(styles))
    }
}

impl<T: Fold> Fold for Corners<Option<T>> {
    fn fold(self, outer: Self) -> Self {
        // Usually, folding an inner `None` with an `outer` prefers the
        // explicit `None`. However, here `None` means unspecified and thus
        // we want `outer`, so we use `fold_or` to opt into such behavior.
        self.zip(outer).map(|(inner, outer)| inner.fold_or(outer))
    }
}

/// The four corners of a rectangle.
#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
pub enum Corner {
    /// The top left corner.
    TopLeft,
    /// The top right corner.
    TopRight,
    /// The bottom right corner.
    BottomRight,
    /// The bottom left corner.
    BottomLeft,
}

impl Corner {
    /// The opposite corner.
    pub fn inv(self) -> Self {
        match self {
            Self::TopLeft => Self::BottomRight,
            Self::TopRight => Self::BottomLeft,
            Self::BottomRight => Self::TopLeft,
            Self::BottomLeft => Self::TopRight,
        }
    }

    /// The next corner, clockwise.
    pub fn next_cw(self) -> Self {
        match self {
            Self::TopLeft => Self::TopRight,
            Self::TopRight => Self::BottomRight,
            Self::BottomRight => Self::BottomLeft,
            Self::BottomLeft => Self::TopLeft,
        }
    }

    /// The next corner, counter-clockwise.
    pub fn next_ccw(self) -> Self {
        match self {
            Self::TopLeft => Self::BottomLeft,
            Self::TopRight => Self::TopLeft,
            Self::BottomRight => Self::TopRight,
            Self::BottomLeft => Self::BottomRight,
        }
    }

    /// The next side, clockwise.
    pub fn side_cw(self) -> Side {
        match self {
            Self::TopLeft => Side::Top,
            Self::TopRight => Side::Right,
            Self::BottomRight => Side::Bottom,
            Self::BottomLeft => Side::Left,
        }
    }

    /// The next side, counter-clockwise.
    pub fn side_ccw(self) -> Side {
        match self {
            Self::TopLeft => Side::Left,
            Self::TopRight => Side::Top,
            Self::BottomRight => Side::Right,
            Self::BottomLeft => Side::Bottom,
        }
    }
}
