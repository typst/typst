use crate::eval::{Cast, FromValue};

use super::*;

/// A stroke of a geometric shape.
#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub struct Stroke {
    /// The stroke's paint.
    pub paint: Paint,
    /// The stroke's thickness.
    pub thickness: Abs,
    /// The stroke's line cap.
    pub line_cap: LineCap,
    /// The stroke's line join.
    pub line_join: LineJoin,
    /// The stroke's line dash pattern.
    pub dash_pattern: Option<DashPattern<Abs, Abs>>,
    /// The miter limit. Defaults to 4.0, same as `tiny-skia`.
    pub miter_limit: Scalar,
}

impl Default for Stroke {
    fn default() -> Self {
        Self {
            paint: Paint::Solid(Color::BLACK),
            thickness: Abs::pt(1.0),
            line_cap: LineCap::Butt,
            line_join: LineJoin::Miter,
            dash_pattern: None,
            miter_limit: Scalar(4.0),
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
    /// The stroke's line cap.
    pub line_cap: Smart<LineCap>,
    /// The stroke's line join.
    pub line_join: Smart<LineJoin>,
    /// The stroke's line dash pattern.
    pub dash_pattern: Smart<Option<DashPattern<T>>>,
    /// The miter limit.
    pub miter_limit: Smart<Scalar>,
}

impl<T> PartialStroke<T> {
    /// Map the contained lengths with `f`.
    pub fn map<F, U>(self, f: F) -> PartialStroke<U>
    where
        F: Fn(T) -> U,
    {
        PartialStroke {
            paint: self.paint,
            thickness: self.thickness.map(&f),
            line_cap: self.line_cap,
            line_join: self.line_join,
            dash_pattern: self.dash_pattern.map(|pattern| {
                pattern.map(|pattern| DashPattern {
                    array: pattern
                        .array
                        .into_iter()
                        .map(|l| match l {
                            DashLength::Length(v) => DashLength::Length(f(v)),
                            DashLength::LineWidth => DashLength::LineWidth,
                        })
                        .collect(),
                    phase: f(pattern.phase),
                })
            }),
            miter_limit: self.miter_limit,
        }
    }
}

impl PartialStroke<Abs> {
    /// Unpack the stroke, filling missing fields from the `default`.
    pub fn unwrap_or(self, default: Stroke) -> Stroke {
        let thickness = self.thickness.unwrap_or(default.thickness);
        let dash_pattern = self
            .dash_pattern
            .map(|pattern| {
                pattern.map(|pattern| DashPattern {
                    array: pattern
                        .array
                        .into_iter()
                        .map(|l| l.finish(thickness))
                        .collect(),
                    phase: pattern.phase,
                })
            })
            .unwrap_or(default.dash_pattern);

        Stroke {
            paint: self.paint.unwrap_or(default.paint),
            thickness,
            line_cap: self.line_cap.unwrap_or(default.line_cap),
            line_join: self.line_join.unwrap_or(default.line_join),
            dash_pattern,
            miter_limit: self.miter_limit.unwrap_or(default.miter_limit),
        }
    }

    /// Unpack the stroke, filling missing fields with the default values.
    pub fn unwrap_or_default(self) -> Stroke {
        self.unwrap_or(Stroke::default())
    }
}

impl<T: Debug> Debug for PartialStroke<T> {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        let Self {
            paint,
            thickness,
            line_cap,
            line_join,
            dash_pattern,
            miter_limit,
        } = &self;
        if line_cap.is_auto()
            && line_join.is_auto()
            && dash_pattern.is_auto()
            && miter_limit.is_auto()
        {
            match (&self.paint, &self.thickness) {
                (Smart::Custom(paint), Smart::Custom(thickness)) => {
                    write!(f, "{thickness:?} + {paint:?}")
                }
                (Smart::Custom(paint), Smart::Auto) => paint.fmt(f),
                (Smart::Auto, Smart::Custom(thickness)) => thickness.fmt(f),
                (Smart::Auto, Smart::Auto) => f.pad("1pt + black"),
            }
        } else {
            write!(f, "(")?;
            let mut sep = "";
            if let Smart::Custom(paint) = &paint {
                write!(f, "{}paint: {:?}", sep, paint)?;
                sep = ", ";
            }
            if let Smart::Custom(thickness) = &thickness {
                write!(f, "{}thickness: {:?}", sep, thickness)?;
                sep = ", ";
            }
            if let Smart::Custom(cap) = &line_cap {
                write!(f, "{}cap: {:?}", sep, cap)?;
                sep = ", ";
            }
            if let Smart::Custom(join) = &line_join {
                write!(f, "{}join: {:?}", sep, join)?;
                sep = ", ";
            }
            if let Smart::Custom(dash) = &dash_pattern {
                write!(f, "{}dash: {:?}", sep, dash)?;
                sep = ", ";
            }
            if let Smart::Custom(miter_limit) = &miter_limit {
                write!(f, "{}miter-limit: {:?}", sep, miter_limit)?;
            }
            write!(f, ")")?;
            Ok(())
        }
    }
}

impl Resolve for PartialStroke {
    type Output = PartialStroke<Abs>;

    fn resolve(self, styles: StyleChain) -> Self::Output {
        PartialStroke {
            paint: self.paint,
            thickness: self.thickness.resolve(styles),
            line_cap: self.line_cap,
            line_join: self.line_join,
            dash_pattern: self.dash_pattern.resolve(styles),
            miter_limit: self.miter_limit,
        }
    }
}

impl Fold for PartialStroke<Abs> {
    type Output = Self;

    fn fold(self, outer: Self::Output) -> Self::Output {
        Self {
            paint: self.paint.or(outer.paint),
            thickness: self.thickness.or(outer.thickness),
            line_cap: self.line_cap.or(outer.line_cap),
            line_join: self.line_join.or(outer.line_join),
            dash_pattern: self.dash_pattern.or(outer.dash_pattern),
            miter_limit: self.miter_limit.or(outer.miter_limit),
        }
    }
}

cast! {
    type PartialStroke: "stroke",
    thickness: Length => Self {
        thickness: Smart::Custom(thickness),
        ..Default::default()
    },
    color: Color => Self {
        paint: Smart::Custom(color.into()),
        ..Default::default()
    },
    mut dict: Dict => {
        fn take<T: FromValue>(dict: &mut Dict, key: &str) -> StrResult<Smart<T>> {
            Ok(dict.take(key).ok().map(T::from_value)
                .transpose()?.map(Smart::Custom).unwrap_or(Smart::Auto))
        }

        let paint = take::<Paint>(&mut dict, "paint")?;
        let thickness = take::<Length>(&mut dict, "thickness")?;
        let line_cap = take::<LineCap>(&mut dict, "cap")?;
        let line_join = take::<LineJoin>(&mut dict, "join")?;
        let dash_pattern = take::<Option<DashPattern>>(&mut dict, "dash")?;
        let miter_limit = take::<f64>(&mut dict, "miter-limit")?;
        dict.finish(&["paint", "thickness", "cap", "join", "dash", "miter-limit"])?;

        Self {
            paint,
            thickness,
            line_cap,
            line_join,
            dash_pattern,
            miter_limit: miter_limit.map(Scalar),
        }
    },
}

cast! {
    PartialStroke<Abs>,
    self => self.map(Length::from).into_value(),
}

/// The line cap of a stroke
#[derive(Copy, Clone, Eq, PartialEq, Hash, Cast)]
pub enum LineCap {
    Butt,
    Round,
    Square,
}

impl Debug for LineCap {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        match self {
            LineCap::Butt => write!(f, "\"butt\""),
            LineCap::Round => write!(f, "\"round\""),
            LineCap::Square => write!(f, "\"square\""),
        }
    }
}

/// The line join of a stroke
#[derive(Copy, Clone, Eq, PartialEq, Hash, Cast)]
pub enum LineJoin {
    Miter,
    Round,
    Bevel,
}

impl Debug for LineJoin {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        match self {
            LineJoin::Miter => write!(f, "\"miter\""),
            LineJoin::Round => write!(f, "\"round\""),
            LineJoin::Bevel => write!(f, "\"bevel\""),
        }
    }
}

/// A line dash pattern.
#[derive(Clone, Eq, PartialEq, Hash)]
pub struct DashPattern<T = Length, DT = DashLength<T>> {
    /// The dash array.
    pub array: Vec<DT>,
    /// The dash phase.
    pub phase: T,
}

impl<T: Debug, DT: Debug> Debug for DashPattern<T, DT> {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        write!(f, "(array: (")?;
        for (i, elem) in self.array.iter().enumerate() {
            if i == 0 {
                write!(f, "{:?}", elem)?;
            } else {
                write!(f, ", {:?}", elem)?;
            }
        }
        write!(f, "), phase: {:?})", self.phase)?;
        Ok(())
    }
}

impl<T: Default> From<Vec<DashLength<T>>> for DashPattern<T> {
    fn from(array: Vec<DashLength<T>>) -> Self {
        Self { array, phase: T::default() }
    }
}

impl Resolve for DashPattern {
    type Output = DashPattern<Abs>;

    fn resolve(self, styles: StyleChain) -> Self::Output {
        DashPattern {
            array: self.array.into_iter().map(|l| l.resolve(styles)).collect(),
            phase: self.phase.resolve(styles),
        }
    }
}

// Same names as tikz:
// https://tex.stackexchange.com/questions/45275/tikz-get-values-for-predefined-dash-patterns
cast! {
    DashPattern,

    "solid" => Vec::new().into(),
    "dotted" => vec![DashLength::LineWidth, Abs::pt(2.0).into()].into(),
    "densely-dotted" => vec![DashLength::LineWidth, Abs::pt(1.0).into()].into(),
    "loosely-dotted" => vec![DashLength::LineWidth, Abs::pt(4.0).into()].into(),
    "dashed" => vec![Abs::pt(3.0).into(), Abs::pt(3.0).into()].into(),
    "densely-dashed" => vec![Abs::pt(3.0).into(), Abs::pt(2.0).into()].into(),
    "loosely-dashed" => vec![Abs::pt(3.0).into(), Abs::pt(6.0).into()].into(),
    "dash-dotted" => vec![Abs::pt(3.0).into(), Abs::pt(2.0).into(), DashLength::LineWidth, Abs::pt(2.0).into()].into(),
    "densely-dash-dotted" => vec![Abs::pt(3.0).into(), Abs::pt(1.0).into(), DashLength::LineWidth, Abs::pt(1.0).into()].into(),
    "loosely-dash-dotted" => vec![Abs::pt(3.0).into(), Abs::pt(4.0).into(), DashLength::LineWidth, Abs::pt(4.0).into()].into(),

    array: Vec<DashLength> => Self { array, phase: Length::zero() },
    mut dict: Dict => {
        let array: Vec<DashLength> = dict.take("array")?.cast()?;
        let phase = dict.take("phase").ok().map(Value::cast)
            .transpose()?.unwrap_or(Length::zero());
        dict.finish(&["array", "phase"])?;
        Self {
            array,
            phase,
        }
    },
}

/// The length of a dash in a line dash pattern
#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub enum DashLength<T = Length> {
    LineWidth,
    Length(T),
}

impl From<Abs> for DashLength {
    fn from(l: Abs) -> Self {
        DashLength::Length(l.into())
    }
}

impl<T> DashLength<T> {
    fn finish(self, line_width: T) -> T {
        match self {
            Self::LineWidth => line_width,
            Self::Length(l) => l,
        }
    }
}

impl Resolve for DashLength {
    type Output = DashLength<Abs>;

    fn resolve(self, styles: StyleChain) -> Self::Output {
        match self {
            Self::LineWidth => DashLength::LineWidth,
            Self::Length(v) => DashLength::Length(v.resolve(styles)),
        }
    }
}

cast! {
    DashLength,
    "dot" => Self::LineWidth,
    v: Length => Self::Length(v),
}
