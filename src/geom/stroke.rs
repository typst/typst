use super::*;

/// A stroke of a geometric shape.
#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub struct Stroke {
    /// The stroke's paint.
    pub paint: Paint,
    /// The stroke's thickness.
    pub thickness: Abs,
}

impl Default for Stroke {
    fn default() -> Self {
        Self {
            paint: Paint::Solid(Color::BLACK),
            thickness: Abs::pt(1.0),
        }
    }
}

/// A partial stroke representation.
///
/// In this representation, both fields are optional so that you can pass either
/// just a paint (`red`), just a thickness (`0.1em`) or both (`2pt + red`) where
/// this is expected.
#[derive(Default, Clone, Eq, PartialEq, Hash)]
pub struct PartialStroke<T = Length> {
    /// The stroke's paint.
    pub paint: Smart<Paint>,
    /// The stroke's thickness.
    pub thickness: Smart<T>,
}

impl PartialStroke<Abs> {
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

impl<T: Debug> Debug for PartialStroke<T> {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        match (&self.paint, &self.thickness) {
            (Smart::Custom(paint), Smart::Custom(thickness)) => {
                write!(f, "{thickness:?} + {paint:?}")
            }
            (Smart::Custom(paint), Smart::Auto) => paint.fmt(f),
            (Smart::Auto, Smart::Custom(thickness)) => thickness.fmt(f),
            (Smart::Auto, Smart::Auto) => f.pad("<stroke>"),
        }
    }
}

cast_from_value! {
    PartialStroke: "stroke",
    thickness: Length => Self {
        paint: Smart::Auto,
        thickness: Smart::Custom(thickness),
    },
    color: Color => Self {
        paint: Smart::Custom(color.into()),
        thickness: Smart::Auto,
    },
}

impl Resolve for PartialStroke {
    type Output = PartialStroke<Abs>;

    fn resolve(self, styles: StyleChain) -> Self::Output {
        PartialStroke {
            paint: self.paint,
            thickness: self.thickness.resolve(styles),
        }
    }
}

impl Fold for PartialStroke<Abs> {
    type Output = Self;

    fn fold(self, outer: Self::Output) -> Self::Output {
        Self {
            paint: self.paint.or(outer.paint),
            thickness: self.thickness.or(outer.thickness),
        }
    }
}
