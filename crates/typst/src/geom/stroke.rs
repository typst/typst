use super::*;
use crate::eval::{dict, Cast, FromValue, NoneValue};

/// Defines how to draw a line.
///
/// A stroke has a _paint_ (typically a solid color), a _thickness,_ a line
/// _cap,_ a line _join,_ a _miter-limit,_ and a _dash_ pattern. All of these
/// values are optional and have sensible defaults.
///
/// # Example
/// ```example
/// #set line(length: 100%)
/// #stack(
///   spacing: 1em,
///   line(stroke: 2pt + red),
///   line(stroke: (paint: blue, thickness: 4pt, cap: "round")),
///   line(stroke: (paint: blue, thickness: 1pt, dash: "dashed")),
///   line(stroke: 2pt + gradient.linear(..color.map.rainbow)),
/// )
/// ```
///
/// # Simple strokes
/// You can create a simple solid stroke from a color, a thickness, or a
/// combination of the two. Specifically, wherever a stroke is expected you can
/// pass any of the following values:
///
/// - A length specifying the stroke's thickness. The color is inherited,
///   defaulting to black.
/// - A color to use for the stroke. The thickness is inherited, defaulting to
///   `{1pt}`.
/// - A stroke combined from color and thickness using the `+` operator as in
///   `{2pt + red}`.
///
/// # Complex strokes
/// For full control, you can also pass a [dictionary]($dictionary) to any
/// function that expects a stroke. This dictionary has the following keys:
///
/// - `paint`: The [color]($color) to use for the stroke.
///
/// - `thickness`: The stroke's thickness as a [length]($length).
///
/// - `cap`: How the line terminates. One of `{"butt"}`, `{"round"}`, or
///   `{"square"}`.
///
/// - `join`: How sharp turns of a contour are rendered. One of `{"miter"}`,
///   `{"round"}`, or `{"bevel"}`. Not applicable to lines but to
///   [polygons]($polygon) or [paths]($path).
///
/// - `miter-limit`: Number at which protruding sharp angles are rendered with a
///   bevel instead. The higher the number, the sharper an angle can be before
///   it is bevelled. Only applicable if `join` is `{"miter"}`. Defaults to
///   `{4.0}`.
///
/// - `dash`: The dash pattern to use. Can be any of the following:
///   - One of the predefined patterns `{"solid"}`, `{"dotted"}`,
///     `{"densely-dotted"}`, `{"loosely-dotted"}`, `{"dashed"}`,
///     `{"densely-dashed"}`, `{"loosely-dashed"}`, `{"dash-dotted"}`,
///     `{"densely-dash-dotted"}` or `{"loosely-dash-dotted"}`
///   - An [array]($array) with alternating lengths for dashes and gaps. You can
///     also use the string `{"dot"}` for a length equal to the line thickness.
///   - A [dictionary]($dictionary) with the keys `array` (same as the array
///     above), and `phase` (of type [length]($length)), which defines where in
///     the pattern to start drawing.
///
/// # Fields
/// On a `stroke` object, you can access any of the fields mentioned in the
/// dictionary format above. For example, `{(2pt + blue).thickness}` is `{2pt}`.
/// Meanwhile, `{(2pt + blue).cap}` is `{auto}` because it's unspecified.
#[ty]
#[derive(Debug, Default, Clone, Eq, PartialEq, Hash)]
pub struct Stroke<T: Numeric = Length> {
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

impl<T: Numeric> Stroke<T> {
    /// Map the contained lengths with `f`.
    pub fn map<F, U: Numeric>(self, f: F) -> Stroke<U>
    where
        F: Fn(T) -> U,
    {
        Stroke {
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

impl Stroke<Abs> {
    /// Unpack the stroke, filling missing fields from the `default`.
    pub fn unwrap_or(self, default: FixedStroke) -> FixedStroke {
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

        FixedStroke {
            paint: self.paint.unwrap_or(default.paint),
            thickness,
            line_cap: self.line_cap.unwrap_or(default.line_cap),
            line_join: self.line_join.unwrap_or(default.line_join),
            dash_pattern,
            miter_limit: self.miter_limit.unwrap_or(default.miter_limit),
        }
    }

    /// Unpack the stroke, filling missing fields with the default values.
    pub fn unwrap_or_default(self) -> FixedStroke {
        // we want to do this; the Clippy lint is not type-aware
        #[allow(clippy::unwrap_or_default)]
        self.unwrap_or(FixedStroke::default())
    }
}

impl<T: Numeric + Repr> Repr for Stroke<T> {
    fn repr(&self) -> EcoString {
        let mut r = EcoString::new();
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
                    r.push_str(&thickness.repr());
                    r.push_str(" + ");
                    r.push_str(&paint.repr());
                }
                (Smart::Custom(paint), Smart::Auto) => r.push_str(&paint.repr()),
                (Smart::Auto, Smart::Custom(thickness)) => r.push_str(&thickness.repr()),
                (Smart::Auto, Smart::Auto) => r.push_str("1pt + black"),
            }
        } else {
            r.push('(');
            let mut sep = "";
            if let Smart::Custom(paint) = &paint {
                r.push_str(sep);
                r.push_str("paint: ");
                r.push_str(&paint.repr());
                sep = ", ";
            }
            if let Smart::Custom(thickness) = &thickness {
                r.push_str(sep);
                r.push_str("thickness: ");
                r.push_str(&thickness.repr());
                sep = ", ";
            }
            if let Smart::Custom(cap) = &line_cap {
                r.push_str(sep);
                r.push_str("cap: ");
                r.push_str(&cap.repr());
                sep = ", ";
            }
            if let Smart::Custom(join) = &line_join {
                r.push_str(sep);
                r.push_str("join: ");
                r.push_str(&join.repr());
                sep = ", ";
            }
            if let Smart::Custom(dash) = &dash_pattern {
                r.push_str(sep);
                r.push_str("cap: ");
                if let Some(dash) = dash {
                    r.push_str(&dash.repr());
                } else {
                    r.push_str(&NoneValue.repr());
                }
                sep = ", ";
            }
            if let Smart::Custom(miter_limit) = &miter_limit {
                r.push_str(sep);
                r.push_str("miter-limit: ");
                r.push_str(&miter_limit.repr());
            }
            r.push(')');
        }
        r
    }
}

impl Resolve for Stroke {
    type Output = Stroke<Abs>;

    fn resolve(self, styles: StyleChain) -> Self::Output {
        Stroke {
            paint: self.paint,
            thickness: self.thickness.resolve(styles),
            line_cap: self.line_cap,
            line_join: self.line_join,
            dash_pattern: self.dash_pattern.resolve(styles),
            miter_limit: self.miter_limit,
        }
    }
}

impl Fold for Stroke<Abs> {
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
    type Stroke,
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
            miter_limit: miter_limit.map(Scalar::new),
        }
    },
}

cast! {
    Stroke<Abs>,
    self => self.map(Length::from).into_value(),
}

/// The line cap of a stroke
#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash, Cast)]
pub enum LineCap {
    Butt,
    Round,
    Square,
}

impl Repr for LineCap {
    fn repr(&self) -> EcoString {
        match self {
            Self::Butt => "butt".repr(),
            Self::Round => "round".repr(),
            Self::Square => "square".repr(),
        }
    }
}

/// The line join of a stroke
#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash, Cast)]
pub enum LineJoin {
    Miter,
    Round,
    Bevel,
}

impl Repr for LineJoin {
    fn repr(&self) -> EcoString {
        match self {
            Self::Miter => "miter".repr(),
            Self::Round => "round".repr(),
            Self::Bevel => "bevel".repr(),
        }
    }
}

/// A line dash pattern.
#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub struct DashPattern<T: Numeric = Length, DT = DashLength<T>> {
    /// The dash array.
    pub array: Vec<DT>,
    /// The dash phase.
    pub phase: T,
}

impl<T: Numeric + Repr, DT: Repr> Repr for DashPattern<T, DT> {
    fn repr(&self) -> EcoString {
        let mut r = EcoString::from("(array: (");
        for (i, elem) in self.array.iter().enumerate() {
            if i != 0 {
                r.push_str(", ")
            }
            r.push_str(&elem.repr())
        }
        r.push_str("), phase: ");
        r.push_str(&self.phase.repr());
        r.push(')');
        r
    }
}

impl<T: Numeric + Default> From<Vec<DashLength<T>>> for DashPattern<T> {
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
    self => dict! { "array" => self.array, "phase" => self.phase }.into_value(),

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

/// The length of a dash in a line dash pattern.
#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub enum DashLength<T: Numeric = Length> {
    LineWidth,
    Length(T),
}

impl<T: Numeric> DashLength<T> {
    fn finish(self, line_width: T) -> T {
        match self {
            Self::LineWidth => line_width,
            Self::Length(l) => l,
        }
    }
}

impl<T: Numeric + Repr> Repr for DashLength<T> {
    fn repr(&self) -> EcoString {
        match self {
            Self::LineWidth => "dot".repr(),
            Self::Length(v) => v.repr(),
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

impl From<Abs> for DashLength {
    fn from(l: Abs) -> Self {
        DashLength::Length(l.into())
    }
}

cast! {
    DashLength,
    self => match self {
        Self::LineWidth => "dot".into_value(),
        Self::Length(v) => v.into_value(),
    },
    "dot" => Self::LineWidth,
    v: Length => Self::Length(v),
}

/// A fully specified stroke of a geometric shape.
#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub struct FixedStroke {
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

impl Default for FixedStroke {
    fn default() -> Self {
        Self {
            paint: Paint::Solid(Color::BLACK),
            thickness: Abs::pt(1.0),
            line_cap: LineCap::Butt,
            line_join: LineJoin::Miter,
            dash_pattern: None,
            miter_limit: Scalar::new(4.0),
        }
    }
}
