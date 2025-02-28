use ecow::EcoString;
use typst_utils::{Numeric, Scalar};

use crate::diag::{HintedStrResult, SourceResult};
use crate::foundations::{
    cast, dict, func, scope, ty, Args, Cast, Dict, Fold, FromValue, NoneValue, Repr,
    Resolve, Smart, StyleChain, Value,
};
use crate::layout::{Abs, Length};
use crate::visualize::{Color, Gradient, Paint, Tiling};

/// Defines how to draw a line.
///
/// A stroke has a _paint_ (a solid color or gradient), a _thickness,_ a line
/// _cap,_ a line _join,_ a _miter limit,_ and a _dash_ pattern. All of these
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
/// For full control, you can also provide a [dictionary] or a `{stroke}` object
/// to any function that expects a stroke. The dictionary's keys may include any
/// of the parameters for the constructor function, shown below.
///
/// # Fields
/// On a stroke object, you can access any of the fields listed in the
/// constructor function. For example, `{(2pt + blue).thickness}` is `{2pt}`.
/// Meanwhile, `{stroke(red).cap}` is `{auto}` because it's unspecified. Fields
/// set to `{auto}` are inherited.
#[ty(scope, cast)]
#[derive(Debug, Default, Clone, Eq, PartialEq, Hash)]
pub struct Stroke<T: Numeric = Length> {
    /// The stroke's paint.
    pub paint: Smart<Paint>,
    /// The stroke's thickness.
    pub thickness: Smart<T>,
    /// The stroke's line cap.
    pub cap: Smart<LineCap>,
    /// The stroke's line join.
    pub join: Smart<LineJoin>,
    /// The stroke's line dash pattern.
    pub dash: Smart<Option<DashPattern<T>>>,
    /// The miter limit.
    pub miter_limit: Smart<Scalar>,
}

impl Stroke {
    /// Create a stroke from a paint and a thickness.
    pub fn from_pair(paint: impl Into<Paint>, thickness: Length) -> Self {
        Self {
            paint: Smart::Custom(paint.into()),
            thickness: Smart::Custom(thickness),
            ..Default::default()
        }
    }
}

#[scope]
impl Stroke {
    /// Converts a value to a stroke or constructs a stroke with the given
    /// parameters.
    ///
    /// Note that in most cases you do not need to convert values to strokes in
    /// order to use them, as they will be converted automatically. However,
    /// this constructor can be useful to ensure a value has all the fields of a
    /// stroke.
    ///
    /// ```example
    /// #let my-func(x) = {
    ///     x = stroke(x) // Convert to a stroke
    ///     [Stroke has thickness #x.thickness.]
    /// }
    /// #my-func(3pt) \
    /// #my-func(red) \
    /// #my-func(stroke(cap: "round", thickness: 1pt))
    /// ```
    #[func(constructor)]
    pub fn construct(
        args: &mut Args,

        /// The color or gradient to use for the stroke.
        ///
        /// If set to `{auto}`, the value is inherited, defaulting to `{black}`.
        #[external]
        paint: Smart<Paint>,

        /// The stroke's thickness.
        ///
        /// If set to `{auto}`, the value is inherited, defaulting to `{1pt}`.
        #[external]
        thickness: Smart<Length>,

        /// How the ends of the stroke are rendered.
        ///
        /// If set to `{auto}`, the value is inherited, defaulting to `{"butt"}`.
        #[external]
        cap: Smart<LineCap>,

        /// How sharp turns are rendered.
        ///
        /// If set to `{auto}`, the value is inherited, defaulting to `{"miter"}`.
        #[external]
        join: Smart<LineJoin>,

        /// The dash pattern to use. This can be:
        ///
        /// - One of the predefined patterns:
        ///   - `{"solid"}` or `{none}`
        ///   - `{"dotted"}`
        ///   - `{"densely-dotted"}`
        ///   - `{"loosely-dotted"}`
        ///   - `{"dashed"}`
        ///   - `{"densely-dashed"}`
        ///   - `{"loosely-dashed"}`
        ///   - `{"dash-dotted"}`
        ///   - `{"densely-dash-dotted"}`
        ///   - `{"loosely-dash-dotted"}`
        /// - An [array] with alternating lengths for dashes and gaps. You can
        ///   also use the string `{"dot"}` for a length equal to the line
        ///   thickness.
        /// - A [dictionary] with the keys `array` (same as the array above),
        ///   and `phase` (of type [length]), which defines where in the pattern
        ///   to start drawing.
        ///
        /// If set to `{auto}`, the value is inherited, defaulting to `{none}`.
        ///
        /// ```example
        /// #set line(length: 100%, stroke: 2pt)
        /// #stack(
        ///   spacing: 1em,
        ///   line(stroke: (dash: "dashed")),
        ///   line(stroke: (dash: (10pt, 5pt, "dot", 5pt))),
        ///   line(stroke: (dash: (array: (10pt, 5pt, "dot", 5pt), phase: 10pt))),
        /// )
        /// ```
        #[external]
        dash: Smart<Option<DashPattern>>,

        /// Number at which protruding sharp bends are rendered with a bevel
        /// instead or a miter join. The higher the number, the sharper an angle
        /// can be before it is bevelled. Only applicable if `join` is
        /// `{"miter"}`.
        ///
        /// Specifically, the miter limit is the maximum ratio between the
        /// corner's protrusion length and the stroke's thickness.
        ///
        /// If set to `{auto}`, the value is inherited, defaulting to `{4.0}`.
        ///
        /// ```example
        /// #let items = (
        ///   curve.move((15pt, 0pt)),
        ///   curve.line((0pt, 30pt)),
        ///   curve.line((30pt, 30pt)),
        ///   curve.line((10pt, 20pt)),
        /// )
        ///
        /// #set curve(stroke: 6pt + blue)
        /// #stack(
        ///   dir: ltr,
        ///   spacing: 1cm,
        ///   curve(stroke: (miter-limit: 1), ..items),
        ///   curve(stroke: (miter-limit: 4), ..items),
        ///   curve(stroke: (miter-limit: 5), ..items),
        /// )
        /// ```
        #[external]
        miter_limit: Smart<f64>,
    ) -> SourceResult<Stroke> {
        if let Some(stroke) = args.eat::<Stroke>()? {
            return Ok(stroke);
        }

        fn take<T: FromValue>(args: &mut Args, arg: &str) -> SourceResult<Smart<T>> {
            Ok(args.named::<Smart<T>>(arg)?.unwrap_or(Smart::Auto))
        }

        let paint = take::<Paint>(args, "paint")?;
        let thickness = take::<Length>(args, "thickness")?;
        let cap = take::<LineCap>(args, "cap")?;
        let join = take::<LineJoin>(args, "join")?;
        let dash = take::<Option<DashPattern>>(args, "dash")?;
        let miter_limit = take::<f64>(args, "miter-limit")?.map(Scalar::new);

        Ok(Self { paint, thickness, cap, join, dash, miter_limit })
    }
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
            cap: self.cap,
            join: self.join,
            dash: self.dash.map(|dash| {
                dash.map(|dash| DashPattern {
                    array: dash
                        .array
                        .into_iter()
                        .map(|l| match l {
                            DashLength::Length(v) => DashLength::Length(f(v)),
                            DashLength::LineWidth => DashLength::LineWidth,
                        })
                        .collect(),
                    phase: f(dash.phase),
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
        let dash = self
            .dash
            .map(|dash| {
                dash.map(|dash| DashPattern {
                    array: dash.array.into_iter().map(|l| l.finish(thickness)).collect(),
                    phase: dash.phase,
                })
            })
            .unwrap_or(default.dash);

        FixedStroke {
            paint: self.paint.unwrap_or(default.paint),
            thickness,
            cap: self.cap.unwrap_or(default.cap),
            join: self.join.unwrap_or(default.join),
            dash,
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
        let Self { paint, thickness, cap, join, dash, miter_limit } = &self;
        if cap.is_auto() && join.is_auto() && dash.is_auto() && miter_limit.is_auto() {
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
            if let Smart::Custom(cap) = &cap {
                r.push_str(sep);
                r.push_str("cap: ");
                r.push_str(&cap.repr());
                sep = ", ";
            }
            if let Smart::Custom(join) = &join {
                r.push_str(sep);
                r.push_str("join: ");
                r.push_str(&join.repr());
                sep = ", ";
            }
            if let Smart::Custom(dash) = &dash {
                r.push_str(sep);
                r.push_str("dash: ");
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
                r.push_str(&miter_limit.get().repr());
            }
            r.push(')');
        }
        r
    }
}

impl<T: Numeric + Fold> Fold for Stroke<T> {
    fn fold(self, outer: Self) -> Self {
        Self {
            paint: self.paint.or(outer.paint),
            thickness: self.thickness.or(outer.thickness),
            cap: self.cap.or(outer.cap),
            join: self.join.or(outer.join),
            dash: self.dash.or(outer.dash),
            miter_limit: self.miter_limit.or(outer.miter_limit),
        }
    }
}

impl Resolve for Stroke {
    type Output = Stroke<Abs>;

    fn resolve(self, styles: StyleChain) -> Self::Output {
        Stroke {
            paint: self.paint,
            thickness: self.thickness.resolve(styles),
            cap: self.cap,
            join: self.join,
            dash: self.dash.resolve(styles),
            miter_limit: self.miter_limit,
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
    gradient: Gradient => Self {
        paint: Smart::Custom(gradient.into()),
        ..Default::default()
    },
    tiling: Tiling => Self {
        paint: Smart::Custom(tiling.into()),
        ..Default::default()
    },
    mut dict: Dict => {
        // Get a value by key, accepting either Auto or something convertible to type T.
        fn take<T: FromValue>(dict: &mut Dict, key: &str) -> HintedStrResult<Smart<T>> {
            Ok(dict.take(key).ok().map(Smart::<T>::from_value)
                .transpose()?.unwrap_or(Smart::Auto))
        }

        let paint = take::<Paint>(&mut dict, "paint")?;
        let thickness = take::<Length>(&mut dict, "thickness")?;
        let cap = take::<LineCap>(&mut dict, "cap")?;
        let join = take::<LineJoin>(&mut dict, "join")?;
        let dash = take::<Option<DashPattern>>(&mut dict, "dash")?;
        let miter_limit = take::<f64>(&mut dict, "miter-limit")?;
        dict.finish(&["paint", "thickness", "cap", "join", "dash", "miter-limit"])?;

        Self {
            paint,
            thickness,
            cap,
            join,
            dash,
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
    /// Square stroke cap with the edge at the stroke's end point.
    Butt,
    /// Circular stroke cap centered at the stroke's end point.
    Round,
    /// Square stroke cap centered at the stroke's end point.
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
    /// Segments are joined with sharp edges. Sharp bends exceeding the miter
    /// limit are bevelled instead.
    Miter,
    /// Segments are joined with circular corners.
    Round,
    /// Segments are joined with a bevel (a straight edge connecting the butts
    /// of the joined segments).
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
    pub cap: LineCap,
    /// The stroke's line join.
    pub join: LineJoin,
    /// The stroke's line dash pattern.
    pub dash: Option<DashPattern<Abs, Abs>>,
    /// The miter limit. Defaults to 4.0, same as `tiny-skia`.
    pub miter_limit: Scalar,
}

impl FixedStroke {
    /// Create a stroke from a paint and a thickness.
    pub fn from_pair(paint: impl Into<Paint>, thickness: Abs) -> Self {
        Self {
            paint: paint.into(),
            thickness,
            ..Default::default()
        }
    }
}

impl Default for FixedStroke {
    fn default() -> Self {
        Self {
            paint: Paint::Solid(Color::BLACK),
            thickness: Abs::pt(1.0),
            cap: LineCap::Butt,
            join: LineJoin::Miter,
            dash: None,
            miter_limit: Scalar::new(4.0),
        }
    }
}
