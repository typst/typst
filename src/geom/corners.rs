use super::*;

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

impl<T> Cast for Corners<Option<T>>
where
    T: Cast + Clone,
{
    fn is(value: &Value) -> bool {
        matches!(value, Value::Dict(_)) || T::is(value)
    }

    fn cast(mut value: Value) -> StrResult<Self> {
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
                let mut take = |key| dict.take(key).ok().map(T::cast).transpose();
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

impl<T> From<Corners<T>> for Value
where
    T: PartialEq + Into<Value>,
{
    fn from(corners: Corners<T>) -> Self {
        if corners.is_uniform() {
            return corners.top_left.into();
        }

        let mut dict = Dict::new();
        let mut handle = |key: &str, component: T| {
            let value = component.into();
            if value != Value::None {
                dict.insert(key.into(), value);
            }
        };

        handle("top-left", corners.top_left);
        handle("top-right", corners.top_right);
        handle("bottom-right", corners.bottom_right);
        handle("bottom-left", corners.bottom_left);

        Value::Dict(dict)
    }
}
