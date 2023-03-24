#[allow(clippy::wildcard_imports /* this module exists to reduce file size, not to introduce a new scope */)]
use super::*;

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
    #[must_use]
    #[inline]
    pub const fn new(left: T, top: T, right: T, bottom: T) -> Self {
        Self { left, top, right, bottom }
    }

    /// Create an instance with four equal components.
    #[must_use]
    #[inline]
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
    #[must_use]
    #[inline]
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

    /// Zip two instances into one.
    #[must_use]
    #[inline]
    pub fn zip<U>(self, other: Sides<U>) -> Sides<(T, U)> {
        Sides {
            left: (self.left, other.left),
            top: (self.top, other.top),
            right: (self.right, other.right),
            bottom: (self.bottom, other.bottom),
        }
    }

    /// An iterator over the sides, starting with the left side, clockwise.
    #[inline]
    pub fn iter(&self) -> impl Iterator<Item = &T> {
        [&self.left, &self.top, &self.right, &self.bottom].into_iter()
    }

    /// Whether all sides are equal.
    #[must_use]
    #[inline]
    pub fn is_uniform(&self) -> bool
    where
        T: PartialEq,
    {
        self.left == self.top && self.top == self.right && self.right == self.bottom
    }
}

impl<T: Add> Sides<T> {
    /// Sums up `left` and `right` into `x`, and `top` and `bottom` into `y`.
    #[must_use]
    #[inline]
    pub fn sum_by_axis(self) -> Axes<T::Output> {
        Axes::new(self.left + self.right, self.top + self.bottom)
    }
}

impl Sides<Rel<Abs>> {
    /// Evaluate the sides relative to the given `size`.
    #[must_use]
    #[inline]
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

    #[inline]
    fn get(self, side: Side) -> T {
        match side {
            Side::Left => self.left,
            Side::Top => self.top,
            Side::Right => self.right,
            Side::Bottom => self.bottom,
        }
    }

    #[inline]
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
    #[must_use]
    #[inline]
    pub fn inv(self) -> Self {
        match self {
            Self::Left => Self::Right,
            Self::Top => Self::Bottom,
            Self::Right => Self::Left,
            Self::Bottom => Self::Top,
        }
    }

    /// The next side, clockwise.
    #[must_use]
    #[inline]
    pub fn next_cw(self) -> Self {
        match self {
            Self::Left => Self::Top,
            Self::Top => Self::Right,
            Self::Right => Self::Bottom,
            Self::Bottom => Self::Left,
        }
    }

    /// The next side, counter-clockwise.
    #[must_use]
    #[inline]
    pub fn next_ccw(self) -> Self {
        match self {
            Self::Left => Self::Bottom,
            Self::Top => Self::Left,
            Self::Right => Self::Top,
            Self::Bottom => Self::Right,
        }
    }

    /// The first corner of the side in clockwise order.
    #[must_use]
    #[inline]
    pub fn start_corner(self) -> Corner {
        match self {
            Self::Left => Corner::BottomLeft,
            Self::Top => Corner::TopLeft,
            Self::Right => Corner::TopRight,
            Self::Bottom => Corner::BottomRight,
        }
    }

    /// The second corner of the side in clockwise order.
    #[must_use]
    #[inline]
    pub fn end_corner(self) -> Corner {
        self.next_cw().start_corner()
    }

    /// Return the corresponding axis.
    #[must_use]
    #[inline]
    pub fn axis(self) -> Axis {
        match self {
            Self::Left | Self::Right => Axis::Y,
            Self::Top | Self::Bottom => Axis::X,
        }
    }
}

impl<T> Cast for Sides<Option<T>>
where
    T: Default + Cast + Copy,
{
    #[inline]
    fn is(value: &Value) -> bool {
        matches!(value, Value::Dict(_)) || T::is(value)
    }

    #[inline]
    fn cast(mut value: Value) -> StrResult<Self> {
        if let Value::Dict(dict) = &mut value {
            let mut take = |key| dict.take(key).ok().map(T::cast).transpose();

            let rest = take("rest")?;
            let x = take("x")?.or(rest);
            let y = take("y")?.or(rest);
            let sides = Sides {
                left: take("left")?.or(x),
                top: take("top")?.or(y),
                right: take("right")?.or(x),
                bottom: take("bottom")?.or(y),
            };

            dict.finish(&["left", "top", "right", "bottom", "x", "y", "rest"])?;

            Ok(sides)
        } else if T::is(&value) {
            Ok(Self::splat(Some(T::cast(value)?)))
        } else {
            <Self as Cast>::error(value)
        }
    }

    #[inline]
    fn describe() -> CastInfo {
        T::describe() + CastInfo::Type("dictionary")
    }
}

impl<T> From<Sides<Option<T>>> for Value
where
    T: PartialEq + Into<Value>,
{
    #[inline]
    fn from(sides: Sides<Option<T>>) -> Self {
        if sides.is_uniform() {
            if let Some(value) = sides.left {
                return value.into();
            }
        }

        let mut dict = Dict::new();
        if let Some(left) = sides.left {
            dict.insert("left".into(), left.into());
        }
        if let Some(top) = sides.top {
            dict.insert("top".into(), top.into());
        }
        if let Some(right) = sides.right {
            dict.insert("right".into(), right.into());
        }
        if let Some(bottom) = sides.bottom {
            dict.insert("bottom".into(), bottom.into());
        }

        Value::Dict(dict)
    }
}

impl<T: Resolve> Resolve for Sides<T> {
    type Output = Sides<T::Output>;

    #[inline]
    fn resolve(self, styles: StyleChain<'_>) -> Self::Output {
        self.map(|v| v.resolve(styles))
    }
}

impl<T: Fold> Fold for Sides<Option<T>> {
    type Output = Sides<T::Output>;

    #[inline]
    fn fold(self, outer: Self::Output) -> Self::Output {
        self.zip(outer).map(|(inner, outer)| match inner {
            Some(value) => value.fold(outer),
            None => outer,
        })
    }
}
