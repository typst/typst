use std::fmt::{self, Debug, Formatter};
use std::ops::Add;

use typst_utils::Get;

use crate::diag::{HintedStrResult, bail};
use crate::foundations::{
    AlternativeFold, CastInfo, Dict, Fold, FromValue, IntoValue, Reflect, Resolve,
    StyleChain, Value, cast,
};
use crate::layout::{Abs, Alignment, Axes, Axis, Corner, Rel, Size};

/// A container with left, top, right and bottom components.
#[derive(Default, Copy, Clone, Eq, PartialEq, Hash)]
pub struct Sides<T> {
    /// The value for the left side.
    pub left: T,
    /// The value for the top side.
    pub top: T,
    /// The value for the right side.
    pub right: T,
    /// The value for the bottom side.
    pub bottom: T,
}

impl<T> Sides<T> {
    /// Create a new instance from the four components.
    pub const fn new(left: T, top: T, right: T, bottom: T) -> Self {
        Self { left, top, right, bottom }
    }

    /// Create an instance with four equal components.
    pub fn splat(value: T) -> Self
    where
        T: Clone,
    {
        Self {
            left: value.clone(),
            top: value.clone(),
            right: value.clone(),
            bottom: value,
        }
    }

    /// Map the individual fields with `f`.
    pub fn map<F, U>(self, mut f: F) -> Sides<U>
    where
        F: FnMut(T) -> U,
    {
        Sides {
            left: f(self.left),
            top: f(self.top),
            right: f(self.right),
            bottom: f(self.bottom),
        }
    }

    /// Convert from `&Sides<T>` to `Sides<&T>`.
    pub fn as_ref(&self) -> Sides<&T> {
        Sides {
            left: &self.left,
            top: &self.top,
            right: &self.right,
            bottom: &self.bottom,
        }
    }

    /// Zip two instances into one.
    pub fn zip<U>(self, other: Sides<U>) -> Sides<(T, U)> {
        Sides {
            left: (self.left, other.left),
            top: (self.top, other.top),
            right: (self.right, other.right),
            bottom: (self.bottom, other.bottom),
        }
    }

    /// An iterator over the sides, starting with the left side, clockwise.
    pub fn iter(&self) -> impl Iterator<Item = &T> {
        [&self.left, &self.top, &self.right, &self.bottom].into_iter()
    }

    /// Whether all sides are equal.
    pub fn is_uniform(&self) -> bool
    where
        T: PartialEq,
    {
        self.left == self.top && self.top == self.right && self.right == self.bottom
    }
}

impl<T: Add> Sides<T> {
    /// Sums up `left` and `right` into `x`, and `top` and `bottom` into `y`.
    pub fn sum_by_axis(self) -> Axes<T::Output> {
        Axes::new(self.left + self.right, self.top + self.bottom)
    }
}

impl<T> Sides<Option<T>> {
    /// Unwrap-or the individual sides.
    pub fn unwrap_or(self, default: T) -> Sides<T>
    where
        T: Clone,
    {
        self.map(|v| v.unwrap_or(default.clone()))
    }

    /// Unwrap-or-default the individual sides.
    pub fn unwrap_or_default(self) -> Sides<T>
    where
        T: Default,
    {
        self.map(Option::unwrap_or_default)
    }
}

impl Sides<Rel<Abs>> {
    /// Evaluate the sides relative to the given `size`.
    pub fn relative_to(&self, size: Size) -> Sides<Abs> {
        Sides {
            left: self.left.relative_to(size.x),
            top: self.top.relative_to(size.y),
            right: self.right.relative_to(size.x),
            bottom: self.bottom.relative_to(size.y),
        }
    }

    /// Whether all sides are zero.
    pub fn is_zero(&self) -> bool {
        self.left.is_zero()
            && self.top.is_zero()
            && self.right.is_zero()
            && self.bottom.is_zero()
    }
}

impl<T> Get<Side> for Sides<T> {
    type Component = T;

    fn get_ref(&self, side: Side) -> &T {
        match side {
            Side::Left => &self.left,
            Side::Top => &self.top,
            Side::Right => &self.right,
            Side::Bottom => &self.bottom,
        }
    }

    fn get_mut(&mut self, side: Side) -> &mut T {
        match side {
            Side::Left => &mut self.left,
            Side::Top => &mut self.top,
            Side::Right => &mut self.right,
            Side::Bottom => &mut self.bottom,
        }
    }
}

impl<T: Debug + PartialEq> Debug for Sides<T> {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        if self.is_uniform() {
            f.write_str("Sides::splat(")?;
            self.left.fmt(f)?;
            f.write_str(")")
        } else {
            f.debug_struct("Sides")
                .field("left", &self.left)
                .field("top", &self.top)
                .field("right", &self.right)
                .field("bottom", &self.bottom)
                .finish()
        }
    }
}

impl<T: Reflect> Reflect for Sides<Option<T>> {
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

impl<T> IntoValue for Sides<Option<T>>
where
    T: PartialEq + IntoValue,
{
    fn into_value(self) -> Value {
        if self.is_uniform()
            && let Some(left) = self.left
        {
            return left.into_value();
        }

        let mut dict = Dict::new();
        let mut handle = |key: &str, component: Option<T>| {
            if let Some(c) = component {
                dict.insert(key.into(), c.into_value());
            }
        };

        handle("left", self.left);
        handle("top", self.top);
        handle("right", self.right);
        handle("bottom", self.bottom);

        Value::Dict(dict)
    }
}

impl<T> FromValue for Sides<Option<T>>
where
    T: Default + FromValue + Clone,
{
    fn from_value(mut value: Value) -> HintedStrResult<Self> {
        let expected_keys = ["left", "top", "right", "bottom", "x", "y", "rest"];
        if let Value::Dict(dict) = &mut value {
            if dict.is_empty() {
                return Ok(Self::splat(None));
            } else if dict.iter().any(|(key, _)| expected_keys.contains(&key.as_str())) {
                let mut take = |key| dict.take(key).ok().map(T::from_value).transpose();
                let rest = take("rest")?;
                let x = take("x")?.or_else(|| rest.clone());
                let y = take("y")?.or_else(|| rest.clone());
                let sides = Sides {
                    left: take("left")?.or_else(|| x.clone()),
                    top: take("top")?.or_else(|| y.clone()),
                    right: take("right")?.or_else(|| x.clone()),
                    bottom: take("bottom")?.or_else(|| y.clone()),
                };

                dict.finish(&expected_keys)?;
                return Ok(sides);
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

impl<T: Resolve> Resolve for Sides<T> {
    type Output = Sides<T::Output>;

    fn resolve(self, styles: StyleChain) -> Self::Output {
        self.map(|v| v.resolve(styles))
    }
}

impl<T: Fold> Fold for Sides<Option<T>> {
    fn fold(self, outer: Self) -> Self {
        // Usually, folding an inner `None` with an `outer` prefers the
        // explicit `None`. However, here `None` means unspecified and thus
        // we want `outer`, so we use `fold_or` to opt into such behavior.
        self.zip(outer).map(|(inner, outer)| inner.fold_or(outer))
    }
}

/// The four sides of objects.
#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
pub enum Side {
    /// The left side.
    Left,
    /// The top side.
    Top,
    /// The right side.
    Right,
    /// The bottom side.
    Bottom,
}

impl Side {
    /// The opposite side.
    pub fn inv(self) -> Self {
        match self {
            Self::Left => Self::Right,
            Self::Top => Self::Bottom,
            Self::Right => Self::Left,
            Self::Bottom => Self::Top,
        }
    }

    /// The next side, clockwise.
    pub fn next_cw(self) -> Self {
        match self {
            Self::Left => Self::Top,
            Self::Top => Self::Right,
            Self::Right => Self::Bottom,
            Self::Bottom => Self::Left,
        }
    }

    /// The next side, counter-clockwise.
    pub fn next_ccw(self) -> Self {
        match self {
            Self::Left => Self::Bottom,
            Self::Top => Self::Left,
            Self::Right => Self::Top,
            Self::Bottom => Self::Right,
        }
    }

    /// The first corner of the side in clockwise order.
    pub fn start_corner(self) -> Corner {
        match self {
            Self::Left => Corner::BottomLeft,
            Self::Top => Corner::TopLeft,
            Self::Right => Corner::TopRight,
            Self::Bottom => Corner::BottomRight,
        }
    }

    /// The second corner of the side in clockwise order.
    pub fn end_corner(self) -> Corner {
        self.next_cw().start_corner()
    }

    /// Return the corresponding axis.
    pub fn axis(self) -> Axis {
        match self {
            Self::Left | Self::Right => Axis::Y,
            Self::Top | Self::Bottom => Axis::X,
        }
    }
}

cast! {
    Side,
    self => Alignment::from(self).into_value(),
    align: Alignment => match align {
        Alignment::LEFT => Self::Left,
        Alignment::RIGHT => Self::Right,
        Alignment::TOP => Self::Top,
        Alignment::BOTTOM => Self::Bottom,
        _ => bail!("cannot convert this alignment to a side"),
    },
}
