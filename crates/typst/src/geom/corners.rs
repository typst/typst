use super::*;
use crate::eval::{CastInfo, FromValue, IntoValue, Reflect};

/// A container with components for the four corners of a rectangle.
#[derive(Debug, Default, Copy, Clone, Eq, PartialEq, Hash)]
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

impl<T> IntoValue for Corners<T>
where
    T: PartialEq + IntoValue,
{
    fn into_value(self) -> Value {
        if self.is_uniform() {
            return self.top_left.into_value();
        }

        let mut dict = Dict::new();
        let mut handle = |key: &str, component: T| {
            let value = component.into_value();
            if value != Value::None {
                dict.insert(key.into(), value);
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
    fn from_value(mut value: Value) -> StrResult<Self> {
        let keys = [
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
            if dict.iter().any(|(key, _)| keys.contains(&key.as_str())) {
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

                dict.finish(&keys)?;
                return Ok(corners);
            }
        }

        if T::castable(&value) {
            Ok(Self::splat(Some(T::from_value(value)?)))
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
    type Output = Corners<T::Output>;

    fn fold(self, outer: Self::Output) -> Self::Output {
        self.zip(outer).map(|(inner, outer)| match inner {
            Some(value) => value.fold(outer),
            None => outer,
        })
    }
}
