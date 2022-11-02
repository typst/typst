use std::fmt::{self, Debug, Formatter};

use crate::geom::{Abs, Align, Axes, Axis, Get, Length, Paint, Stroke};
use crate::library::text::TextNode;
use crate::model::{Fold, Resolve, Smart, StyleChain, Value};

/// The unresolved alignment representation.
#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub enum RawAlign {
    /// Align at the start side of the text direction.
    Start,
    /// Align at the end side of the text direction.
    End,
    /// Align at a specific alignment.
    Specific(Align),
}

impl Resolve for RawAlign {
    type Output = Align;

    fn resolve(self, styles: StyleChain) -> Self::Output {
        let dir = styles.get(TextNode::DIR);
        match self {
            Self::Start => dir.start().into(),
            Self::End => dir.end().into(),
            Self::Specific(align) => align,
        }
    }
}

impl RawAlign {
    /// The axis this alignment belongs to.
    pub const fn axis(self) -> Axis {
        match self {
            Self::Start | Self::End => Axis::X,
            Self::Specific(align) => align.axis(),
        }
    }
}

impl From<Align> for RawAlign {
    fn from(align: Align) -> Self {
        Self::Specific(align)
    }
}

impl Debug for RawAlign {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        match self {
            Self::Start => f.pad("start"),
            Self::End => f.pad("end"),
            Self::Specific(align) => align.fmt(f),
        }
    }
}

dynamic! {
    RawAlign: "alignment",
}

dynamic! {
    Axes<RawAlign>: "2d alignment",
}

castable! {
    Axes<Option<RawAlign>>,
    Expected: "1d or 2d alignment",
    @align: RawAlign => {
        let mut aligns = Axes::default();
        aligns.set(align.axis(), Some(*align));
        aligns
    },
    @aligns: Axes<RawAlign> => aligns.map(Some),
}

/// The unresolved stroke representation.
///
/// In this representation, both fields are optional so that you can pass either
/// just a paint (`red`), just a thickness (`0.1em`) or both (`2pt + red`) where
/// this is expected.
#[derive(Default, Copy, Clone, Eq, PartialEq, Hash)]
pub struct RawStroke<T = Length> {
    /// The stroke's paint.
    pub paint: Smart<Paint>,
    /// The stroke's thickness.
    pub thickness: Smart<T>,
}

impl RawStroke<Abs> {
    /// Unpack the stroke, filling missing fields from the `default`.
    pub fn unwrap_or(self, default: Stroke) -> Stroke {
        Stroke {
            paint: self.paint.unwrap_or(default.paint),
            thickness: self.thickness.unwrap_or(default.thickness),
        }
    }

    /// Unpack the stroke, filling missing fields with the default values.
    pub fn unwrap_or_default(self) -> Stroke {
        self.unwrap_or(Stroke::default())
    }
}

impl Resolve for RawStroke {
    type Output = RawStroke<Abs>;

    fn resolve(self, styles: StyleChain) -> Self::Output {
        RawStroke {
            paint: self.paint,
            thickness: self.thickness.resolve(styles),
        }
    }
}

impl Fold for RawStroke<Abs> {
    type Output = Self;

    fn fold(self, outer: Self::Output) -> Self::Output {
        Self {
            paint: self.paint.or(outer.paint),
            thickness: self.thickness.or(outer.thickness),
        }
    }
}

impl<T: Debug> Debug for RawStroke<T> {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        match (self.paint, &self.thickness) {
            (Smart::Custom(paint), Smart::Custom(thickness)) => {
                write!(f, "{thickness:?} + {paint:?}")
            }
            (Smart::Custom(paint), Smart::Auto) => paint.fmt(f),
            (Smart::Auto, Smart::Custom(thickness)) => thickness.fmt(f),
            (Smart::Auto, Smart::Auto) => f.pad("<stroke>"),
        }
    }
}

dynamic! {
    RawStroke: "stroke",
    Value::Length(thickness) => Self {
        paint: Smart::Auto,
        thickness: Smart::Custom(thickness),
    },
    Value::Color(color) => Self {
        paint: Smart::Custom(color.into()),
        thickness: Smart::Auto,
    },
}
