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

#[derive(Debug, Default, Copy, Clone, Eq, PartialEq, Hash)]
pub struct Margin {
    pub sides: Sides<Option<Smart<Rel<Length>>>>,
    pub inside: Option<Smart<Rel<Length>>>,
    pub outside: Option<Smart<Rel<Length>>>,
}

impl Cast for Margin {
    fn cast(mut value: Value) -> StrResult<Self> {
        if let Value::Length(_) = value {
            Ok(Self::splat(Some(Value::cast(value)?)))
        } else if let Value::Relative(_) = value {
            Ok(Self::splat(Some(Value::cast(value)?)))
        } else if let Value::Dict(dict) = &mut value {
            let mut take = |key| dict.take(key).ok().map(Value::cast).transpose();

            let rest = take("rest")?;
            let x = take("x")?.or(rest);
            let y = take("y")?.or(rest);

            let outside = take("outside")?.or(x);
            let inside = take("inside")?.or(x);

            let sides = Sides {
                left: take("left")?.or(outside),
                top: take("top")?.or(y),
                right: take("right")?.or(inside),
                bottom: take("bottom")?.or(y),
            };

            let margin = Margin { sides, outside, inside };

            dict.finish(&[
                "left", "top", "right", "bottom", "x", "y", "outside", "inside", "rest",
            ])?;

            Ok(margin)
        } else {
            <Self as Cast>::error(value)
        }
    }

    fn is(value: &Value) -> bool {
        matches!(value, Value::Dict(_) | Value::Length(_) | Value::Relative(_))
    }

    fn describe() -> CastInfo {
        CastInfo::Type("relative length")
            + CastInfo::Type("dictionary")
            + CastInfo::Type("length")
    }
}

impl Margin {
    /// Create an instance with four equal components.
    pub fn splat(value: Option<Smart<Rel<Length>>>) -> Self {
        Self {
            sides: Sides::splat(value),
            outside: value,
            inside: value,
        }
    }
}
impl From<Margin> for Value {
    fn from(margin: Margin) -> Self {
        Self::from(margin.sides)
    }
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

impl<T> Cast for Sides<Option<T>>
where
    T: Default + Cast + Clone,
{
    fn is(value: &Value) -> bool {
        matches!(value, Value::Dict(_)) || T::is(value)
    }

    fn cast(mut value: Value) -> StrResult<Self> {
        if let Value::Dict(dict) = &mut value {
            let mut try_cast = || -> StrResult<_> {
                let mut take = |key| dict.take(key).ok().map(T::cast).transpose();

                let rest = take("rest")?;
                let x = take("x")?.or_else(|| rest.clone());
                let y = take("y")?.or_else(|| rest.clone());
                let sides = Sides {
                    left: take("left")?.or_else(|| x.clone()),
                    top: take("top")?.or_else(|| y.clone()),
                    right: take("right")?.or_else(|| x.clone()),
                    bottom: take("bottom")?.or_else(|| y.clone()),
                };

                dict.finish(&["left", "top", "right", "bottom", "x", "y", "rest"])?;

                Ok(sides)
            };

            if let Ok(res) = try_cast() {
                return Ok(res);
            }
        }

        if T::is(&value) {
            Ok(Self::splat(Some(T::cast(value)?)))
        } else {
            <Self as Cast>::error(value)
        }
    }

    fn describe() -> CastInfo {
        T::describe() + CastInfo::Type("dictionary")
    }
}

impl<T> From<Sides<Option<T>>> for Value
where
    T: PartialEq + Into<Value>,
{
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

impl Fold for Margin {
    type Output = Sides<Smart<Rel<Length>>>;

    fn fold(self, outer: Self::Output) -> Self::Output {
        self.sides.zip(outer).map(|(inner, outer)| match inner {
            Some(value) => value.fold(outer),
            None => outer,
        })
    }
}
