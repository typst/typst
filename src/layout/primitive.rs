//! Layouting primitives.

use std::fmt::{self, Display, Formatter};

/// Generic access to a structure's components.
pub trait Get<Index> {
    /// The structure's component type.
    type Component;

    /// Return the component for the specified index.
    fn get(self, index: Index) -> Self::Component;

    /// Borrow the component for the specified index mutably.
    fn get_mut(&mut self, index: Index) -> &mut Self::Component;
}

/// Convert a type into its generic representation.
///
/// The generic representation deals with main and cross axes while the specific
/// representation deals with horizontal and vertical axes.
///
/// See also [`ToSpec`] for the inverse conversion.
///
/// [`ToSpec`]: trait.ToSpec.html
pub trait ToGen {
    /// The generic version of this type.
    type Output;

    /// The generic version of this type based on the current directions.
    fn to_gen(self, dirs: Gen2<Dir>) -> Self::Output;
}

/// Convert a type into its specific representation.
///
/// The specific representation deals with horizontal and vertical axes while
/// the generic representation deals with main and cross axes.
///
/// See also [`ToGen`] for the inverse conversion.
///
/// [`ToGen`]: trait.ToGen.html
pub trait ToSpec {
    /// The specific version of this type.
    type Output;

    /// The specific version of this type based on the current directions.
    fn to_spec(self, dirs: Gen2<Dir>) -> Self::Output;
}

/// The four directions into which content can be laid out.
#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum Dir {
    /// Left to right.
    LTR,
    /// Right to left.
    RTL,
    /// Top to bottom.
    TTB,
    /// Bottom to top.
    BTT,
}

impl Dir {
    /// The specific axis this direction belongs to.
    pub fn axis(self) -> SpecAxis {
        match self {
            Self::LTR | Self::RTL => SpecAxis::Horizontal,
            Self::TTB | Self::BTT => SpecAxis::Vertical,
        }
    }

    /// Whether this direction points into the positive coordinate direction.
    ///
    /// The positive directions are left-to-right and top-to-bottom.
    pub fn is_positive(self) -> bool {
        match self {
            Self::LTR | Self::TTB => true,
            Self::RTL | Self::BTT => false,
        }
    }

    /// The factor for this direction.
    ///
    /// - `1.0` if the direction is positive.
    /// - `-1.0` if the direction is negative.
    pub fn factor(self) -> f64 {
        if self.is_positive() { 1.0 } else { -1.0 }
    }

    /// The side of this direction the alignment identifies.
    ///
    /// `Center` alignment is treated the same as `Start` alignment.
    pub fn side(self, align: GenAlign) -> Side {
        let start = match self {
            Self::LTR => Side::Left,
            Self::RTL => Side::Right,
            Self::TTB => Side::Top,
            Self::BTT => Side::Bottom,
        };

        match align {
            GenAlign::Start | GenAlign::Center => start,
            GenAlign::End => start.inv(),
        }
    }

    /// The inverse direction.
    pub fn inv(self) -> Self {
        match self {
            Self::LTR => Self::RTL,
            Self::RTL => Self::LTR,
            Self::TTB => Self::BTT,
            Self::BTT => Self::TTB,
        }
    }
}

impl Display for Dir {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        f.pad(match self {
            Self::LTR => "ltr",
            Self::RTL => "rtl",
            Self::TTB => "ttb",
            Self::BTT => "btt",
        })
    }
}

/// A generic container with two components for the two generic axes.
#[derive(Debug, Default, Copy, Clone, Eq, PartialEq)]
pub struct Gen2<T> {
    /// The main component.
    pub main: T,
    /// The cross component.
    pub cross: T,
}

impl<T> Gen2<T> {
    /// Create a new instance from the two components.
    pub fn new(main: T, cross: T) -> Self {
        Self { main, cross }
    }
}

impl<T> Get<GenAxis> for Gen2<T> {
    type Component = T;

    fn get(self, axis: GenAxis) -> T {
        match axis {
            GenAxis::Main => self.main,
            GenAxis::Cross => self.cross,
        }
    }

    fn get_mut(&mut self, axis: GenAxis) -> &mut T {
        match axis {
            GenAxis::Main => &mut self.main,
            GenAxis::Cross => &mut self.cross,
        }
    }
}

impl<T> ToSpec for Gen2<T> {
    type Output = Spec2<T>;

    fn to_spec(self, dirs: Gen2<Dir>) -> Self::Output {
        match dirs.main.axis() {
            SpecAxis::Horizontal => Spec2::new(self.main, self.cross),
            SpecAxis::Vertical => Spec2::new(self.cross, self.main),
        }
    }
}

/// A generic container with two components for the two specific axes.
#[derive(Debug, Default, Copy, Clone, Eq, PartialEq)]
pub struct Spec2<T> {
    /// The horizontal component.
    pub horizontal: T,
    /// The vertical component.
    pub vertical: T,
}

impl<T> Spec2<T> {
    /// Create a new instance from the two components.
    pub fn new(horizontal: T, vertical: T) -> Self {
        Self { horizontal, vertical }
    }
}

impl<T> Get<SpecAxis> for Spec2<T> {
    type Component = T;

    fn get(self, axis: SpecAxis) -> T {
        match axis {
            SpecAxis::Horizontal => self.horizontal,
            SpecAxis::Vertical => self.vertical,
        }
    }

    fn get_mut(&mut self, axis: SpecAxis) -> &mut T {
        match axis {
            SpecAxis::Horizontal => &mut self.horizontal,
            SpecAxis::Vertical => &mut self.vertical,
        }
    }
}

impl<T> ToGen for Spec2<T> {
    type Output = Gen2<T>;

    fn to_gen(self, dirs: Gen2<Dir>) -> Self::Output {
        match dirs.main.axis() {
            SpecAxis::Horizontal => Gen2::new(self.horizontal, self.vertical),
            SpecAxis::Vertical => Gen2::new(self.vertical, self.horizontal),
        }
    }
}

/// The two generic layouting axes.
#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum GenAxis {
    /// The axis pages and paragraphs are set along.
    Main,
    /// The axis words and lines are set along.
    Cross,
}

impl GenAxis {
    /// The other axis.
    pub fn other(self) -> Self {
        match self {
            Self::Main => Self::Cross,
            Self::Cross => Self::Main,
        }
    }
}

impl ToSpec for GenAxis {
    type Output = SpecAxis;

    fn to_spec(self, dirs: Gen2<Dir>) -> Self::Output {
        match self {
            Self::Main => dirs.main.axis(),
            Self::Cross => dirs.cross.axis(),
        }
    }
}

impl Display for GenAxis {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        f.pad(match self {
            Self::Main => "main",
            Self::Cross => "cross",
        })
    }
}

/// The two specific layouting axes.
#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum SpecAxis {
    /// The vertical layouting axis.
    Vertical,
    /// The horizontal layouting axis.
    Horizontal,
}

impl SpecAxis {
    /// The other axis.
    pub fn other(self) -> Self {
        match self {
            Self::Horizontal => Self::Vertical,
            Self::Vertical => Self::Horizontal,
        }
    }
}

impl ToGen for SpecAxis {
    type Output = GenAxis;

    fn to_gen(self, dirs: Gen2<Dir>) -> Self::Output {
        if self == dirs.main.axis() {
            GenAxis::Main
        } else {
            debug_assert_eq!(self, dirs.cross.axis());
            GenAxis::Cross
        }
    }
}

impl Display for SpecAxis {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        f.pad(match self {
            Self::Vertical => "vertical",
            Self::Horizontal => "horizontal",
        })
    }
}

/// Where to align content along an axis in a generic context.
#[derive(Debug, Copy, Clone, Eq, PartialEq, Ord, PartialOrd)]
pub enum GenAlign {
    Start,
    Center,
    End,
}

impl GenAlign {
    /// The inverse alignment.
    pub fn inv(self) -> Self {
        match self {
            Self::Start => Self::End,
            Self::Center => Self::Center,
            Self::End => Self::Start,
        }
    }
}

impl Default for GenAlign {
    fn default() -> Self {
        Self::Start
    }
}

impl Display for GenAlign {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        f.pad(match self {
            Self::Start => "start",
            Self::Center => "center",
            Self::End => "end",
        })
    }
}

/// Where to align content along an axis in a specific context.
#[derive(Debug, Copy, Clone, Eq, PartialEq, Ord, PartialOrd)]
pub enum SpecAlign {
    Left,
    Right,
    Top,
    Bottom,
    Center,
}

impl SpecAlign {
    /// The specific axis this alignment refers to.
    ///
    /// Returns `None` if this is `Center` since the axis is unknown.
    pub fn axis(self) -> Option<SpecAxis> {
        match self {
            Self::Left => Some(SpecAxis::Horizontal),
            Self::Right => Some(SpecAxis::Horizontal),
            Self::Top => Some(SpecAxis::Vertical),
            Self::Bottom => Some(SpecAxis::Vertical),
            Self::Center => None,
        }
    }

    /// The inverse alignment.
    pub fn inv(self) -> Self {
        match self {
            Self::Left => Self::Right,
            Self::Right => Self::Left,
            Self::Top => Self::Bottom,
            Self::Bottom => Self::Top,
            Self::Center => Self::Center,
        }
    }
}

impl ToGen for SpecAlign {
    type Output = GenAlign;

    fn to_gen(self, dirs: Gen2<Dir>) -> Self::Output {
        let dirs = dirs.to_spec(dirs);
        let get = |dir: Dir, at_positive_start| {
            if dir.is_positive() == at_positive_start {
                GenAlign::Start
            } else {
                GenAlign::End
            }
        };

        match self {
            Self::Left => get(dirs.horizontal, true),
            Self::Right => get(dirs.horizontal, false),
            Self::Top => get(dirs.vertical, true),
            Self::Bottom => get(dirs.vertical, false),
            Self::Center => GenAlign::Center,
        }
    }
}

impl Display for SpecAlign {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        f.pad(match self {
            Self::Left => "left",
            Self::Right => "right",
            Self::Top => "top",
            Self::Bottom => "bottom",
            Self::Center => "center",
        })
    }
}

/// A generic container with left, top, right and bottom components.
#[derive(Debug, Default, Copy, Clone, Eq, PartialEq)]
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
    /// Create a new box from four sizes.
    pub fn new(left: T, top: T, right: T, bottom: T) -> Self {
        Self { left, top, right, bottom }
    }

    /// Create an instance with all four components set to the same `value`.
    pub fn uniform(value: T) -> Self
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
}

impl<T> Get<Side> for Sides<T> {
    type Component = T;

    fn get(self, side: Side) -> T {
        match side {
            Side::Left => self.left,
            Side::Top => self.top,
            Side::Right => self.right,
            Side::Bottom => self.bottom,
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

/// A side of a container.
#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum Side {
    Left,
    Top,
    Right,
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
}
