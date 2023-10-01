use std::f64::consts::{FRAC_PI_2, PI, TAU};
use std::f64::{EPSILON, NEG_INFINITY};
use std::fmt::{Debug, Write};
use std::hash::Hash;

use ecow::EcoVec;
use typst_macros::{cast, func, scope, ty};
use typst_syntax::{Span, Spanned};

use super::color::Rgba;
use super::*;
use crate::diag::{bail, error, SourceResult};
use crate::eval::{array, Array, Func, IntoValue};
use crate::geom::{ColorSpace, Smart};

/// A color gradient.
///
/// Typst supports:
/// - Linear gradients through the [`gradient.linear` function]($gradient.linear)
/// - 🚧 Radial gradient will soon™️ be available
/// - 🚧 Conic gradient will soon™️ be available
///
/// ## Stops
///
/// A gradient is composed of a series of stops, each stop has a color and an offset.
/// The offset is a [ratio]($ratio) between 0% and 100% that determines the position
/// of the stop along the gradient. The color is the color of the gradient at that
/// position. In Typst, you can choose to omit the offset, in which case it will be
/// automatically computed for you, and all the stops will be evenly spaced.
///
/// ## Usage
///
/// Gradients can be used for the following purposes:
/// - As fills to paint the interior of a shape: `rect(fill: gradient.linear(..))`
/// - As strokes to paint the outline of a shape: `rect(stroke: 1pt + gradient.linear(..))`
/// - As color maps you can [sample]($gradient.sample) from:
///   `gradient.linear(..).sample(0.5)`
///
/// ## 🚧 Gradients on text
///
/// Currently gradients are not supported on text. However, in an upcoming release,
/// gradients will be supported on text.
///
/// ## Relativeness
///
/// Gradients can be relative to either the shape they are painted on, or to the
/// nearest parent containers. This is controlled by the `relative` argument of the
/// constructors. By default, gradients are relative to the shape they are painted on,
/// unless they are painted on text, in which case they are relative to the parent.
///
/// The way the parent is determined is as follows:
/// - For shapes that are placed at the root/top level of the document, the parent
///   is the page itself.
/// - For other shapes, the parent is the innermost [`block`]($block) or [`box`]($box)
///   that contains the shape. This includes the boxes and blocks that are implicitly
///   created by show rules. For example, a [`rotate`]($rotate) will not affect the
///   parent of a gradient, but a [`grid`]($grid) will.
///
/// ## Color spaces and interpolation
///
/// Gradients can be interpolated in any color space. By default, gradients are
/// interpolated in the [Oklab]($color.oklab) color space, which is a perceptually
/// uniform color space. This means that the gradient will be perceived as having
/// a uniform progression of colors. This is particularly useful for data
/// visualization.
///
/// However, you can choose to interpolate the gradient in any supported color
/// space you want, but beware that some color spaces are not suitable for
/// perceptually interpolating between colors. Below you can find a list of
/// color spaces and whether they are suitable for perceptual interpolation.
///
/// |           Color space           | Perceptually uniform? |
/// | ------------------------------- | --------------------- |
/// |      [Oklab]($color.oklab)      |           ✅          |
/// |      [sRGB]($color.rgb)         |           ❌          |
/// | [linear-RGB]($color.linear-rgb) |           ✅          |
/// |      [CMYK]($color.cmyk)        |           ❌          |
/// |     [Grayscale]($color.luma)    |           ✅          |
/// |       [HSL]($color.hsl)         |           ❌          |
/// |       [HSV]($color.hsv)         |           ❌          |
///
/// ```example
/// #set text(fill: white)
/// #set block(spacing: 0pt)
///
/// #let spaces = (
///   ("Oklab", color.oklab),
///   ("sRGB", color.rgb),
///   ("linear-RGB", color.linear-rgb),
///   ("CMYK", color.cmyk),
///   ("Grayscale", color.luma),
///   ("HSL", color.hsl),
///   ("HSV", color.hsv),
/// )
///
/// #for space in spaces {
///   block(
///     width: 100%,
///     height: 10pt,
///     fill: gradient.linear(red, blue, space: space.at(1))
///   )[
///     #space.at(0)
///   ]
/// }
/// ```
///
/// ## Direction
///
/// Some gradients are sensitive to the direction of the gradient. For example, a
/// linear gradient has an angle that determines the direction of the gradient. Instead
/// of the traditional clockwise angle, Typst uses an anti-clockwise angle, with 0° being
/// from left-to-right, 90° from top-to-bottom, 180° from right-to-left, and 270° from
/// bottom-to-top.
///
/// ```example
/// #set block(spacing: 0pt)
/// #stack(
///   dir: ltr,
///   square(size: 50pt, fill: gradient.linear(red, blue, dir: 0deg)),
///   square(size: 50pt, fill: gradient.linear(red, blue, dir: 90deg)),
///   square(size: 50pt, fill: gradient.linear(red, blue, dir: 180deg)),
///   square(size: 50pt, fill: gradient.linear(red, blue, dir: 270deg)),
/// )
/// ```
///
/// ## Note on compatibility
///
/// Typst's gradients were designed to be widely compatible, however, in
/// [PDF.js](https://mozilla.github.io/pdf.js/), the reader bundled with Firefox,
/// gradients in `rotate` blocks may not be rendered correctly. This is a bug in
/// PDF.js and not in Typst. Despite this, every type of gradient has been
/// tested in every major PDF reader, and should work in most browsers as
/// expected.
///
/// ## Presets
///
/// Typst also includes a number of preset color maps. In the following section the
/// list of available presets is given, along with a sample of the gradient and
/// relevant comments. Most of these color maps are chosen to be color blind friendly.
///
/// ### Turbo
///
/// The [`turbo`]($gradient.turbo) gradient is a rainbow-like gradient that is
/// perceptually uniform. Turbo is a gradient that takes an optional number of
/// stops, by default it is set to 20.
///
/// ✅ This gradient is suitable for data visualization.
///
/// ```example
/// #rect(width: 100pt, height: 20pt, fill: gradient.linear(..gradient.turbo(10)))
/// ```
///
/// ### Cividis
///
/// The [`cividis`]($gradient.cividis) gradient is a blue to gray to
/// yellow gradient that is perceptually uniform. Cividis is a gradient
/// that takes an optional number of stops, by default it is set to 20.
///
/// ✅ This gradient is suitable for data visualization.
///
/// ```example
/// #rect(width: 100pt, height: 20pt, fill: gradient.linear(..gradient.cividis(10)))
/// ```
///
/// ### Rainbow
///
/// The [`rainbow`]($gradient.rainbow) gradient is a rainbow gradient that is
/// **not** perceptually uniform, therefore it should only be used for decorative
/// purposes and not for data visualization. Rainbow is a gradient that takes an
/// optional number of stops, by default it is set to 20. This gradient is best
/// used by setting the interpolation color space to [HSL]($color.hsl).
///
/// ❌ This gradient is **not** suitable for data visualization.
///
/// ```example
/// #rect(
///   width: 100pt,
///   height: 20pt,
///   fill: gradient.linear(..gradient.rainbow(10), space: color.hsl)
/// )
/// ```
///
/// ### Spectral
///
/// The [`spectral`]($gradient.spectral) gradient is a red to yellow to blue
/// gradient that is perceptually uniform. Spectral does not take any parameters.
///
/// ✅ This gradient is suitable for data visualization.
///
/// ```example
/// #rect(width: 100pt, height: 20pt, fill: gradient.linear(..gradient.spectral))
/// ```
///
/// ### Viridis
///
/// The [`viridis`]($gradient.viridis) gradient is a purple to teal to yellow
/// gradient that is perceptually uniform. Viridis does not take any parameters.
///
/// ✅ This gradient is suitable for data visualization.
///
/// ```example
/// #rect(width: 100pt, height: 20pt, fill: gradient.linear(..gradient.viridis))
/// ```
///
/// ### Inferno
///
/// The [`inferno`]($gradient.inferno) gradient is a black to red to yellow
/// gradient that is perceptually uniform. Inferno does not take any parameters.
///
/// ✅ This gradient is suitable for data visualization.
///
/// ```example
/// #rect(width: 100pt, height: 20pt, fill: gradient.linear(..gradient.inferno))
/// ```
///
/// ### Magma
///
/// The [`magma`]($gradient.magma) gradient is a black to purple to yellow
/// gradient that is perceptually uniform. Magma does not take any parameters.
///
/// ✅ This gradient is suitable for data visualization.
///
/// ```example
/// #rect(width: 100pt, height: 20pt, fill: gradient.linear(..gradient.magma))
/// ```
///
/// ### Plasma
///
/// The [`plasma`]($gradient.plasma) gradient is a purple to pink to yellow
/// gradient that is perceptually uniform. Plasma does not take any parameters.
///
/// ✅ This gradient is suitable for data visualization.
///
/// ```example
/// #rect(width: 100pt, height: 20pt, fill: gradient.linear(..gradient.plasma))
/// ```
///
/// ### Rocket
///
/// The [`rocket`]($gradient.rocket) gradient is a black to red to white
/// gradient that is perceptually uniform. Rocket does not take any parameters.
///
/// ✅ This gradient is suitable for data visualization.
///
/// ```example
/// #rect(width: 100pt, height: 20pt, fill: gradient.linear(..gradient.rocket))
/// ```
///
/// ### Mako
///
/// The [`mako`]($gradient.mako) gradient is a black to teal to yellow
/// gradient that is perceptually uniform. Mako does not take any parameters.
///
/// ✅ This gradient is suitable for data visualization.
///
/// ```example
/// #rect(width: 100pt, height: 20pt, fill: gradient.linear(..gradient.mako))
/// ```
///
/// ### Vlag
///
/// The [`vlag`]($gradient.vlag) gradient is a light blue to white to red
/// gradient that is perceptually uniform. Vlag does not take any parameters.
///
/// ✅ This gradient is suitable for data visualization.
///
/// ```example
/// #rect(width: 100pt, height: 20pt, fill: gradient.linear(..gradient.vlag))
/// ```
///
/// ### Icefire
///
/// The [`icefire`]($gradient.icefire) gradient is a light teal to black to yellow
/// gradient that is perceptually uniform. Icefire does not take any parameters.
///
/// ✅ This gradient is suitable for data visualization.
///
/// ```example
/// #rect(width: 100pt, height: 20pt, fill: gradient.linear(..gradient.icefire))
/// ```
///
/// ### Flare
///
/// The [`flare`]($gradient.flare) gradient is an orange to purple gradient
/// that is perceptually uniform. Flare does not take any parameters.
///
/// ✅ This gradient is suitable for data visualization.
///
/// ```example
/// #rect(width: 100pt, height: 20pt, fill: gradient.linear(..gradient.flare))
/// ```
///
/// ### Crest
///
/// The [`crest`]($gradient.crest) gradient is a blue to white to red gradient
/// that is perceptually uniform. Crest does not take any parameters.
///
/// ✅ This gradient is suitable for data visualization.
///
/// ```example
/// #rect(width: 100pt, height: 20pt, fill: gradient.linear(..gradient.crest))
/// ```
///
/// ### What about presets like "jet" and "parula"
///
/// - [Jet](https://jakevdp.github.io/blog/2014/10/16/how-bad-is-your-colormap/)
///   is not a good color map, as it is not perceptually uniform. As such,
///   it is not color blind friendly and should not be used for data visualization,
///   due to which it is not included in Typst.
/// - [Parula](https://www.mathworks.com/help/matlab/ref/parula.html)
///   is a good color map included in matlab, but it is not included as
///   a preset in Typst. This is because it is owned by MathWorks and is not public.
///
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
    pub const SPECTRAL: fn() -> Array = spectral;
    pub const VIRIDIS: fn() -> Array = viridis;
    pub const INFERNO: fn() -> Array = inferno;
    pub const MAGMA: fn() -> Array = magma;
    pub const PLASMA: fn() -> Array = plasma;
    pub const ROCKET: fn() -> Array = rocket;
    pub const MAKO: fn() -> Array = mako;
    pub const VLAG: fn() -> Array = vlag;
    pub const ICEFIRE: fn() -> Array = icefire;
    pub const FLARE: fn() -> Array = flare;
    pub const CREST: fn() -> Array = crest;

    /// Creates a new linear gradient.
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

    /// Returns the kind of this gradient.
    #[func]
    pub fn kind(&self) -> Func {
        match self {
            Self::Linear(_) => Self::linear_data().into(),
        }
    }

    /// Sample the gradient at a given position.
    ///
    /// The position is either the progress along the gradient (a [ratio]($ratio)
    /// between 0% and 100%) or an [angle]($angle). Any value outside
    /// of this range will be clamped.
    ///
    /// 🚧 The angle will be mainly used for conic gradients once they are ready.
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

    /// Sample the gradient at the given positions.
    ///
    /// The positions are either the progress along the gradient (a [ratio]($ratio)
    /// between 0% and 100%) or an [angle]($angle). Any value outside
    /// of this range will be clamped.
    ///
    /// 🚧 Angles will be mainly used for conic gradients once they are ready.
    #[func]
    pub fn samples(
        &self,

        /// The positions at which to sample the gradient.
        #[variadic]
        ts: Vec<RatioOrAngle>,
    ) -> Array {
        Array::from(
            ts.into_iter()
                .map(|t| self.sample(t).into_value())
                .collect::<EcoVec<_>>(),
        )
    }

    /// Create a sharp version of this gradient.
    ///
    /// This is particularly useful for creating color lists that come from
    /// a preset gradient.
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
        #[named]
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

        let mut stops = colors
            .into_iter()
            .zip(positions)
            .map(|(c, p)| (c, Ratio::new(p)))
            .collect::<Vec<_>>();

        stops.dedup();

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
        #[named]
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
            .collect::<Vec<_>>();

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

    /// Creates a [turbo] stop list.
    ///
    /// You can control the number of stops in the gradient using the `stops` parameter, by default it is set to 20.
    ///
    /// ```example
    /// #rect(width: 100pt, height: 20pt, fill: gradient.linear(..gradient.turbo(10)))
    /// ````
    ///
    /// [turbo]: https://ai.googleblog.com/2019/08/turbo-improved-rainbow-colormap-for.html
    #[func]
    fn turbo(
        #[default(Spanned::new(20, Span::detached()))] stops: Spanned<i64>,
    ) -> SourceResult<Value> {
        fn at(t: f32) -> Rgba {
            let t = t.clamp(0.0, 1.0);
            let r = (34.61
                + t * (1172.33
                    - t * (10793.56 - t * (33300.12 - t * (38394.49 - t * 14825.05)))))
                .round();
            let g = (23.31
                + t * (557.33
                    + t * (1225.33 - t * (3574.96 - t * (1073.77 + t * 707.56)))))
                .round();
            let b = (27.2
                + t * (3211.1
                    - t * (15327.97 - t * (27814.0 - t * (22569.18 - t * 6838.66)))))
                .round();

            Rgba::new(r / 255.0, g / 255.0, b / 255.0, 1.0)
        }

        if stops.v < 2 {
            bail!(stops.span, "number of stops must be bigger or equal to 2");
        }

        Ok(Array::from(
            (0..stops.v)
                .map(|i| {
                    let t = i as f64 / (stops.v - 1) as f64;
                    Stop::new(Color::Rgba(at(t as f32)), t).into_value()
                })
                .collect::<EcoVec<_>>(),
        )
        .into_value())
    }

    /// Creates a [cividis] stop list.
    ///
    /// You can control the number of stops in the gradient using the `stops` parameter, by default it is set to 20.
    ///
    /// ```example
    /// #rect(width: 100pt, height: 20pt, fill: gradient.linear(..gradient.cividis(10)))
    /// ````
    ///
    /// [cividis]: https://bids.github.io/colormap/
    #[func]
    fn cividis(
        #[default(Spanned::new(20, Span::detached()))] stops: Spanned<i64>,
    ) -> SourceResult<Value> {
        fn at(t: f32) -> Rgba {
            let t = t.clamp(0.0, 1.0);
            let r = -4.54
                - t * (35.34
                    - t * (2381.73 - t * (6402.7 - t * (7024.72 - t * 2710.57))));
            let g = 32.49
                + t * (170.73 + t * (52.82 - t * (131.46 - t * (176.58 - t * 67.37))));
            let b = 81.24
                + t * (442.36
                    - t * (2482.43 - t * (6167.24 - t * (6614.94 - t * 2475.67))));

            Rgba::new(r / 255.0, g / 255.0, b / 255.0, 1.0)
        }

        if stops.v < 2 {
            bail!(stops.span, "number of stops must be bigger or equal to 2");
        }

        Ok(Array::from(
            (0..stops.v)
                .map(|i| {
                    let t = i as f64 / (stops.v - 1) as f64;
                    Stop::new(Color::Rgba(at(t as f32)), t).into_value()
                })
                .collect::<EcoVec<_>>(),
        )
        .into_value())
    }

    /// Creates a list of rainbow color stops with the given parameters.
    ///
    /// You can control the number of stops in the gradient using the `stops`
    /// parameter, which is set to 20 by default.
    ///
    /// This gradient is best used by setting the interpolation color space to
    /// [HSL]($color.hsl). It should also be noted that this is not a good
    /// choice for a color scale, as it is not perceptually uniform. This preset
    /// is more intended for decorative purposes than for data visualization.
    ///
    /// ```example
    /// #rect(width: 100pt, height: 20pt, fill: gradient.linear(..gradient.rainbow(2)))
    /// ````
    #[func]
    fn rainbow(
        #[default(Spanned::new(20, Span::detached()))] stops: Spanned<i64>,
    ) -> SourceResult<Array> {
        if stops.v < 2 {
            bail!(stops.span, "number of stops must be bigger or equal to 2");
        }

        Ok((0..stops.v)
            .map(|i| {
                let t = i as f32 / (stops.v - 1) as f32;
                let ts = (t - 0.5).abs();

                Stop::new(
                    cubehelix_to_rgb(360.0 * t - 100.0, 1.5 - 1.5 * ts, 0.8 - 0.8 * ts),
                    t as f64,
                )
                .into_value()
            })
            .collect())
    }
}

impl Gradient {
    pub fn sample_at(&self, (x, y): (f32, f32), (width, height): (f32, f32)) -> Color {
        let t = match self {
            Self::Linear(linear) => {
                // Normalize the coordinates.
                let (mut x, mut y) = (x / width, y / height);

                // Handle the direction of the gradient.
                let angle = linear.angle.to_rad().rem_euclid(TAU);

                // Aspect ratio correction.
                let angle = (angle.tan() * height as f64).atan2(width as f64);
                let angle = match linear.angle.quadrant() {
                    Quadrant::First => angle,
                    Quadrant::Second => angle + PI,
                    Quadrant::Third => angle + PI,
                    Quadrant::Fourth => angle + TAU,
                };

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
            Self::Linear(linear) => linear.anti_alias,
        }
    }

    /// Returns the relative placement of this gradient, handling
    /// the special case of `Auto`.
    pub fn unwrap_relative(&self, on_text: bool) -> Relative {
        self.relative().unwrap_or_else(|| {
            if on_text {
                Relative::Parent
            } else {
                Relative::This
            }
        })
    }
}

#[derive(Clone, Hash, PartialEq, Eq)]
pub struct LinearGradient {
    /// The color stops of this gradient.
    pub stops: Vec<(Color, Ratio)>,

    /// The direction of this gradient.
    pub angle: Angle,

    /// The color space in which to interpolate the gradient.
    pub space: ColorSpace,

    /// The relative placement of the gradient.
    pub relative: Smart<Relative>,

    /// Whether to anti-alias the gradient (used for sharp gradients).
    pub anti_alias: bool,
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

/// What is the gradient relative to.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Relative {
    /// The gradient is relative to itself (its own bounding box).
    This,

    /// The gradient is relative to its parent (the parent's bounding box).
    Parent,
}

cast! {
    Relative,
    self => match self {
        Self::This => "self".into_value(),
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

impl Stop {
    pub fn new(color: Color, offset: f64) -> Self {
        Self { color, offset: Some(Ratio::new(offset)) }
    }
}

cast! {
    Stop,
    self => if let Some(offset) = self.offset {
        array![ self.color.into_value(), offset ].into_value()
    } else {
        self.color.into_value()
    },
    color: Color => Self { color, offset: None },
    array: Array => {
        let mut iter = array.into_iter();
        match (iter.next(), iter.next(), iter.next()) {
            (Some(a), Some(b), None) => Self {
                color: a.cast()?,
                offset: Some(b.cast()?)
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
                Dir::RTL => Angle::rad(PI),
                Dir::TTB => Angle::rad(FRAC_PI_2),
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

fn process_stops(stops: &[Spanned<Stop>]) -> SourceResult<Vec<(Color, Ratio)>> {
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
            .collect::<SourceResult<Vec<_>>>();
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

fn cubehelix_to_rgb(h: f32, s: f32, l: f32) -> Color {
    let h = (h + 120.0).to_radians();
    let l = l;
    let a = s * l * (1.0 - l);

    let (sinh, cosh) = h.sin_cos();

    let r = l - a * (0.14861 * cosh - 1.78277 * sinh).min(1.0);
    let g = l - a * (0.29227 * cosh + 0.90649 * sinh).min(1.0);
    let b = l + a * (1.97294 * cosh);

    Color::Rgba(Rgba::new(r.clamp(0.0, 1.0), g.clamp(0.0, 1.0), b.clamp(0.0, 1.0), 1.0))
}

macro_rules! preset {
    ($name:ident; $($colors:literal),* $(,)*) => {
        #[comemo::memoize]
        fn $name() -> Array {
            let colors = [$(Color::from_u32($colors)),*];
            Array::from(
                colors
                    .iter()
                    .enumerate()
                    .map(|(i, c)| Stop::new(*c, i as f64 / (colors.len() - 1) as f64).into_value())
                    .collect::<EcoVec<_>>()
            )
        }
    };
}

preset!(spectral; 0x9e0142ff, 0xd53e4fff, 0xf46d43ff, 0xfdae61ff, 0xfee08bff, 0xffffbfff, 0xe6f598ff, 0xabdda4ff, 0x66c2a5ff, 0x3288bdff, 0x5e4fa2ff);
preset!(viridis; 0x440154ff, 0x482777ff, 0x3f4a8aff, 0x31678eff, 0x26838fff, 0x1f9d8aff, 0x6cce5aff, 0xb6de2bff, 0xfee825ff);
preset!(inferno; 0x000004ff, 0x170b3aff, 0x420a68ff, 0x6b176eff, 0x932667ff, 0xbb3654ff, 0xdd513aff, 0xf3771aff, 0xfca50aff, 0xf6d644ff, 0xfcffa4ff);
preset!(magma; 0x000004ff, 0x140e37ff, 0x3b0f70ff, 0x641a80ff, 0x8c2981ff, 0xb63679ff, 0xde4968ff, 0xf66f5cff, 0xfe9f6dff, 0xfece91ff, 0xfcfdbfff);
preset!(plasma; 0x0d0887ff, 0x42039dff, 0x6a00a8ff, 0x900da3ff, 0xb12a90ff, 0xcb4678ff, 0xe16462ff, 0xf1834bff, 0xfca636ff, 0xfccd25ff, 0xf0f921ff);
preset!(rocket; 0x3051aff, 0x4051aff, 0x5061bff, 0x6071cff, 0x7071dff, 0x8081eff, 0xa091fff, 0xb0920ff, 0xd0a21ff, 0xe0b22ff, 0x100b23ff, 0x110c24ff, 0x130d25ff, 0x140e26ff, 0x160e27ff, 0x170f28ff, 0x180f29ff, 0x1a102aff, 0x1b112bff, 0x1d112cff, 0x1e122dff, 0x20122eff, 0x211330ff, 0x221331ff, 0x241432ff, 0x251433ff, 0x271534ff, 0x281535ff, 0x2a1636ff, 0x2b1637ff, 0x2d1738ff, 0x2e1739ff, 0x30173aff, 0x31183bff, 0x33183cff, 0x34193dff, 0x35193eff, 0x37193fff, 0x381a40ff, 0x3a1a41ff, 0x3c1a42ff, 0x3d1a42ff, 0x3f1b43ff, 0x401b44ff, 0x421b45ff, 0x431c46ff, 0x451c47ff, 0x461c48ff, 0x481c48ff, 0x491d49ff, 0x4b1d4aff, 0x4c1d4bff, 0x4e1d4bff, 0x501d4cff, 0x511e4dff, 0x531e4dff, 0x541e4eff, 0x561e4fff, 0x581e4fff, 0x591e50ff, 0x5b1e51ff, 0x5c1e51ff, 0x5e1f52ff, 0x601f52ff, 0x611f53ff, 0x631f53ff, 0x641f54ff, 0x661f54ff, 0x681f55ff, 0x691f55ff, 0x6b1f56ff, 0x6d1f56ff, 0x6e1f57ff, 0x701f57ff, 0x711f57ff, 0x731f58ff, 0x751f58ff, 0x761f58ff, 0x781f59ff, 0x7a1f59ff, 0x7b1f59ff, 0x7d1f5aff, 0x7f1e5aff, 0x811e5aff, 0x821e5aff, 0x841e5aff, 0x861e5bff, 0x871e5bff, 0x891e5bff, 0x8b1d5bff, 0x8c1d5bff, 0x8e1d5bff, 0x901d5bff, 0x921c5bff, 0x931c5bff, 0x951c5bff, 0x971c5bff, 0x981b5bff, 0x9a1b5bff, 0x9c1b5bff, 0x9e1a5bff, 0x9f1a5bff, 0xa11a5bff, 0xa3195bff, 0xa4195bff, 0xa6195aff, 0xa8185aff, 0xaa185aff, 0xab185aff, 0xad1759ff, 0xaf1759ff, 0xb01759ff, 0xb21758ff, 0xb41658ff, 0xb51657ff, 0xb71657ff, 0xb91657ff, 0xba1656ff, 0xbc1656ff, 0xbd1655ff, 0xbf1654ff, 0xc11754ff, 0xc21753ff, 0xc41753ff, 0xc51852ff, 0xc71951ff, 0xc81951ff, 0xca1a50ff, 0xcb1b4fff, 0xcd1c4eff, 0xce1d4eff, 0xcf1e4dff, 0xd11f4cff, 0xd2204cff, 0xd3214bff, 0xd5224aff, 0xd62449ff, 0xd72549ff, 0xd82748ff, 0xd92847ff, 0xdb2946ff, 0xdc2b46ff, 0xdd2c45ff, 0xde2e44ff, 0xdf2f44ff, 0xe03143ff, 0xe13342ff, 0xe23442ff, 0xe33641ff, 0xe43841ff, 0xe53940ff, 0xe63b40ff, 0xe73d3fff, 0xe83f3fff, 0xe8403eff, 0xe9423eff, 0xea443eff, 0xeb463eff, 0xeb483eff, 0xec4a3eff, 0xec4c3eff, 0xed4e3eff, 0xed503eff, 0xee523fff, 0xee543fff, 0xef5640ff, 0xef5840ff, 0xef5a41ff, 0xf05c42ff, 0xf05e42ff, 0xf06043ff, 0xf16244ff, 0xf16445ff, 0xf16646ff, 0xf26747ff, 0xf26948ff, 0xf26b49ff, 0xf26d4bff, 0xf26f4cff, 0xf3714dff, 0xf3734eff, 0xf37450ff, 0xf37651ff, 0xf37852ff, 0xf47a54ff, 0xf47c55ff, 0xf47d57ff, 0xf47f58ff, 0xf4815aff, 0xf4835bff, 0xf4845dff, 0xf4865eff, 0xf58860ff, 0xf58a61ff, 0xf58b63ff, 0xf58d64ff, 0xf58f66ff, 0xf59067ff, 0xf59269ff, 0xf5946bff, 0xf5966cff, 0xf5976eff, 0xf59970ff, 0xf69b71ff, 0xf69c73ff, 0xf69e75ff, 0xf6a077ff, 0xf6a178ff, 0xf6a37aff, 0xf6a47cff, 0xf6a67eff, 0xf6a880ff, 0xf6a981ff, 0xf6ab83ff, 0xf6ad85ff, 0xf6ae87ff, 0xf6b089ff, 0xf6b18bff, 0xf6b38dff, 0xf6b48fff, 0xf6b691ff, 0xf6b893ff, 0xf6b995ff, 0xf6bb97ff, 0xf6bc99ff, 0xf6be9bff, 0xf6bf9dff, 0xf6c19fff, 0xf7c2a2ff, 0xf7c4a4ff, 0xf7c6a6ff, 0xf7c7a8ff, 0xf7c9aaff, 0xf7caacff, 0xf7ccafff, 0xf7cdb1ff, 0xf7cfb3ff, 0xf7d0b5ff, 0xf8d1b8ff, 0xf8d3baff, 0xf8d4bcff, 0xf8d6beff, 0xf8d7c0ff, 0xf8d9c3ff, 0xf8dac5ff, 0xf8dcc7ff, 0xf9ddc9ff, 0xf9dfcbff, 0xf9e0cdff, 0xf9e2d0ff, 0xf9e3d2ff, 0xf9e5d4ff, 0xfae6d6ff, 0xfae8d8ff, 0xfae9daff, 0xfaebddff);
preset!(mako; 0xb0405ff, 0xd0406ff, 0xe0508ff, 0xf0609ff, 0x10060aff, 0x11070cff, 0x12080dff, 0x13090fff, 0x140910ff, 0x150a12ff, 0x160b13ff, 0x170c15ff, 0x180d16ff, 0x190e18ff, 0x1a0e19ff, 0x1b0f1aff, 0x1c101cff, 0x1d111dff, 0x1e111fff, 0x1f1220ff, 0x201322ff, 0x211423ff, 0x221425ff, 0x231526ff, 0x241628ff, 0x251729ff, 0x26172bff, 0x27182dff, 0x28192eff, 0x291930ff, 0x291a31ff, 0x2a1b33ff, 0x2b1c35ff, 0x2c1c36ff, 0x2d1d38ff, 0x2e1e39ff, 0x2e1e3bff, 0x2f1f3dff, 0x30203eff, 0x312140ff, 0x312142ff, 0x322243ff, 0x332345ff, 0x342447ff, 0x342548ff, 0x35254aff, 0x35264cff, 0x36274dff, 0x37284fff, 0x372851ff, 0x382953ff, 0x382a54ff, 0x392b56ff, 0x3a2c58ff, 0x3a2c59ff, 0x3b2d5bff, 0x3b2e5dff, 0x3b2f5fff, 0x3c3060ff, 0x3c3162ff, 0x3d3164ff, 0x3d3266ff, 0x3e3367ff, 0x3e3469ff, 0x3e356bff, 0x3f366dff, 0x3f366fff, 0x3f3770ff, 0x403872ff, 0x403974ff, 0x403a76ff, 0x403b78ff, 0x403c79ff, 0x413d7bff, 0x413e7dff, 0x413e7fff, 0x413f80ff, 0x414082ff, 0x414184ff, 0x414285ff, 0x414387ff, 0x414488ff, 0x40468aff, 0x40478bff, 0x40488dff, 0x40498eff, 0x3f4a8fff, 0x3f4b90ff, 0x3f4c92ff, 0x3e4d93ff, 0x3e4f94ff, 0x3e5095ff, 0x3d5195ff, 0x3d5296ff, 0x3c5397ff, 0x3c5598ff, 0x3b5698ff, 0x3b5799ff, 0x3b589aff, 0x3a599aff, 0x3a5b9bff, 0x3a5c9bff, 0x395d9cff, 0x395e9cff, 0x385f9cff, 0x38619dff, 0x38629dff, 0x38639dff, 0x37649eff, 0x37659eff, 0x37669eff, 0x37689fff, 0x36699fff, 0x366a9fff, 0x366b9fff, 0x366ca0ff, 0x366da0ff, 0x366fa0ff, 0x3670a0ff, 0x3671a0ff, 0x3572a1ff, 0x3573a1ff, 0x3574a1ff, 0x3575a1ff, 0x3576a2ff, 0x3578a2ff, 0x3579a2ff, 0x357aa2ff, 0x357ba3ff, 0x357ca3ff, 0x357da3ff, 0x357ea4ff, 0x347fa4ff, 0x3480a4ff, 0x3482a4ff, 0x3483a5ff, 0x3484a5ff, 0x3485a5ff, 0x3486a5ff, 0x3487a6ff, 0x3488a6ff, 0x3489a6ff, 0x348ba6ff, 0x348ca7ff, 0x348da7ff, 0x348ea7ff, 0x348fa7ff, 0x3490a8ff, 0x3491a8ff, 0x3492a8ff, 0x3493a8ff, 0x3495a9ff, 0x3496a9ff, 0x3497a9ff, 0x3498a9ff, 0x3499aaff, 0x349aaaff, 0x359baaff, 0x359caaff, 0x359eaaff, 0x359fabff, 0x35a0abff, 0x35a1abff, 0x36a2abff, 0x36a3abff, 0x36a4abff, 0x37a5acff, 0x37a6acff, 0x37a8acff, 0x38a9acff, 0x38aaacff, 0x39abacff, 0x39acacff, 0x3aadacff, 0x3aaeadff, 0x3bafadff, 0x3cb1adff, 0x3cb2adff, 0x3db3adff, 0x3eb4adff, 0x3fb5adff, 0x3fb6adff, 0x40b7adff, 0x41b8adff, 0x42b9adff, 0x43baadff, 0x44bcadff, 0x45bdadff, 0x46beadff, 0x47bfadff, 0x48c0adff, 0x49c1adff, 0x4bc2adff, 0x4cc3adff, 0x4dc4adff, 0x4fc5adff, 0x50c6adff, 0x52c7adff, 0x53c9adff, 0x55caadff, 0x57cbadff, 0x59ccadff, 0x5bcdadff, 0x5ecdadff, 0x60ceacff, 0x62cfacff, 0x65d0adff, 0x68d1adff, 0x6ad2adff, 0x6dd3adff, 0x70d4adff, 0x73d4adff, 0x76d5aeff, 0x79d6aeff, 0x7cd6afff, 0x7fd7afff, 0x82d8b0ff, 0x85d9b1ff, 0x88d9b1ff, 0x8bdab2ff, 0x8edbb3ff, 0x91dbb4ff, 0x94dcb5ff, 0x96ddb5ff, 0x99ddb6ff, 0x9cdeb7ff, 0x9edfb8ff, 0xa1dfb9ff, 0xa4e0bbff, 0xa6e1bcff, 0xa9e1bdff, 0xabe2beff, 0xaee3c0ff, 0xb0e4c1ff, 0xb2e4c2ff, 0xb5e5c4ff, 0xb7e6c5ff, 0xb9e6c7ff, 0xbbe7c8ff, 0xbee8caff, 0xc0e9ccff, 0xc2e9cdff, 0xc4eacfff, 0xc6ebd1ff, 0xc8ecd2ff, 0xcaedd4ff, 0xccedd6ff, 0xceeed7ff, 0xd0efd9ff, 0xd2f0dbff, 0xd4f1dcff, 0xd6f1deff, 0xd8f2e0ff, 0xdaf3e1ff, 0xdcf4e3ff, 0xdef5e5ff);
preset!(vlag; 0x2369bdff, 0x266abdff, 0x296cbcff, 0x2c6dbcff, 0x2f6ebcff, 0x316fbcff, 0x3470bcff, 0x3671bcff, 0x3972bcff, 0x3b73bcff, 0x3d74bcff, 0x3f75bcff, 0x4276bcff, 0x4477bcff, 0x4678bcff, 0x4879bcff, 0x4a7bbcff, 0x4c7cbcff, 0x4e7dbcff, 0x507ebcff, 0x517fbcff, 0x5380bcff, 0x5581bcff, 0x5782bcff, 0x5983bdff, 0x5b84bdff, 0x5c85bdff, 0x5e86bdff, 0x6087bdff, 0x6288bdff, 0x6489beff, 0x658abeff, 0x678bbeff, 0x698cbeff, 0x6a8dbfff, 0x6c8ebfff, 0x6e90bfff, 0x6f91bfff, 0x7192c0ff, 0x7393c0ff, 0x7594c0ff, 0x7695c1ff, 0x7896c1ff, 0x7997c1ff, 0x7b98c2ff, 0x7d99c2ff, 0x7e9ac2ff, 0x809bc3ff, 0x829cc3ff, 0x839dc4ff, 0x859ec4ff, 0x87a0c4ff, 0x88a1c5ff, 0x8aa2c5ff, 0x8ba3c6ff, 0x8da4c6ff, 0x8fa5c7ff, 0x90a6c7ff, 0x92a7c8ff, 0x93a8c8ff, 0x95a9c8ff, 0x97abc9ff, 0x98acc9ff, 0x9aadcaff, 0x9baecbff, 0x9dafcbff, 0x9fb0ccff, 0xa0b1ccff, 0xa2b2cdff, 0xa3b4cdff, 0xa5b5ceff, 0xa7b6ceff, 0xa8b7cfff, 0xaab8d0ff, 0xabb9d0ff, 0xadbbd1ff, 0xafbcd1ff, 0xb0bdd2ff, 0xb2bed3ff, 0xb3bfd3ff, 0xb5c0d4ff, 0xb7c2d5ff, 0xb8c3d5ff, 0xbac4d6ff, 0xbbc5d7ff, 0xbdc6d7ff, 0xbfc8d8ff, 0xc0c9d9ff, 0xc2cadaff, 0xc3cbdaff, 0xc5cddbff, 0xc7cedcff, 0xc8cfddff, 0xcad0ddff, 0xcbd1deff, 0xcdd3dfff, 0xcfd4e0ff, 0xd0d5e0ff, 0xd2d7e1ff, 0xd4d8e2ff, 0xd5d9e3ff, 0xd7dae4ff, 0xd9dce5ff, 0xdadde5ff, 0xdcdee6ff, 0xdde0e7ff, 0xdfe1e8ff, 0xe1e2e9ff, 0xe2e3eaff, 0xe4e5ebff, 0xe6e6ecff, 0xe7e7ecff, 0xe9e9edff, 0xebeaeeff, 0xecebefff, 0xeeedf0ff, 0xefeef1ff, 0xf1eff2ff, 0xf2f0f2ff, 0xf3f1f3ff, 0xf5f2f4ff, 0xf6f3f4ff, 0xf7f4f4ff, 0xf8f4f5ff, 0xf9f5f5ff, 0xf9f5f5ff, 0xfaf5f5ff, 0xfaf5f5ff, 0xfaf5f4ff, 0xfaf5f4ff, 0xfaf4f3ff, 0xfaf3f3ff, 0xfaf3f2ff, 0xfaf2f1ff, 0xfaf0efff, 0xf9efeeff, 0xf9eeedff, 0xf8edebff, 0xf7ebeaff, 0xf7eae8ff, 0xf6e8e7ff, 0xf5e7e5ff, 0xf5e5e4ff, 0xf4e3e2ff, 0xf3e2e0ff, 0xf2e0dfff, 0xf2dfddff, 0xf1dddbff, 0xf0dbdaff, 0xefdad8ff, 0xefd8d6ff, 0xeed7d5ff, 0xedd5d3ff, 0xecd3d2ff, 0xecd2d0ff, 0xebd0ceff, 0xeacfcdff, 0xeacdcbff, 0xe9cbc9ff, 0xe8cac8ff, 0xe7c8c6ff, 0xe7c7c5ff, 0xe6c5c3ff, 0xe5c3c1ff, 0xe5c2c0ff, 0xe4c0beff, 0xe3bfbdff, 0xe3bdbbff, 0xe2bcb9ff, 0xe1bab8ff, 0xe1b9b6ff, 0xe0b7b5ff, 0xdfb5b3ff, 0xdfb4b2ff, 0xdeb2b0ff, 0xdeb1aeff, 0xddafadff, 0xdcaeabff, 0xdcacaaff, 0xdbaba8ff, 0xdaa9a7ff, 0xdaa8a5ff, 0xd9a6a4ff, 0xd9a5a2ff, 0xd8a3a0ff, 0xd7a29fff, 0xd7a09dff, 0xd69f9cff, 0xd59d9aff, 0xd59c99ff, 0xd49a97ff, 0xd49896ff, 0xd39794ff, 0xd29593ff, 0xd29491ff, 0xd19290ff, 0xd1918eff, 0xd08f8dff, 0xcf8e8bff, 0xcf8c8aff, 0xce8b88ff, 0xcd8987ff, 0xcd8885ff, 0xcc8784ff, 0xcc8582ff, 0xcb8481ff, 0xca827fff, 0xca817eff, 0xc97f7dff, 0xc87e7bff, 0xc87c7aff, 0xc77b78ff, 0xc77977ff, 0xc67875ff, 0xc57674ff, 0xc57572ff, 0xc47371ff, 0xc3726fff, 0xc3706eff, 0xc26f6dff, 0xc16d6bff, 0xc16c6aff, 0xc06a68ff, 0xc06967ff, 0xbf6765ff, 0xbe6664ff, 0xbe6463ff, 0xbd6361ff, 0xbc6160ff, 0xbc605eff, 0xbb5e5dff, 0xba5d5cff, 0xb95b5aff, 0xb95a59ff, 0xb85857ff, 0xb75756ff, 0xb75555ff, 0xb65453ff, 0xb55252ff, 0xb55151ff, 0xb44f4fff, 0xb34d4eff, 0xb24c4cff, 0xb24a4bff, 0xb1494aff, 0xb04748ff, 0xaf4647ff, 0xaf4446ff, 0xae4244ff, 0xad4143ff, 0xac3f42ff, 0xac3e40ff, 0xab3c3fff, 0xaa3a3eff, 0xa9393cff, 0xa9373bff);
preset!(icefire; 0xbde7dbff, 0xbae5daff, 0xb7e3d9ff, 0xb4e1d9ff, 0xb2dfd8ff, 0xafddd7ff, 0xacdbd7ff, 0xa9d9d6ff, 0xa7d7d5ff, 0xa4d5d5ff, 0xa1d3d4ff, 0x9ed1d3ff, 0x9bcfd3ff, 0x98cdd2ff, 0x95cbd2ff, 0x93cad1ff, 0x90c8d1ff, 0x8dc6d0ff, 0x8ac4d0ff, 0x87c2cfff, 0x84c1cfff, 0x81bfcfff, 0x7ebdceff, 0x7bbbceff, 0x78b9ceff, 0x75b8ceff, 0x72b6ceff, 0x6eb4cdff, 0x6bb2cdff, 0x68b0cdff, 0x65afcdff, 0x63adcdff, 0x60abcdff, 0x5da9cdff, 0x5aa7cdff, 0x58a5cdff, 0x55a3cdff, 0x53a2cdff, 0x50a0cdff, 0x4e9ecdff, 0x4c9ccdff, 0x499aceff, 0x4798ceff, 0x4596ceff, 0x4394ceff, 0x4192ceff, 0x3f90ceff, 0x3e8ecfff, 0x3c8ccfff, 0x3a89cfff, 0x3987cfff, 0x3885d0ff, 0x3783d0ff, 0x3781d0ff, 0x377fd0ff, 0x377cd0ff, 0x377ad0ff, 0x3878cfff, 0x3975cfff, 0x3a73ceff, 0x3b71cdff, 0x3d6eccff, 0x3e6ccbff, 0x3f69c9ff, 0x4167c7ff, 0x4265c5ff, 0x4363c3ff, 0x4560c1ff, 0x465ebeff, 0x475cbcff, 0x475ab9ff, 0x4858b6ff, 0x4956b3ff, 0x4954b0ff, 0x4952adff, 0x4a50a9ff, 0x4a4fa5ff, 0x494da1ff, 0x494c9eff, 0x494a9aff, 0x484996ff, 0x474792ff, 0x47468eff, 0x46458aff, 0x454386ff, 0x444282ff, 0x43417fff, 0x42407bff, 0x413e77ff, 0x3f3d74ff, 0x3e3c70ff, 0x3d3b6dff, 0x3c3a69ff, 0x3b3866ff, 0x393763ff, 0x38365fff, 0x37355cff, 0x363459ff, 0x343356ff, 0x333153ff, 0x323050ff, 0x312f4dff, 0x302e4aff, 0x2e2d48ff, 0x2d2c45ff, 0x2c2b42ff, 0x2b2a40ff, 0x2a293dff, 0x29283bff, 0x282739ff, 0x272636ff, 0x262534ff, 0x252532ff, 0x242430ff, 0x24232eff, 0x23222dff, 0x22222bff, 0x222129ff, 0x212028ff, 0x212026ff, 0x202025ff, 0x201f24ff, 0x1f1f23ff, 0x1f1f21ff, 0x1f1e21ff, 0x1f1e20ff, 0x1f1e1fff, 0x1f1e1eff, 0x1f1e1eff, 0x201e1eff, 0x211e1eff, 0x221e1eff, 0x231e1eff, 0x251e1fff, 0x261e1fff, 0x271e1fff, 0x291e20ff, 0x2a1e20ff, 0x2c1e21ff, 0x2d1f21ff, 0x2f1f22ff, 0x311f23ff, 0x332023ff, 0x352024ff, 0x372025ff, 0x392126ff, 0x3b2127ff, 0x3d2228ff, 0x3f2228ff, 0x412329ff, 0x43232aff, 0x46242bff, 0x48242cff, 0x4a252eff, 0x4d252fff, 0x4f2630ff, 0x522731ff, 0x542732ff, 0x572833ff, 0x5a2834ff, 0x5c2935ff, 0x5f2936ff, 0x622937ff, 0x642a38ff, 0x672a39ff, 0x6a2b3aff, 0x6d2b3bff, 0x702b3cff, 0x722c3dff, 0x752c3eff, 0x782c3fff, 0x7b2d40ff, 0x7e2d40ff, 0x812d41ff, 0x842d42ff, 0x872d42ff, 0x8a2e43ff, 0x8d2e43ff, 0x902e44ff, 0x932e44ff, 0x962e44ff, 0x992e44ff, 0x9c2f45ff, 0x9f2f44ff, 0xa22f44ff, 0xa52f44ff, 0xa83044ff, 0xab3043ff, 0xae3143ff, 0xb13242ff, 0xb33341ff, 0xb63441ff, 0xb93540ff, 0xbb363fff, 0xbe373eff, 0xc0393dff, 0xc33a3cff, 0xc53c3cff, 0xc73d3bff, 0xc93f3aff, 0xcc4139ff, 0xce4338ff, 0xd04537ff, 0xd24737ff, 0xd34936ff, 0xd54b35ff, 0xd74e35ff, 0xd95034ff, 0xda5334ff, 0xdc5534ff, 0xde5733ff, 0xdf5a33ff, 0xe15c33ff, 0xe25f33ff, 0xe36233ff, 0xe56433ff, 0xe66734ff, 0xe76a34ff, 0xe86d35ff, 0xe96f36ff, 0xea7238ff, 0xeb753aff, 0xec783bff, 0xed7b3eff, 0xed7e40ff, 0xee8142ff, 0xef8445ff, 0xef8748ff, 0xf0894bff, 0xf18c4eff, 0xf18f51ff, 0xf29255ff, 0xf29558ff, 0xf3985bff, 0xf39a5fff, 0xf49d63ff, 0xf5a066ff, 0xf5a36aff, 0xf6a56dff, 0xf6a871ff, 0xf7ab75ff, 0xf7ae79ff, 0xf8b07cff, 0xf8b380ff, 0xf9b684ff, 0xfab887ff, 0xfabb8bff, 0xfbbe8fff, 0xfbc192ff, 0xfcc396ff, 0xfcc69aff, 0xfdc99eff, 0xfdcca1ff, 0xfecea5ff, 0xfed1a9ff, 0xffd4acff);
preset!(flare; 0xedb081ff, 0xedaf80ff, 0xedae7fff, 0xedad7fff, 0xedac7eff, 0xedab7eff, 0xecaa7dff, 0xeca97cff, 0xeca87cff, 0xeca77bff, 0xeca67bff, 0xeca57aff, 0xeca479ff, 0xeca379ff, 0xeca278ff, 0xeca178ff, 0xeca077ff, 0xec9f76ff, 0xeb9e76ff, 0xeb9d75ff, 0xeb9c75ff, 0xeb9b74ff, 0xeb9a73ff, 0xeb9973ff, 0xeb9972ff, 0xeb9872ff, 0xeb9771ff, 0xea9671ff, 0xea9570ff, 0xea946fff, 0xea936fff, 0xea926eff, 0xea916eff, 0xea906dff, 0xea8f6cff, 0xea8e6cff, 0xe98d6bff, 0xe98c6bff, 0xe98b6aff, 0xe98a6aff, 0xe98969ff, 0xe98868ff, 0xe98768ff, 0xe98667ff, 0xe88567ff, 0xe88466ff, 0xe88366ff, 0xe88265ff, 0xe88165ff, 0xe88064ff, 0xe87f64ff, 0xe77e63ff, 0xe77d63ff, 0xe77c63ff, 0xe77b62ff, 0xe77a62ff, 0xe67961ff, 0xe67861ff, 0xe67760ff, 0xe67660ff, 0xe67560ff, 0xe5745fff, 0xe5735fff, 0xe5725fff, 0xe5715eff, 0xe5705eff, 0xe46f5eff, 0xe46e5eff, 0xe46d5dff, 0xe46c5dff, 0xe36b5dff, 0xe36a5dff, 0xe3695dff, 0xe3685cff, 0xe2675cff, 0xe2665cff, 0xe2655cff, 0xe1645cff, 0xe1635cff, 0xe1625cff, 0xe0615cff, 0xe0605cff, 0xe05f5cff, 0xdf5f5cff, 0xdf5e5cff, 0xde5d5cff, 0xde5c5cff, 0xde5b5cff, 0xdd5a5cff, 0xdd595cff, 0xdc585cff, 0xdc575cff, 0xdb565dff, 0xdb565dff, 0xda555dff, 0xda545dff, 0xd9535dff, 0xd9525eff, 0xd8525eff, 0xd7515eff, 0xd7505eff, 0xd64f5fff, 0xd64f5fff, 0xd54e5fff, 0xd44d60ff, 0xd44c60ff, 0xd34c60ff, 0xd24b60ff, 0xd24a61ff, 0xd14a61ff, 0xd04962ff, 0xd04962ff, 0xcf4862ff, 0xce4763ff, 0xcd4763ff, 0xcc4663ff, 0xcc4664ff, 0xcb4564ff, 0xca4564ff, 0xc94465ff, 0xc84465ff, 0xc84365ff, 0xc74366ff, 0xc64366ff, 0xc54266ff, 0xc44267ff, 0xc34167ff, 0xc24167ff, 0xc14168ff, 0xc14068ff, 0xc04068ff, 0xbf4069ff, 0xbe3f69ff, 0xbd3f69ff, 0xbc3f69ff, 0xbb3f6aff, 0xba3e6aff, 0xb93e6aff, 0xb83e6bff, 0xb73d6bff, 0xb63d6bff, 0xb53d6bff, 0xb43d6bff, 0xb33c6cff, 0xb23c6cff, 0xb13c6cff, 0xb13c6cff, 0xb03b6dff, 0xaf3b6dff, 0xae3b6dff, 0xad3b6dff, 0xac3a6dff, 0xab3a6dff, 0xaa3a6eff, 0xa93a6eff, 0xa8396eff, 0xa7396eff, 0xa6396eff, 0xa5396eff, 0xa4386fff, 0xa3386fff, 0xa2386fff, 0xa1386fff, 0xa1376fff, 0xa0376fff, 0x9f376fff, 0x9e3770ff, 0x9d3670ff, 0x9c3670ff, 0x9b3670ff, 0x9a3670ff, 0x993570ff, 0x983570ff, 0x973570ff, 0x963570ff, 0x953470ff, 0x943470ff, 0x943471ff, 0x933471ff, 0x923371ff, 0x913371ff, 0x903371ff, 0x8f3371ff, 0x8e3271ff, 0x8d3271ff, 0x8c3271ff, 0x8b3271ff, 0x8a3171ff, 0x893171ff, 0x883171ff, 0x873171ff, 0x873171ff, 0x863071ff, 0x853071ff, 0x843071ff, 0x833070ff, 0x822f70ff, 0x812f70ff, 0x802f70ff, 0x7f2f70ff, 0x7e2f70ff, 0x7d2e70ff, 0x7c2e70ff, 0x7b2e70ff, 0x7a2e70ff, 0x792e6fff, 0x782e6fff, 0x772d6fff, 0x762d6fff, 0x752d6fff, 0x752d6fff, 0x742d6eff, 0x732c6eff, 0x722c6eff, 0x712c6eff, 0x702c6eff, 0x6f2c6dff, 0x6e2c6dff, 0x6d2b6dff, 0x6c2b6dff, 0x6b2b6cff, 0x6a2b6cff, 0x692b6cff, 0x682a6cff, 0x672a6bff, 0x662a6bff, 0x652a6bff, 0x642a6aff, 0x642a6aff, 0x63296aff, 0x62296aff, 0x612969ff, 0x602969ff, 0x5f2969ff, 0x5e2868ff, 0x5d2868ff, 0x5c2868ff, 0x5b2867ff, 0x5a2767ff, 0x592767ff, 0x582766ff, 0x582766ff, 0x572766ff, 0x562666ff, 0x552665ff, 0x542665ff, 0x532665ff, 0x522564ff, 0x512564ff, 0x502564ff, 0x4f2463ff, 0x4f2463ff, 0x4e2463ff, 0x4d2463ff, 0x4c2362ff, 0x4b2362ff);
preset!(crest; 0xa5cd90ff, 0xa4cc90ff, 0xa3cc91ff, 0xa2cb91ff, 0xa0cb91ff, 0x9fca91ff, 0x9eca91ff, 0x9dc991ff, 0x9cc891ff, 0x9bc891ff, 0x9ac791ff, 0x99c791ff, 0x98c691ff, 0x96c691ff, 0x95c591ff, 0x94c591ff, 0x93c491ff, 0x92c491ff, 0x91c391ff, 0x90c391ff, 0x8fc291ff, 0x8ec291ff, 0x8dc191ff, 0x8bc191ff, 0x8ac091ff, 0x89bf91ff, 0x88bf91ff, 0x87be91ff, 0x86be91ff, 0x85bd91ff, 0x84bd91ff, 0x82bc91ff, 0x81bc91ff, 0x80bb91ff, 0x7fbb91ff, 0x7eba91ff, 0x7dba91ff, 0x7cb991ff, 0x7bb991ff, 0x79b891ff, 0x78b891ff, 0x77b791ff, 0x76b791ff, 0x75b690ff, 0x74b690ff, 0x73b590ff, 0x72b490ff, 0x71b490ff, 0x70b390ff, 0x6fb390ff, 0x6eb290ff, 0x6db290ff, 0x6cb190ff, 0x6bb190ff, 0x6ab090ff, 0x69b090ff, 0x68af90ff, 0x67ae90ff, 0x66ae90ff, 0x65ad90ff, 0x64ad90ff, 0x63ac90ff, 0x62ac90ff, 0x62ab90ff, 0x61aa90ff, 0x60aa90ff, 0x5fa990ff, 0x5ea990ff, 0x5da890ff, 0x5ca890ff, 0x5ba790ff, 0x5ba690ff, 0x5aa690ff, 0x59a590ff, 0x58a590ff, 0x57a490ff, 0x57a490ff, 0x56a390ff, 0x55a290ff, 0x54a290ff, 0x53a190ff, 0x53a190ff, 0x52a090ff, 0x519f90ff, 0x509f90ff, 0x509e90ff, 0x4f9e90ff, 0x4e9d90ff, 0x4e9d90ff, 0x4d9c90ff, 0x4c9b90ff, 0x4b9b90ff, 0x4b9a8fff, 0x4a9a8fff, 0x49998fff, 0x49988fff, 0x48988fff, 0x47978fff, 0x47978fff, 0x46968fff, 0x45958fff, 0x45958fff, 0x44948fff, 0x43948fff, 0x43938fff, 0x42928fff, 0x41928fff, 0x41918fff, 0x40918fff, 0x40908eff, 0x3f8f8eff, 0x3e8f8eff, 0x3e8e8eff, 0x3d8e8eff, 0x3c8d8eff, 0x3c8c8eff, 0x3b8c8eff, 0x3a8b8eff, 0x3a8b8eff, 0x398a8eff, 0x388a8eff, 0x38898eff, 0x37888eff, 0x37888dff, 0x36878dff, 0x35878dff, 0x35868dff, 0x34858dff, 0x33858dff, 0x33848dff, 0x32848dff, 0x31838dff, 0x31828dff, 0x30828dff, 0x2f818dff, 0x2f818dff, 0x2e808dff, 0x2d808cff, 0x2d7f8cff, 0x2c7e8cff, 0x2c7e8cff, 0x2b7d8cff, 0x2a7d8cff, 0x2a7c8cff, 0x297b8cff, 0x287b8cff, 0x287a8cff, 0x277a8cff, 0x27798cff, 0x26788cff, 0x25788cff, 0x25778cff, 0x24778bff, 0x24768bff, 0x23758bff, 0x23758bff, 0x22748bff, 0x22748bff, 0x21738bff, 0x21728bff, 0x20728bff, 0x20718bff, 0x20718bff, 0x1f708bff, 0x1f6f8aff, 0x1e6f8aff, 0x1e6e8aff, 0x1e6d8aff, 0x1e6d8aff, 0x1d6c8aff, 0x1d6c8aff, 0x1d6b8aff, 0x1d6a8aff, 0x1d6a8aff, 0x1c6989ff, 0x1c6889ff, 0x1c6889ff, 0x1c6789ff, 0x1c6689ff, 0x1c6689ff, 0x1c6589ff, 0x1c6488ff, 0x1c6488ff, 0x1c6388ff, 0x1d6388ff, 0x1d6288ff, 0x1d6188ff, 0x1d6187ff, 0x1d6087ff, 0x1d5f87ff, 0x1d5f87ff, 0x1e5e87ff, 0x1e5d86ff, 0x1e5d86ff, 0x1e5c86ff, 0x1e5b86ff, 0x1f5b86ff, 0x1f5a85ff, 0x1f5985ff, 0x1f5985ff, 0x205885ff, 0x205784ff, 0x205784ff, 0x205684ff, 0x215584ff, 0x215583ff, 0x215483ff, 0x225383ff, 0x225283ff, 0x225282ff, 0x225182ff, 0x235082ff, 0x235081ff, 0x234f81ff, 0x244e81ff, 0x244e80ff, 0x244d80ff, 0x254c80ff, 0x254c7fff, 0x254b7fff, 0x254a7fff, 0x26497eff, 0x26497eff, 0x26487eff, 0x27477dff, 0x27477dff, 0x27467cff, 0x27457cff, 0x28457cff, 0x28447bff, 0x28437bff, 0x28427aff, 0x29427aff, 0x29417aff, 0x294079ff, 0x294079ff, 0x2a3f78ff, 0x2a3e78ff, 0x2a3d78ff, 0x2a3d77ff, 0x2a3c77ff, 0x2a3b76ff, 0x2b3b76ff, 0x2b3a76ff, 0x2b3975ff, 0x2b3875ff, 0x2b3875ff, 0x2b3774ff, 0x2b3674ff, 0x2c3574ff, 0x2c3573ff, 0x2c3473ff, 0x2c3373ff, 0x2c3272ff, 0x2c3172ff, 0x2c3172ff);
