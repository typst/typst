use std::f64::consts::{FRAC_PI_2, PI, TAU};
use std::f64::{EPSILON, NEG_INFINITY};
use std::fmt::{Debug, Write};
use std::hash::Hash;

use ecow::EcoVec;
use typst_macros::{cast, func, scope, ty};
use typst_syntax::{Span, Spanned};

use super::*;
use crate::diag::{bail, error, SourceResult};
use crate::eval::{array, Array, IntoValue};
use crate::geom::{ColorSpace, Smart};

/// A color gradient.
///
/// Typst supports:
/// - Linear gradients through the [`gradient.linear` function]($gradient.linear)
#[ty(scope)]
#[derive(Clone, PartialEq, Eq, Hash)]
pub enum Gradient {
    Linear(LinearGradient),
}

impl Debug for Gradient {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Linear(linear) => linear.fmt(f),
        }
    }
}

#[scope]
impl Gradient {
    /// Creates a new linear gradient.
    ///
    /// # Stops
    ///
    /// # Position
    ///
    /// # Angle
    ///
    /// # Color space
    ///
    /// # Relative
    ///
    ///
    #[func(title = "Linear gradient")]
    pub fn linear(
        /// The call site of this function.
        span: Span,

        /// The color stops of the gradient.
        #[variadic]
        stops: Vec<Spanned<Stop>>,

        /// The direction or angle of the gradient.
        #[named]
        #[default(DirOrAngle::Dir(Dir::LTR))]
        dir: DirOrAngle,

        /// The color space in which to interpolate the gradient.
        ///
        /// Defaults to a perceptually uniform color space called
        /// [Oklab]($color.oklab).
        #[named]
        #[default(ColorSpace::Oklab)]
        space: ColorSpace,

        /// The relative placement of the gradient.
        ///
        /// - `"this"`: The gradient is relative to the bounding box of the
        ///   container onto which it is painted.
        /// - `"parent"`: The gradient is relative to the bounding box of the
        ///   parent that contains the element onto which the gradient is applied.
        ///
        /// For an element placed at the root/top level of the document, the parent
        /// is the page itself. For other elements, the parent is the first block or
        /// box that contains the element.
        #[named]
        #[default(Smart::Auto)]
        relative: Smart<Relative>,
    ) -> SourceResult<Gradient> {
        if stops.len() < 2 {
            bail!(error!(span, "a gradient must have at least two stops")
                .with_hint("try filling the shape with a single color instead"));
        }

        let stops = process_stops(&stops)?;

        Ok(Self::Linear(LinearGradient {
            stops,
            angle: dir.into(),
            space,
            relative,
            anti_alias: true,
        }))
    }

    /// Return the stops of this gradient.
    #[func]
    pub fn stops(&self) -> Vec<Stop> {
        match self {
            Self::Linear(linear) => linear
                .stops
                .iter()
                .map(|(color, offset)| Stop { color: *color, offset: Some(*offset) })
                .collect(),
        }
    }

    /// Returns the mixing space of this gradient.
    #[func]
    pub fn space(&self) -> ColorSpace {
        match self {
            Self::Linear(linear) => linear.space,
        }
    }

    /// Returns the relative placement of this gradient.
    #[func]
    pub fn relative(&self) -> Smart<Relative> {
        match self {
            Self::Linear(linear) => linear.relative,
        }
    }

    /// Returns the direction of this gradient.
    #[func]
    pub fn dir(&self) -> Angle {
        match self {
            Self::Linear(linear) => linear.angle,
        }
    }

    /// Sample the gradient at a given position.
    ///
    /// The position is either the progress along the gradient (a number between
    /// 0 and 1) or an angle (in radians). Any value outside of this range will
    /// be clamped.
    #[func]
    pub fn sample(
        &self,

        /// The position at which to sample the gradient.
        t: RatioOrAngle,
    ) -> Color {
        let value: f64 = t.into();

        match self {
            Self::Linear(linear) => sample_stops(&linear.stops, linear.space, value),
        }
    }

    /// Create a sharp version of this gradient.
    ///
    /// ```example
    /// #let grad = gradient.linear(..gradient.rainbow(20))
    /// #rect(width: 100pt, height: 20pt, fill: grad)
    /// #rect(width: 100pt, height: 20pt, fill: grad.sharp(5))
    /// ```
    #[func]
    pub fn sharp(
        &self,

        /// The number of stops in the gradient.
        n: Spanned<usize>,

        /// How much to smooth the gradient.
        #[default(Spanned::new(Ratio::zero(), Span::detached()))]
        smoothness: Spanned<Ratio>,
    ) -> SourceResult<Gradient> {
        if n.v < 2 {
            bail!(n.span, "sharp gradients must have at least two stops");
        }

        if smoothness.v.get() < 0.0 || smoothness.v.get() > 1.0 {
            bail!(smoothness.span, "smoothness must be between 0 and 1");
        }

        let smoothness = smoothness.v.get();
        let colors = (0..n.v)
            .flat_map(|i| {
                let c = self
                    .sample(RatioOrAngle::Ratio(Ratio::new(i as f64 / (n.v - 1) as f64)));

                [c, c]
            })
            .collect::<Vec<_>>();

        let mut positions = Vec::with_capacity(n.v * 2);
        let p = |i| i as f64 * 1.0 / n.v as f64;

        let t = smoothness * 1.0 / (4.0 * n.v as f64);
        for i in 0..n.v {
            let mut j = 2 * i;
            positions.push(p(i));
            if j > 0 {
                positions[j] += t;
            }

            j += 1;
            positions.push(p(i + 1));
            if j < colors.len() - 1 {
                positions[j] -= t;
            }
        }

        let stops = colors
            .into_iter()
            .zip(positions)
            .map(|(c, p)| (c, Ratio::new(p)))
            .collect::<EcoVec<_>>();

        Ok(match self {
            Self::Linear(linear) => Self::Linear(LinearGradient {
                stops,
                angle: linear.angle,
                space: linear.space,
                relative: linear.relative,
                anti_alias: false,
            }),
        })
    }

    /// Repeat this gradient a given number of times, optionally mirroring it at
    /// each repetition.
    #[func]
    pub fn repeat(
        &self,

        /// The number of times to repeat the gradient.
        n: Spanned<usize>,

        /// Whether to mirror the gradient at each repetition.
        #[default(false)]
        mirror: bool,
    ) -> SourceResult<Gradient> {
        if n.v == 0 {
            bail!(n.span, "must repeat at least once");
        }

        let stops = std::iter::repeat(self.stops())
            .take(n.v)
            .enumerate()
            .flat_map(|(i, stops)| {
                let mut stops = stops
                    .iter()
                    .map(move |stop| {
                        let offset = i as f64 / n.v as f64;
                        let r = stop.offset.unwrap();
                        if i % 2 == 1 && mirror {
                            (
                                stop.color,
                                Ratio::new(offset + (1.0 - r.get()) / n.v as f64),
                            )
                        } else {
                            (stop.color, Ratio::new(offset + r.get() / n.v as f64))
                        }
                    })
                    .collect::<Vec<_>>();

                if i % 2 == 1 && mirror {
                    stops.reverse();
                }

                stops
            })
            .collect::<EcoVec<_>>();

        Ok(match self {
            Self::Linear(grad) => Self::Linear(LinearGradient {
                stops,
                angle: grad.angle,
                space: grad.space,
                relative: grad.relative,
                anti_alias: true,
            }),
        })
    }
}

impl Gradient {
    pub fn sample_at(&self, (x, y): (f32, f32), (width, height): (f32, f32)) -> Color {
        let t = match self {
            Self::Linear(linear) => {
                // normalize the coordinates
                let (mut x, mut y) = (x / width, y / height);

                // Handle the direction of the gradient
                let angle = (linear.angle.to_rad()).rem_euclid(TAU);
                let (sin, cos) = angle.sin_cos();

                let length = sin.abs() + cos.abs();
                if angle > FRAC_PI_2 && angle < 3.0 * FRAC_PI_2 {
                    x = 1.0 - x;
                }

                if angle > PI {
                    y = 1.0 - y;
                }

                (x as f64 * cos.abs() + y as f64 * sin.abs()) / length
            }
        };

        self.sample(RatioOrAngle::Ratio(Ratio::new(t)))
    }

    pub fn anti_alias(&self) -> bool {
        match self {
            Self::Linear(linear) => linear.anti_alias
        }
    }
}

#[derive(Clone, Hash, PartialEq, Eq)]
pub struct LinearGradient {
    /// The color stops of this gradient
    stops: EcoVec<(Color, Ratio)>,

    /// The direction of this gradient
    angle: Angle,

    /// The color space in which to interpolate the gradient
    space: ColorSpace,

    /// The relative placement of the gradient
    relative: Smart<Relative>,

    /// Whether to anti-alias the gradient (used for sharp gradient)
    anti_alias: bool,
}

impl Debug for LinearGradient {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("gradient.linear(")?;

        let angle = self.angle.to_rad().rem_euclid(TAU);
        if angle.abs() < EPSILON {
            // Default value, do nothing
        } else if (angle - FRAC_PI_2).abs() < EPSILON {
            f.write_str("dir: rtl, ")?;
        } else if (angle - PI).abs() < EPSILON {
            f.write_str("dir: ttb, ")?;
        } else if (angle - 3.0 * FRAC_PI_2).abs() < EPSILON {
            f.write_str("dir: btt, ")?;
        } else {
            write!(f, "angle: {:?}, ", self.angle)?;
        }

        if self.space != ColorSpace::Oklab {
            write!(f, "space: {:?}, ", self.space.into_value())?;
        }

        if self.relative.is_custom() {
            write!(f, "relative: {:?}, ", self.relative.into_value())?;
        }

        for (i, (color, offset)) in self.stops.iter().enumerate() {
            write!(f, "({color:?}, {offset:?})")?;

            if i != self.stops.len() - 1 {
                f.write_str(", ")?;
            }
        }

        f.write_char(')')
    }
}

/// What is the gradient relative to:
/// - Itself (its own bounding box)
/// - Its parent (the parent's bounding box)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Relative {
    This,
    Parent,
}

cast! {
    Relative,
    self => match self {
        Self::This => "this".into_value(),
        Self::Parent => "parent".into_value(),
    },
    "self" => Self::This,
    "parent" => Self::Parent,
}

/// A color stop.
#[derive(Copy, Clone, PartialEq, Eq, Hash)]
pub struct Stop {
    pub color: Color,
    pub offset: Option<Ratio>,
}

cast! {
    Stop,
    self => if let Some(offset) = self.offset {
        array![ self.color, offset ].into_value()
    } else {
        self.color.into_value()
    },
    color: Color => Self { color, offset: None },
    array: Array => {
        let mut iter = array.into_iter();
        match (iter.next(), iter.next(), iter.next()) {
            (Some(a), Some(b), None) => Self {
                color: a.cast()?,
                offset: b.cast()?
            },
            _ => Err("a color stop must contain exactly two entries")?,
        }
    }
}

/// A direction or an angle.
#[derive(Copy, Clone, PartialEq, Eq, Hash)]
pub enum DirOrAngle {
    Dir(Dir),
    Angle(Angle),
}

cast! {
    DirOrAngle,
    self => match self {
        Self::Dir(dir) => dir.into_value(),
        Self::Angle(angle) => angle.into_value(),
    },
    dir: Dir => Self::Dir(dir),
    angle: Angle => Self::Angle(angle),
}

impl From<DirOrAngle> for Angle {
    fn from(value: DirOrAngle) -> Self {
        match value {
            DirOrAngle::Dir(dir) => match dir {
                Dir::LTR => Angle::zero(),
                Dir::RTL => Angle::rad(FRAC_PI_2),
                Dir::TTB => Angle::rad(PI),
                Dir::BTT => Angle::rad(3.0 * FRAC_PI_2),
            },
            DirOrAngle::Angle(angle) => angle,
        }
    }
}

/// A ratio or an angle.
#[derive(Copy, Clone, PartialEq, Eq, Hash)]
pub enum RatioOrAngle {
    Ratio(Ratio),
    Angle(Angle),
}

cast! {
    RatioOrAngle,
    self => match self {
        Self::Ratio(ratio) => ratio.into_value(),
        Self::Angle(angle) => angle.into_value(),
    },
    ratio: Ratio => Self::Ratio(ratio),
    angle: Angle => Self::Angle(angle),
}

impl From<RatioOrAngle> for f64 {
    fn from(value: RatioOrAngle) -> Self {
        match value {
            RatioOrAngle::Ratio(ratio) => ratio.get(),
            RatioOrAngle::Angle(angle) => angle.to_rad().rem_euclid(TAU) / TAU,
        }
        .clamp(0.0, 1.0)
    }
}

fn process_stops(stops: &[Spanned<Stop>]) -> SourceResult<EcoVec<(Color, Ratio)>> {
    let has_offset = stops.iter().any(|stop| stop.v.offset.is_some());
    if has_offset {
        let mut last_stop = NEG_INFINITY;
        for Spanned { v: stop, span } in stops.iter() {
            let Some(stop) = stop.offset else {
                bail!(error!(
                    *span,
                    "either all stops must have an offset or none of them can"
                )
                .with_hint("try adding an offset to all stops"));
            };

            if stop.get() < last_stop {
                bail!(*span, "offsets must be in stricly monotonic");
            }

            last_stop = stop.get();
        }

        return stops
            .iter()
            .map(|Spanned { v: Stop { color, offset }, span }| {
                if offset.unwrap().get() > 1.0 || offset.unwrap().get() < 0.0 {
                    bail!(*span, "offset must be between 0 and 1");
                }
                Ok((*color, offset.unwrap()))
            })
            .collect::<SourceResult<EcoVec<_>>>();
    }

    Ok(stops
        .iter()
        .enumerate()
        .map(|(i, stop)| {
            let offset = i as f64 / (stops.len() - 1) as f64;
            (stop.v.color, Ratio::new(offset))
        })
        .collect())
}

fn sample_stops(stops: &[(Color, Ratio)], mixing_space: ColorSpace, t: f64) -> Color {
    let t = t.clamp(0.0, 1.0);
    let mut low = 0;
    let mut high = stops.len();

    while low < high {
        let mid = (low + high) / 2;
        if stops[mid].1.get() < t {
            low = mid + 1;
        } else {
            high = mid;
        }
    }

    if low == 0 {
        low = 1;
    }
    let (col_0, pos_0) = stops[low - 1];
    let (col_1, pos_1) = stops[low];
    let t = (t - pos_0.get()) / (pos_1.get() - pos_0.get());

    Color::mix(
        vec![WeightedColor::new(col_0, 1.0 - t), WeightedColor::new(col_1, t)],
        mixing_space,
    )
    .unwrap()
}
