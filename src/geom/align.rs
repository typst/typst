use super::*;

/// Where to align something along an axis.
#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub enum Align {
    /// Align at the left side.
    Left,
    /// Align in the horizontal middle.
    Center,
    /// Align at the right side.
    Right,
    /// Align at the top side.
    Top,
    /// Align in the vertical middle.
    Horizon,
    /// Align at the bottom side.
    Bottom,
}

impl Align {
    /// Top-left alignment.
    pub const LEFT_TOP: Axes<Self> = Axes { x: Align::Left, y: Align::Top };

    /// Center-horizon alignment.
    pub const CENTER_HORIZON: Axes<Self> = Axes { x: Align::Center, y: Align::Horizon };

    /// The axis this alignment belongs to.
    pub const fn axis(self) -> Axis {
        match self {
            Self::Left | Self::Center | Self::Right => Axis::X,
            Self::Top | Self::Horizon | Self::Bottom => Axis::Y,
        }
    }

    /// The inverse alignment.
    pub const fn inv(self) -> Self {
        match self {
            Self::Left => Self::Right,
            Self::Center => Self::Center,
            Self::Right => Self::Left,
            Self::Top => Self::Bottom,
            Self::Horizon => Self::Horizon,
            Self::Bottom => Self::Top,
        }
    }

    /// Returns the position of this alignment in a container with the given
    /// extent.
    pub fn position(self, extent: Abs) -> Abs {
        match self {
            Self::Left | Self::Top => Abs::zero(),
            Self::Center | Self::Horizon => extent / 2.0,
            Self::Right | Self::Bottom => extent,
        }
    }
}

impl From<Side> for Align {
    fn from(side: Side) -> Self {
        match side {
            Side::Left => Self::Left,
            Side::Top => Self::Top,
            Side::Right => Self::Right,
            Side::Bottom => Self::Bottom,
        }
    }
}

impl Debug for Align {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        f.pad(match self {
            Self::Left => "left",
            Self::Center => "center",
            Self::Right => "right",
            Self::Top => "top",
            Self::Horizon => "horizon",
            Self::Bottom => "bottom",
        })
    }
}

/// The generic alignment representation.
#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub enum GenAlign {
    /// Align at the start side of the text direction.
    Start,
    /// Align at the end side of the text direction.
    End,
    /// Align at a specific alignment.
    Specific(Align),
}

impl GenAlign {
    /// The axis this alignment belongs to.
    pub const fn axis(self) -> Axis {
        match self {
            Self::Start | Self::End => Axis::X,
            Self::Specific(align) => align.axis(),
        }
    }
}

impl From<Align> for GenAlign {
    fn from(align: Align) -> Self {
        Self::Specific(align)
    }
}

impl From<HorizontalAlign> for GenAlign {
    fn from(align: HorizontalAlign) -> Self {
        align.0
    }
}

impl From<VerticalAlign> for GenAlign {
    fn from(align: VerticalAlign) -> Self {
        align.0
    }
}

impl Debug for GenAlign {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        match self {
            Self::Start => f.pad("start"),
            Self::End => f.pad("end"),
            Self::Specific(align) => align.fmt(f),
        }
    }
}

cast_from_value! {
    GenAlign: "alignment",
}

cast_from_value! {
    Axes<GenAlign>: "2d alignment",
}

cast_from_value! {
    Axes<Option<GenAlign>>,
    align: GenAlign => {
        let mut aligns = Axes::default();
        aligns.set(align.axis(), Some(align));
        aligns
    },
    aligns: Axes<GenAlign> => aligns.map(Some),
}

cast_to_value! {
    v: Axes<Align> => v.map(GenAlign::from).into()
}

cast_to_value! {
    v: Axes<Option<GenAlign>> => match (v.x, v.y) {
        (Some(x), Some(y)) => Axes::new(x, y).into(),
        (Some(x), None) => x.into(),
        (None, Some(y)) => y.into(),
        (None, None) => Value::None,
    }
}

impl From<Axes<GenAlign>> for Axes<Option<GenAlign>> {
    fn from(axes: Axes<GenAlign>) -> Self {
        axes.map(Some)
    }
}

impl From<Axes<Align>> for Axes<Option<GenAlign>> {
    fn from(axes: Axes<Align>) -> Self {
        axes.map(GenAlign::Specific).into()
    }
}

impl From<Align> for Axes<Option<GenAlign>> {
    fn from(align: Align) -> Self {
        let mut axes = Axes::splat(None);
        axes.set(align.axis(), Some(align.into()));
        axes
    }
}

impl Resolve for GenAlign {
    type Output = Align;

    fn resolve(self, styles: StyleChain) -> Self::Output {
        let dir = item!(dir)(styles);
        match self {
            Self::Start => dir.start().into(),
            Self::End => dir.end().into(),
            Self::Specific(align) => align,
        }
    }
}

impl Fold for GenAlign {
    type Output = Self;

    fn fold(self, _: Self::Output) -> Self::Output {
        self
    }
}

impl Fold for Align {
    type Output = Self;

    fn fold(self, _: Self::Output) -> Self::Output {
        self
    }
}

/// Utility struct to restrict a passed alignment value to the horizontal axis
/// on cast.
#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
pub struct HorizontalAlign(pub GenAlign);

cast_from_value! {
    HorizontalAlign,
    align: GenAlign => {
        if align.axis() != Axis::X {
            Err("alignment must be horizontal")?;
        }
        Self(align)
    },
}

cast_to_value! {
    v: HorizontalAlign => v.0.into()
}

/// Utility struct to restrict a passed alignment value to the vertical axis on
/// cast.
#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
pub struct VerticalAlign(pub GenAlign);

cast_from_value! {
    VerticalAlign,
    align: GenAlign => {
        if align.axis() != Axis::Y {
            Err("alignment must be vertical")?;
        }
        Self(align)
    },
}

cast_to_value! {
    v: VerticalAlign => v.0.into()
}

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum LeftRightAlternator {
    Left,
    Right,
}

impl Iterator for LeftRightAlternator {
    type Item = LeftRightAlternator;

    fn next(&mut self) -> Option<Self::Item> {
        let r = Some(*self);
        match self {
            Self::Left => *self = Self::Right,
            Self::Right => *self = Self::Left,
        }
        r
    }
}
