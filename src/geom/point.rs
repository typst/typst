use super::*;

/// A point in 2D.
#[derive(Default, Copy, Clone, Eq, PartialEq, Hash)]
pub struct Point {
    /// The x coordinate.
    pub x: Length,
    /// The y coordinate.
    pub y: Length,
}

impl Point {
    /// The origin point.
    pub const fn zero() -> Self {
        Self { x: Length::zero(), y: Length::zero() }
    }

    /// Create a new point from x and y coordinate.
    pub const fn new(x: Length, y: Length) -> Self {
        Self { x, y }
    }

    /// Create an instance with two equal components.
    pub const fn splat(value: Length) -> Self {
        Self { x: value, y: value }
    }

    /// Create a new point with y set to zero.
    pub const fn with_x(x: Length) -> Self {
        Self { x, y: Length::zero() }
    }

    /// Create a new point with x set to zero.
    pub const fn with_y(y: Length) -> Self {
        Self { x: Length::zero(), y }
    }

    /// Whether both components are zero.
    pub fn is_zero(self) -> bool {
        self.x.is_zero() && self.y.is_zero()
    }

    /// Transform the point with the given transformation.
    pub fn transform(self, transform: Transform) -> Self {
        Self::new(
            transform.sx.resolve(self.x) + transform.kx.resolve(self.y) + transform.tx,
            transform.ky.resolve(self.x) + transform.sy.resolve(self.y) + transform.ty,
        )
    }
}

impl From<Spec<Length>> for Point {
    fn from(spec: Spec<Length>) -> Self {
        Self::new(spec.x, spec.y)
    }
}

impl Get<SpecAxis> for Point {
    type Component = Length;

    fn get(self, axis: SpecAxis) -> Length {
        match axis {
            SpecAxis::Horizontal => self.x,
            SpecAxis::Vertical => self.y,
        }
    }

    fn get_mut(&mut self, axis: SpecAxis) -> &mut Length {
        match axis {
            SpecAxis::Horizontal => &mut self.x,
            SpecAxis::Vertical => &mut self.y,
        }
    }
}

impl Debug for Point {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        write!(f, "Point({:?}, {:?})", self.x, self.y)
    }
}

impl Neg for Point {
    type Output = Self;

    fn neg(self) -> Self {
        Self { x: -self.x, y: -self.y }
    }
}

impl Add for Point {
    type Output = Self;

    fn add(self, other: Self) -> Self {
        Self { x: self.x + other.x, y: self.y + other.y }
    }
}

sub_impl!(Point - Point -> Point);

impl Mul<f64> for Point {
    type Output = Self;

    fn mul(self, other: f64) -> Self {
        Self { x: self.x * other, y: self.y * other }
    }
}

impl Mul<Point> for f64 {
    type Output = Point;

    fn mul(self, other: Point) -> Point {
        other * self
    }
}

impl Div<f64> for Point {
    type Output = Self;

    fn div(self, other: f64) -> Self {
        Self { x: self.x / other, y: self.y / other }
    }
}

assign_impl!(Point += Point);
assign_impl!(Point -= Point);
assign_impl!(Point *= f64);
assign_impl!(Point /= f64);
