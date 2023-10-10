use super::*;
use crate::eval::{CastInfo, FromValue, IntoValue, Reflect};

/// A container with left, top, right and bottom components.
#[derive(Debug, Default, Copy, Clone, Eq, PartialEq, Hash)]
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

impl Sides<Rel<Abs>> {
    /// Evaluate the sides relative to the given `size`.
    pub fn relative_to(self, size: Size) -> Sides<Abs> {
        Sides {
            left: self.left.relative_to(size.x),
            top: self.top.relative_to(size.y),
            right: self.right.relative_to(size.x),
            bottom: self.bottom.relative_to(size.y),
        }
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
    self => Align::from(self).into_value(),
    align: Align => match align {
        Align::LEFT => Self::Left,
        Align::RIGHT => Self::Right,
        Align::TOP => Self::Top,
        Align::BOTTOM => Self::Bottom,
        _ => bail!("cannot convert this alignment to a side"),
    },
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

impl<T> IntoValue for Sides<T>
where
    T: PartialEq + IntoValue,
{
    fn into_value(self) -> Value {
        if self.is_uniform() {
            return self.left.into_value();
        }

        let mut dict = Dict::new();
        let mut handle = |key: &str, component: T| {
            let value = component.into_value();
            if value != Value::None {
                dict.insert(key.into(), value);
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
    fn from_value(mut value: Value) -> StrResult<Self> {
        let keys = ["left", "top", "right", "bottom", "x", "y", "rest"];
        if let Value::Dict(dict) = &mut value {
            if dict.iter().any(|(key, _)| keys.contains(&key.as_str())) {
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

                dict.finish(&keys)?;
                return Ok(sides);
            }
        }

        if T::castable(&value) {
            Ok(Self::splat(Some(T::from_value(value)?)))
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
    type Output = Sides<T::Output>;

    fn fold(self, outer: Self::Output) -> Self::Output {
        self.zip(outer).map(|(inner, outer)| match inner {
            Some(value) => value.fold(outer),
            None => outer,
        })
    }
}
