use std::f64::consts::{FRAC_PI_2, PI, TAU};
use std::fmt::{self, Debug, Formatter};
use std::hash::Hash;
use std::sync::Arc;

use ecow::EcoString;
use kurbo::Vec2;
use typst_syntax::{Span, Spanned};

use crate::diag::{SourceResult, bail};
use crate::foundations::{
    Args, Array, Cast, Func, IntoValue, Repr, Smart, array, cast, func, scope, ty,
};
use crate::layout::{Angle, Axes, Dir, Quadrant, Ratio};
use crate::visualize::{Color, ColorSpace, WeightedColor};

/// A color gradient.
///
/// Typst supports linear gradients through the
/// [`gradient.linear` function]($gradient.linear), radial gradients through
/// the [`gradient.radial` function]($gradient.radial), and conic gradients
/// through the [`gradient.conic` function]($gradient.conic).
///
/// A gradient can be used for the following purposes:
/// - As a fill to paint the interior of a shape:
///   `{rect(fill: gradient.linear(..))}`
/// - As a stroke to paint the outline of a shape:
///   `{rect(stroke: 1pt + gradient.linear(..))}`
/// - As the fill of text:
///   `{set text(fill: gradient.linear(..))}`
/// - As a color map you can [sample]($gradient.sample) from:
///   `{gradient.linear(..).sample(50%)}`
///
/// # Examples
/// ```example
/// >>> #set square(size: 50pt)
/// #stack(
///   dir: ltr,
///   spacing: 1fr,
///   square(fill: gradient.linear(..color.map.rainbow)),
///   square(fill: gradient.radial(..color.map.rainbow)),
///   square(fill: gradient.conic(..color.map.rainbow)),
/// )
/// ```
///
/// Gradients are also supported on text, but only when setting the
/// [relativeness]($gradient.relative) to either `{auto}` (the default value) or
/// `{"parent"}`. To create word-by-word or glyph-by-glyph gradients, you can
/// wrap the words or characters of your text in [boxes]($box) manually or
/// through a [show rule]($styling/#show-rules).
///
/// ```example
/// >>> #set page(width: auto, height: auto, margin: 12pt)
/// >>> #set text(size: 12pt)
/// #set text(fill: gradient.linear(red, blue))
/// #let rainbow(content) = {
///   set text(fill: gradient.linear(..color.map.rainbow))
///   box(content)
/// }
///
/// This is a gradient on text, but with a #rainbow[twist]!
/// ```
///
/// # Stops
/// A gradient is composed of a series of stops. Each of these stops has a color
/// and an offset. The offset is a [ratio]($ratio) between `{0%}` and `{100%}` or
/// an angle between `{0deg}` and `{360deg}`. The offset is a relative position
/// that determines how far along the gradient the stop is located. The stop's
/// color is the color of the gradient at that position. You can choose to omit
/// the offsets when defining a gradient. In this case, Typst will space all
/// stops evenly.
///
/// Typst predefines color maps that you can use as stops. See the
/// [`color`]($color/#predefined-color-maps) documentation for more details.
///
/// # Relativeness
/// The location of the `{0%}` and `{100%}` stops depends on the dimensions
/// of a container. This container can either be the shape that it is being
/// painted on, or the closest surrounding container. This is controlled by the
/// `relative` argument of a gradient constructor. By default, gradients are
/// relative to the shape they are being painted on, unless the gradient is
/// applied on text, in which case they are relative to the closest ancestor
/// container.
///
/// Typst determines the ancestor container as follows:
/// - For shapes that are placed at the root/top level of the document, the
///   closest ancestor is the page itself.
/// - For other shapes, the ancestor is the innermost [`block`] or [`box`] that
///   contains the shape. This includes the boxes and blocks that are implicitly
///   created by show rules and elements. For example, a [`rotate`] will not
///   affect the parent of a gradient, but a [`grid`] will.
///
/// # Color spaces and interpolation
/// Gradients can be interpolated in any color space. By default, gradients are
/// interpolated in the [Oklab]($color.oklab) color space, which is a
/// [perceptually uniform](https://programmingdesignsystems.com/color/perceptually-uniform-color-spaces/index.html)
/// color space. This means that the gradient will be perceived as having a
/// smooth progression of colors. This is particularly useful for data
/// visualization.
///
/// However, you can choose to interpolate the gradient in any supported color
/// space you want, but beware that some color spaces are not suitable for
/// perceptually interpolating between colors. Consult the table below when
/// choosing an interpolation space.
///
/// |           Color space           | Perceptually uniform? |
/// | ------------------------------- |-----------------------|
/// | [Oklab]($color.oklab)           | *Yes*                 |
/// | [Oklch]($color.oklch)           | *Yes*                 |
/// | [sRGB]($color.rgb)              | *No*                  |
/// | [linear-RGB]($color.linear-rgb) | *Yes*                 |
/// | [CMYK]($color.cmyk)             | *No*                  |
/// | [Grayscale]($color.luma)        | *Yes*                 |
/// | [HSL]($color.hsl)               | *No*                  |
/// | [HSV]($color.hsv)               | *No*                  |
///
/// ```preview
/// >>> #set text(fill: white, font: "IBM Plex Sans", 8pt)
/// >>> #set block(spacing: 0pt)
/// #let spaces = (
///   ("Oklab", color.oklab),
///   ("Oklch", color.oklch),
///   ("sRGB", color.rgb),
///   ("linear-RGB", color.linear-rgb),
///   ("CMYK", color.cmyk),
///   ("Grayscale", color.luma),
///   ("HSL", color.hsl),
///   ("HSV", color.hsv),
/// )
///
/// #for (name, space) in spaces {
///   block(
///     width: 100%,
///     inset: 4pt,
///     fill: gradient.linear(
///       red,
///       blue,
///       space: space,
///     ),
///     strong(upper(name)),
///   )
/// }
/// ```
///
/// # Direction
/// Some gradients are sensitive to direction. For example, a linear gradient
/// has an angle that determines its direction. Typst uses a clockwise angle,
/// with 0° being from left to right, 90° from top to bottom, 180° from right to
/// left, and 270° from bottom to top.
///
/// ```example
/// >>> #set square(size: 50pt)
/// #stack(
///   dir: ltr,
///   spacing: 1fr,
///   square(fill: gradient.linear(red, blue, angle: 0deg)),
///   square(fill: gradient.linear(red, blue, angle: 90deg)),
///   square(fill: gradient.linear(red, blue, angle: 180deg)),
///   square(fill: gradient.linear(red, blue, angle: 270deg)),
/// )
/// ```
///
/// # Note on file sizes
///
/// Gradients can be quite large, especially if they have many stops. This is
/// because gradients are stored as a list of colors and offsets, which can
/// take up a lot of space. If you are concerned about file sizes, you should
/// consider the following:
/// - SVG gradients are currently inefficiently encoded. This will be improved
///   in the future.
/// - PDF gradients in the [`color.oklab`]($color.oklab), [`color.hsv`]($color.hsv),
///   [`color.hsl`]($color.hsl), and [`color.oklch`]($color.oklch) color spaces
///   are stored as a list of [`color.rgb`]($color.rgb) colors with extra stops
///   in between. This avoids needing to encode these color spaces in your PDF
///   file, but it does add extra stops to your gradient, which can increase
///   the file size.
#[ty(scope, cast)]
#[derive(Clone, PartialEq, Eq, Hash)]
pub enum Gradient {
    Linear(Arc<LinearGradient>),
    Radial(Arc<RadialGradient>),
    Conic(Arc<ConicGradient>),
}

#[scope]
#[allow(clippy::too_many_arguments)]
impl Gradient {
    /// Creates a new linear gradient, in which colors transition along a
    /// straight line.
    ///
    /// ```example
    /// #rect(
    ///   width: 100%,
    ///   height: 20pt,
    ///   fill: gradient.linear(
    ///     ..color.map.viridis,
    ///   ),
    /// )
    /// ```
    #[func(title = "Linear Gradient")]
    pub fn linear(
        args: &mut Args,
        span: Span,
        /// The color [stops](#stops) of the gradient.
        #[variadic]
        stops: Vec<Spanned<GradientStop>>,
        /// The color space in which to interpolate the gradient.
        ///
        /// Defaults to a perceptually uniform color space called
        /// [Oklab]($color.oklab).
        #[named]
        #[default(ColorSpace::Oklab)]
        space: ColorSpace,
        /// The [relative placement](#relativeness) of the gradient.
        ///
        /// For an element placed at the root/top level of the document, the
        /// parent is the page itself. For other elements, the parent is the
        /// innermost block, box, column, grid, or stack that contains the
        /// element.
        #[named]
        #[default(Smart::Auto)]
        relative: Smart<RelativeTo>,
        /// The direction of the gradient.
        #[external]
        #[default(Dir::LTR)]
        dir: Dir,
        /// The angle of the gradient.
        #[external]
        angle: Angle,
    ) -> SourceResult<Gradient> {
        let angle = if let Some(angle) = args.named::<Angle>("angle")? {
            angle
        } else if let Some(dir) = args.named::<Dir>("dir")? {
            match dir {
                Dir::LTR => Angle::rad(0.0),
                Dir::RTL => Angle::rad(PI),
                Dir::TTB => Angle::rad(FRAC_PI_2),
                Dir::BTT => Angle::rad(3.0 * FRAC_PI_2),
            }
        } else {
            Angle::rad(0.0)
        };

        if stops.len() < 2 {
            bail!(
                span, "a gradient must have at least two stops";
                hint: "try filling the shape with a single color instead"
            );
        }

        Ok(Self::Linear(Arc::new(LinearGradient {
            stops: process_stops(&stops)?,
            angle,
            space,
            relative,
            anti_alias: true,
        })))
    }

    /// Creates a new radial gradient, in which colors radiate away from an
    /// origin.
    ///
    /// The gradient is defined by two circles: the focal circle and the end
    /// circle. The focal circle is a circle with center `focal-center` and
    /// radius `focal-radius`, that defines the points at which the gradient
    /// starts and has the color of the first stop. The end circle is a circle
    /// with center `center` and radius `radius`, that defines the points at
    /// which the gradient ends and has the color of the last stop. The gradient
    /// is then interpolated between these two circles.
    ///
    /// Using these four values, also called the focal point for the starting
    /// circle and the center and radius for the end circle, we can define a
    /// gradient with more interesting properties than a basic radial gradient.
    ///
    /// ```example
    /// >>> #set circle(radius: 30pt)
    /// #stack(
    ///   dir: ltr,
    ///   spacing: 1fr,
    ///   circle(fill: gradient.radial(
    ///     ..color.map.viridis,
    ///   )),
    ///   circle(fill: gradient.radial(
    ///     ..color.map.viridis,
    ///     focal-center: (10%, 40%),
    ///     focal-radius: 5%,
    ///   )),
    /// )
    /// ```
    #[func(title = "Radial Gradient")]
    fn radial(
        span: Span,
        /// The color [stops](#stops) of the gradient.
        #[variadic]
        stops: Vec<Spanned<GradientStop>>,
        /// The color space in which to interpolate the gradient.
        ///
        /// Defaults to a perceptually uniform color space called
        /// [Oklab]($color.oklab).
        #[named]
        #[default(ColorSpace::Oklab)]
        space: ColorSpace,
        /// The [relative placement](#relativeness) of the gradient.
        ///
        /// For an element placed at the root/top level of the document, the parent
        /// is the page itself. For other elements, the parent is the innermost block,
        /// box, column, grid, or stack that contains the element.
        #[named]
        #[default(Smart::Auto)]
        relative: Smart<RelativeTo>,
        /// The center of the end circle of the gradient.
        ///
        /// A value of `{(50%, 50%)}` means that the end circle is
        /// centered inside of its container.
        #[named]
        #[default(Axes::splat(Ratio::new(0.5)))]
        center: Axes<Ratio>,
        /// The radius of the end circle of the gradient.
        ///
        /// By default, it is set to `{50%}`. The ending radius must be bigger
        /// than the focal radius.
        #[named]
        #[default(Spanned::new(Ratio::new(0.5), Span::detached()))]
        radius: Spanned<Ratio>,
        /// The center of the focal circle of the gradient.
        ///
        /// The focal center must be inside of the end circle.
        ///
        /// A value of `{(50%, 50%)}` means that the focal circle is
        /// centered inside of its container.
        ///
        /// By default it is set to the same as the center of the last circle.
        #[named]
        #[default(Smart::Auto)]
        focal_center: Smart<Axes<Ratio>>,
        /// The radius of the focal circle of the gradient.
        ///
        /// The focal center must be inside of the end circle.
        ///
        /// By default, it is set to `{0%}`. The focal radius must be smaller
        /// than the ending radius`.
        #[named]
        #[default(Spanned::new(Ratio::new(0.0), Span::detached()))]
        focal_radius: Spanned<Ratio>,
    ) -> SourceResult<Gradient> {
        if stops.len() < 2 {
            bail!(
                span, "a gradient must have at least two stops";
                hint: "try filling the shape with a single color instead"
            );
        }

        if focal_radius.v > radius.v {
            bail!(
                focal_radius.span,
                "the focal radius must be smaller than the end radius";
                hint: "try using a focal radius of `0%` instead"
            );
        }

        let focal_center = focal_center.unwrap_or(center);
        let d_center_sqr = (focal_center.x - center.x).get().powi(2)
            + (focal_center.y - center.y).get().powi(2);
        if d_center_sqr.sqrt() >= (radius.v - focal_radius.v).get() {
            bail!(
                span,
                "the focal circle must be inside of the end circle";
                hint: "try using a focal center of `auto` instead"
            );
        }

        Ok(Gradient::Radial(Arc::new(RadialGradient {
            stops: process_stops(&stops)?,
            center: center.map(From::from),
            radius: radius.v,
            focal_center,
            focal_radius: focal_radius.v,
            space,
            relative,
            anti_alias: true,
        })))
    }

    /// Creates a new conic gradient, in which colors change radially around a
    /// center point.
    ///
    /// You can control the center point of the gradient by using the `center`
    /// argument. By default, the center point is the center of the shape.
    ///
    /// ```example
    /// >>> #set circle(radius: 30pt)
    /// #stack(
    ///   dir: ltr,
    ///   spacing: 1fr,
    ///   circle(fill: gradient.conic(
    ///     ..color.map.viridis,
    ///   )),
    ///   circle(fill: gradient.conic(
    ///     ..color.map.viridis,
    ///     center: (20%, 30%),
    ///   )),
    /// )
    /// ```
    #[func(title = "Conic Gradient")]
    pub fn conic(
        span: Span,
        /// The color [stops](#stops) of the gradient.
        #[variadic]
        stops: Vec<Spanned<GradientStop>>,
        /// The angle of the gradient.
        #[named]
        #[default(Angle::zero())]
        angle: Angle,
        /// The color space in which to interpolate the gradient.
        ///
        /// Defaults to a perceptually uniform color space called
        /// [Oklab]($color.oklab).
        #[named]
        #[default(ColorSpace::Oklab)]
        space: ColorSpace,
        /// The [relative placement](#relativeness) of the gradient.
        ///
        /// For an element placed at the root/top level of the document, the parent
        /// is the page itself. For other elements, the parent is the innermost block,
        /// box, column, grid, or stack that contains the element.
        #[named]
        #[default(Smart::Auto)]
        relative: Smart<RelativeTo>,
        /// The center of the circle of the gradient.
        ///
        /// A value of `{(50%, 50%)}` means that the circle is centered inside
        /// of its container.
        #[named]
        #[default(Axes::splat(Ratio::new(0.5)))]
        center: Axes<Ratio>,
    ) -> SourceResult<Gradient> {
        if stops.len() < 2 {
            bail!(
                span, "a gradient must have at least two stops";
                hint: "try filling the shape with a single color instead"
            );
        }

        Ok(Gradient::Conic(Arc::new(ConicGradient {
            stops: process_stops(&stops)?,
            angle,
            center: center.map(From::from),
            space,
            relative,
            anti_alias: true,
        })))
    }

    /// Creates a sharp version of this gradient.
    ///
    /// Sharp gradients have discrete jumps between colors, instead of a
    /// smooth transition. They are particularly useful for creating color
    /// lists for a preset gradient.
    ///
    /// ```example
    /// #set rect(width: 100%, height: 20pt)
    /// #let grad = gradient.linear(..color.map.rainbow)
    /// #rect(fill: grad)
    /// #rect(fill: grad.sharp(5))
    /// #rect(fill: grad.sharp(5, smoothness: 20%))
    /// ```
    #[func]
    pub fn sharp(
        &self,
        /// The number of stops in the gradient.
        steps: Spanned<usize>,
        /// How much to smooth the gradient.
        #[named]
        #[default(Spanned::new(Ratio::zero(), Span::detached()))]
        smoothness: Spanned<Ratio>,
    ) -> SourceResult<Gradient> {
        if steps.v < 2 {
            bail!(steps.span, "sharp gradients must have at least two stops");
        }

        if smoothness.v.get() < 0.0 || smoothness.v.get() > 1.0 {
            bail!(smoothness.span, "smoothness must be between 0 and 1");
        }

        let n = steps.v;
        let smoothness = smoothness.v.get();
        let colors = (0..n)
            .flat_map(|i| {
                let c = self
                    .sample(RatioOrAngle::Ratio(Ratio::new(i as f64 / (n - 1) as f64)));

                [c, c]
            })
            .collect::<Vec<_>>();

        let mut positions = Vec::with_capacity(n * 2);
        let index_to_progress = |i| i as f64 * 1.0 / n as f64;

        let progress = smoothness * 1.0 / (4.0 * n as f64);
        for i in 0..n {
            let mut j = 2 * i;
            positions.push(index_to_progress(i));
            if j > 0 {
                positions[j] += progress;
            }

            j += 1;
            positions.push(index_to_progress(i + 1));
            if j < colors.len() - 1 {
                positions[j] -= progress;
            }
        }

        let mut stops = colors
            .into_iter()
            .zip(positions)
            .map(|(c, p)| (c, Ratio::new(p)))
            .collect::<Vec<_>>();

        stops.dedup();

        Ok(match self {
            Self::Linear(linear) => Self::Linear(Arc::new(LinearGradient {
                stops,
                angle: linear.angle,
                space: linear.space,
                relative: linear.relative,
                anti_alias: false,
            })),
            Self::Radial(radial) => Self::Radial(Arc::new(RadialGradient {
                stops,
                center: radial.center,
                radius: radial.radius,
                focal_center: radial.focal_center,
                focal_radius: radial.focal_radius,
                space: radial.space,
                relative: radial.relative,
                anti_alias: false,
            })),
            Self::Conic(conic) => Self::Conic(Arc::new(ConicGradient {
                stops,
                angle: conic.angle,
                center: conic.center,
                space: conic.space,
                relative: conic.relative,
                anti_alias: false,
            })),
        })
    }

    /// Repeats this gradient a given number of times, optionally mirroring it
    /// at every second repetition.
    ///
    /// ```example
    /// #circle(
    ///   radius: 40pt,
    ///   fill: gradient
    ///     .radial(aqua, white)
    ///     .repeat(4),
    /// )
    /// ```
    #[func]
    pub fn repeat(
        &self,
        /// The number of times to repeat the gradient.
        repetitions: Spanned<usize>,
        /// Whether to mirror the gradient at every second repetition, i.e.,
        /// the first instance (and all odd ones) stays unchanged.
        ///
        /// ```example
        /// #circle(
        ///   radius: 40pt,
        ///   fill: gradient
        ///     .conic(green, black)
        ///     .repeat(2, mirror: true)
        /// )
        /// ```
        #[named]
        #[default(false)]
        mirror: bool,
    ) -> SourceResult<Gradient> {
        if repetitions.v == 0 {
            bail!(repetitions.span, "must repeat at least once");
        }

        let n = repetitions.v;
        let mut stops = std::iter::repeat_n(self.stops_ref(), n)
            .enumerate()
            .flat_map(|(i, stops)| {
                let mut stops = stops
                    .iter()
                    .map(move |&(color, offset)| {
                        let r = offset.get();
                        if i % 2 == 1 && mirror {
                            (color, Ratio::new((i as f64 + 1.0 - r) / n as f64))
                        } else {
                            (color, Ratio::new((i as f64 + r) / n as f64))
                        }
                    })
                    .collect::<Vec<_>>();

                if i % 2 == 1 && mirror {
                    stops.reverse();
                }

                stops
            })
            .collect::<Vec<_>>();

        stops.dedup();

        Ok(match self {
            Self::Linear(linear) => Self::Linear(Arc::new(LinearGradient {
                stops,
                angle: linear.angle,
                space: linear.space,
                relative: linear.relative,
                anti_alias: linear.anti_alias,
            })),
            Self::Radial(radial) => Self::Radial(Arc::new(RadialGradient {
                stops,
                center: radial.center,
                radius: radial.radius,
                focal_center: radial.focal_center,
                focal_radius: radial.focal_radius,
                space: radial.space,
                relative: radial.relative,
                anti_alias: radial.anti_alias,
            })),
            Self::Conic(conic) => Self::Conic(Arc::new(ConicGradient {
                stops,
                angle: conic.angle,
                center: conic.center,
                space: conic.space,
                relative: conic.relative,
                anti_alias: conic.anti_alias,
            })),
        })
    }

    /// Returns the kind of this gradient.
    #[func]
    pub fn kind(&self) -> Func {
        match self {
            Self::Linear(_) => Self::linear_data().into(),
            Self::Radial(_) => Self::radial_data().into(),
            Self::Conic(_) => Self::conic_data().into(),
        }
    }

    /// Returns the stops of this gradient.
    #[func]
    pub fn stops(&self) -> Vec<GradientStop> {
        match self {
            Self::Linear(linear) => linear
                .stops
                .iter()
                .map(|(color, offset)| GradientStop {
                    color: *color,
                    offset: Some(*offset),
                })
                .collect(),
            Self::Radial(radial) => radial
                .stops
                .iter()
                .map(|(color, offset)| GradientStop {
                    color: *color,
                    offset: Some(*offset),
                })
                .collect(),
            Self::Conic(conic) => conic
                .stops
                .iter()
                .map(|(color, offset)| GradientStop {
                    color: *color,
                    offset: Some(*offset),
                })
                .collect(),
        }
    }

    /// Returns the mixing space of this gradient.
    #[func]
    pub fn space(&self) -> ColorSpace {
        match self {
            Self::Linear(linear) => linear.space,
            Self::Radial(radial) => radial.space,
            Self::Conic(conic) => conic.space,
        }
    }

    /// Returns the relative placement of this gradient.
    #[func]
    pub fn relative(&self) -> Smart<RelativeTo> {
        match self {
            Self::Linear(linear) => linear.relative,
            Self::Radial(radial) => radial.relative,
            Self::Conic(conic) => conic.relative,
        }
    }

    /// Returns the angle of this gradient.
    ///
    /// Returns `{none}` if the gradient is neither linear nor conic.
    #[func]
    pub fn angle(&self) -> Option<Angle> {
        match self {
            Self::Linear(linear) => Some(linear.angle),
            Self::Radial(_) => None,
            Self::Conic(conic) => Some(conic.angle),
        }
    }

    /// Returns the center of this gradient.
    ///
    /// Returns `{none}` if the gradient is neither radial nor conic.
    #[func]
    pub fn center(&self) -> Option<Axes<Ratio>> {
        match self {
            Self::Linear(_) => None,
            Self::Radial(radial) => Some(radial.center),
            Self::Conic(conic) => Some(conic.center),
        }
    }

    /// Returns the radius of this gradient.
    ///
    /// Returns `{none}` if the gradient is not radial.
    #[func]
    pub fn radius(&self) -> Option<Ratio> {
        match self {
            Self::Linear(_) => None,
            Self::Radial(radial) => Some(radial.radius),
            Self::Conic(_) => None,
        }
    }

    /// Returns the focal-center of this gradient.
    ///
    /// Returns `{none}` if the gradient is not radial.
    #[func]
    pub fn focal_center(&self) -> Option<Axes<Ratio>> {
        match self {
            Self::Linear(_) => None,
            Self::Radial(radial) => Some(radial.focal_center),
            Self::Conic(_) => None,
        }
    }

    /// Returns the focal-radius of this gradient.
    ///
    /// Returns `{none}` if the gradient is not radial.
    #[func]
    pub fn focal_radius(&self) -> Option<Ratio> {
        match self {
            Self::Linear(_) => None,
            Self::Radial(radial) => Some(radial.focal_radius),
            Self::Conic(_) => None,
        }
    }

    /// Sample the gradient at a given position.
    ///
    /// The position is either a position along the gradient (a [ratio] between
    /// `{0%}` and `{100%}`) or an [angle]. Any value outside of this range will
    /// be clamped.
    #[func]
    pub fn sample(
        &self,
        /// The position at which to sample the gradient.
        t: RatioOrAngle,
    ) -> Color {
        let value: f64 = t.to_ratio().get();

        match self {
            Self::Linear(linear) => sample_stops(&linear.stops, linear.space, value),
            Self::Radial(radial) => sample_stops(&radial.stops, radial.space, value),
            Self::Conic(conic) => sample_stops(&conic.stops, conic.space, value),
        }
    }

    /// Samples the gradient at multiple positions at once and returns the
    /// results as an array.
    #[func]
    pub fn samples(
        &self,
        /// The positions at which to sample the gradient.
        #[variadic]
        ts: Vec<RatioOrAngle>,
    ) -> Array {
        ts.into_iter().map(|t| self.sample(t).into_value()).collect()
    }
}

impl Gradient {
    /// Clones this gradient, but with a different relative placement.
    pub fn with_relative(mut self, relative: RelativeTo) -> Self {
        match &mut self {
            Self::Linear(linear) => {
                Arc::make_mut(linear).relative = Smart::Custom(relative);
            }
            Self::Radial(radial) => {
                Arc::make_mut(radial).relative = Smart::Custom(relative);
            }
            Self::Conic(conic) => {
                Arc::make_mut(conic).relative = Smart::Custom(relative);
            }
        }

        self
    }
    /// Returns a reference to the stops of this gradient.
    pub fn stops_ref(&self) -> &[(Color, Ratio)] {
        match self {
            Gradient::Linear(linear) => &linear.stops,
            Gradient::Radial(radial) => &radial.stops,
            Gradient::Conic(conic) => &conic.stops,
        }
    }

    /// Samples the gradient at a given position, in the given container.
    /// Handles the aspect ratio and angle directly.
    pub fn sample_at(&self, (x, y): (f32, f32), (width, height): (f32, f32)) -> Color {
        // Normalize the coordinates.
        let (mut x, mut y) = (x / width, y / height);
        let t = match self {
            Self::Linear(linear) => {
                // Aspect ratio correction.
                let angle = Gradient::correct_aspect_ratio(
                    linear.angle,
                    Ratio::new((width / height) as f64),
                )
                .to_rad();
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
            Self::Radial(radial) => {
                // Source: @Enivex - https://typst.app/project/pYLeS0QyCCe8mf0pdnwoAI
                let cr = radial.radius.get();
                let fr = radial.focal_radius.get();
                let z = Vec2::new(x as f64, y as f64);
                let p = Vec2::new(radial.center.x.get(), radial.center.y.get());
                let q =
                    Vec2::new(radial.focal_center.x.get(), radial.focal_center.y.get());

                if (z - q).hypot() < fr {
                    0.0
                } else if (z - p).hypot() > cr {
                    1.0
                } else {
                    let uz = (z - q).normalize();
                    let az = (q - p).dot(uz);
                    let rho = cr.powi(2) - (q - p).hypot().powi(2);
                    let bz = (az.powi(2) + rho).sqrt() - az;

                    ((z - q).hypot() - fr) / (bz - fr)
                }
            }
            Self::Conic(conic) => {
                let (x, y) =
                    (x as f64 - conic.center.x.get(), y as f64 - conic.center.y.get());
                let angle = Gradient::correct_aspect_ratio(
                    conic.angle,
                    Ratio::new((width / height) as f64),
                );
                ((-y.atan2(x) + PI + angle.to_rad()) % TAU) / TAU
            }
        };

        self.sample(RatioOrAngle::Ratio(Ratio::new(t.clamp(0.0, 1.0))))
    }

    /// Does this gradient need to be anti-aliased?
    pub fn anti_alias(&self) -> bool {
        match self {
            Self::Linear(linear) => linear.anti_alias,
            Self::Radial(radial) => radial.anti_alias,
            Self::Conic(conic) => conic.anti_alias,
        }
    }

    /// Returns the relative placement of this gradient, handling
    /// the special case of `auto`.
    pub fn unwrap_relative(&self, on_text: bool) -> RelativeTo {
        self.relative().unwrap_or_else(|| {
            if on_text { RelativeTo::Parent } else { RelativeTo::Self_ }
        })
    }

    /// Corrects this angle for the aspect ratio of a gradient.
    ///
    /// This is used specifically for gradients.
    pub fn correct_aspect_ratio(angle: Angle, aspect_ratio: Ratio) -> Angle {
        let rad = (angle.to_rad().rem_euclid(TAU).tan() / aspect_ratio.get()).atan();
        let rad = match angle.quadrant() {
            Quadrant::First => rad,
            Quadrant::Second => rad + PI,
            Quadrant::Third => rad + PI,
            Quadrant::Fourth => rad + TAU,
        };
        Angle::rad(rad.rem_euclid(TAU))
    }
}

impl Debug for Gradient {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        match self {
            Self::Linear(v) => v.fmt(f),
            Self::Radial(v) => v.fmt(f),
            Self::Conic(v) => v.fmt(f),
        }
    }
}

impl Repr for Gradient {
    fn repr(&self) -> EcoString {
        match self {
            Self::Radial(radial) => radial.repr(),
            Self::Linear(linear) => linear.repr(),
            Self::Conic(conic) => conic.repr(),
        }
    }
}

/// A gradient that interpolates between two colors along an axis.
#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub struct LinearGradient {
    /// The color stops of this gradient.
    pub stops: Vec<(Color, Ratio)>,
    /// The direction of this gradient.
    pub angle: Angle,
    /// The color space in which to interpolate the gradient.
    pub space: ColorSpace,
    /// The relative placement of the gradient.
    pub relative: Smart<RelativeTo>,
    /// Whether to anti-alias the gradient (used for sharp gradients).
    pub anti_alias: bool,
}

impl Repr for LinearGradient {
    fn repr(&self) -> EcoString {
        let mut r = EcoString::from("gradient.linear(");

        let angle = self.angle.to_rad().rem_euclid(TAU);
        if angle.abs() < f64::EPSILON {
            // Default value, do nothing
        } else if (angle - FRAC_PI_2).abs() < f64::EPSILON {
            r.push_str("dir: rtl, ");
        } else if (angle - PI).abs() < f64::EPSILON {
            r.push_str("dir: ttb, ");
        } else if (angle - 3.0 * FRAC_PI_2).abs() < f64::EPSILON {
            r.push_str("dir: btt, ");
        } else {
            r.push_str("angle: ");
            r.push_str(&self.angle.repr());
            r.push_str(", ");
        }

        if self.space != ColorSpace::Oklab {
            r.push_str("space: ");
            r.push_str(&self.space.into_value().repr());
            r.push_str(", ");
        }

        if self.relative.is_custom() {
            r.push_str("relative: ");
            r.push_str(&self.relative.into_value().repr());
            r.push_str(", ");
        }

        for (i, (color, offset)) in self.stops.iter().enumerate() {
            r.push('(');
            r.push_str(&color.repr());
            r.push_str(", ");
            r.push_str(&offset.repr());
            r.push(')');
            if i != self.stops.len() - 1 {
                r.push_str(", ");
            }
        }

        r.push(')');
        r
    }
}

/// A gradient that interpolates between two colors along a circle.
#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub struct RadialGradient {
    /// The color stops of this gradient.
    pub stops: Vec<(Color, Ratio)>,
    /// The center of last circle of this gradient.
    pub center: Axes<Ratio>,
    /// The radius of last circle of this gradient.
    pub radius: Ratio,
    /// The center of first circle of this gradient.
    pub focal_center: Axes<Ratio>,
    /// The radius of first circle of this gradient.
    pub focal_radius: Ratio,
    /// The color space in which to interpolate the gradient.
    pub space: ColorSpace,
    /// The relative placement of the gradient.
    pub relative: Smart<RelativeTo>,
    /// Whether to anti-alias the gradient (used for sharp gradients).
    pub anti_alias: bool,
}

impl Repr for RadialGradient {
    fn repr(&self) -> EcoString {
        let mut r = EcoString::from("gradient.radial(");

        if self.center.x != Ratio::new(0.5) || self.center.y != Ratio::new(0.5) {
            r.push_str("center: (");
            r.push_str(&self.center.x.repr());
            r.push_str(", ");
            r.push_str(&self.center.y.repr());
            r.push_str("), ");
        }

        if self.radius != Ratio::new(0.5) {
            r.push_str("radius: ");
            r.push_str(&self.radius.repr());
            r.push_str(", ");
        }

        if self.focal_center != self.center {
            r.push_str("focal-center: (");
            r.push_str(&self.focal_center.x.repr());
            r.push_str(", ");
            r.push_str(&self.focal_center.y.repr());
            r.push_str("), ");
        }

        if self.focal_radius != Ratio::zero() {
            r.push_str("focal-radius: ");
            r.push_str(&self.focal_radius.repr());
            r.push_str(", ");
        }

        if self.space != ColorSpace::Oklab {
            r.push_str("space: ");
            r.push_str(&self.space.into_value().repr());
            r.push_str(", ");
        }

        if self.relative.is_custom() {
            r.push_str("relative: ");
            r.push_str(&self.relative.into_value().repr());
            r.push_str(", ");
        }

        for (i, (color, offset)) in self.stops.iter().enumerate() {
            r.push('(');
            r.push_str(&color.repr());
            r.push_str(", ");
            r.push_str(&offset.repr());
            r.push(')');
            if i != self.stops.len() - 1 {
                r.push_str(", ");
            }
        }

        r.push(')');
        r
    }
}

/// A gradient that interpolates between two colors radially
/// around a center point.
#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub struct ConicGradient {
    /// The color stops of this gradient.
    pub stops: Vec<(Color, Ratio)>,
    /// The direction of this gradient.
    pub angle: Angle,
    /// The center of circle of this gradient.
    pub center: Axes<Ratio>,
    /// The color space in which to interpolate the gradient.
    pub space: ColorSpace,
    /// The relative placement of the gradient.
    pub relative: Smart<RelativeTo>,
    /// Whether to anti-alias the gradient (used for sharp gradients).
    pub anti_alias: bool,
}

impl Repr for ConicGradient {
    fn repr(&self) -> EcoString {
        let mut r = EcoString::from("gradient.conic(");

        let angle = self.angle.to_rad().rem_euclid(TAU);
        if angle.abs() > f64::EPSILON {
            r.push_str("angle: ");
            r.push_str(&self.angle.repr());
            r.push_str(", ");
        }

        if self.center.x != Ratio::new(0.5) || self.center.y != Ratio::new(0.5) {
            r.push_str("center: (");
            r.push_str(&self.center.x.repr());
            r.push_str(", ");
            r.push_str(&self.center.y.repr());
            r.push_str("), ");
        }

        if self.space != ColorSpace::Oklab {
            r.push_str("space: ");
            r.push_str(&self.space.into_value().repr());
            r.push_str(", ");
        }

        if self.relative.is_custom() {
            r.push_str("relative: ");
            r.push_str(&self.relative.into_value().repr());
            r.push_str(", ");
        }

        for (i, (color, offset)) in self.stops.iter().enumerate() {
            r.push('(');
            r.push_str(&color.repr());
            r.push_str(", ");
            r.push_str(&Angle::deg(offset.get() * 360.0).repr());
            r.push(')');
            if i != self.stops.len() - 1 {
                r.push_str(", ");
            }
        }

        r.push(')');
        r
    }
}

/// What is the gradient relative to.
#[derive(Cast, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum RelativeTo {
    /// The gradient is relative to itself (its own bounding box).
    Self_,
    /// The gradient is relative to its parent (the parent's bounding box).
    Parent,
}

/// A color stop.
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub struct GradientStop {
    /// The color for this stop.
    pub color: Color,
    /// The offset of the stop along the gradient.
    pub offset: Option<Ratio>,
}

impl GradientStop {
    /// Create a new stop from a `color` and an `offset`.
    pub fn new(color: Color, offset: Ratio) -> Self {
        Self { color, offset: Some(offset) }
    }
}

cast! {
    GradientStop,
    self => if let Some(offset) = self.offset {
        array![self.color.into_value(), offset].into_value()
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

/// A ratio or an angle.
#[derive(Copy, Clone, PartialEq, Eq, Hash)]
pub enum RatioOrAngle {
    Ratio(Ratio),
    Angle(Angle),
}

impl RatioOrAngle {
    pub fn to_ratio(self) -> Ratio {
        match self {
            Self::Ratio(ratio) => ratio,
            Self::Angle(angle) => Ratio::new(angle.to_rad().rem_euclid(TAU) / TAU),
        }
        .clamp(Ratio::zero(), Ratio::one())
    }
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

/// Pre-processes the stops, checking that they are valid and computing the
/// offsets if necessary.
///
/// Returns an error if the stops are invalid.
///
/// This is split into its own function because it is used by all of the
/// different gradient types.
#[comemo::memoize]
fn process_stops(stops: &[Spanned<GradientStop>]) -> SourceResult<Vec<(Color, Ratio)>> {
    let has_offset = stops.iter().any(|stop| stop.v.offset.is_some());
    if has_offset {
        let mut last_stop = f64::NEG_INFINITY;
        for Spanned { v: stop, span } in stops.iter() {
            let Some(stop) = stop.offset else {
                bail!(
                    *span, "either all stops must have an offset or none of them can";
                    hint: "try adding an offset to all stops"
                );
            };

            if stop.get() < last_stop {
                bail!(*span, "offsets must be in monotonic order");
            }

            last_stop = stop.get();
        }

        let out = stops
            .iter()
            .map(|Spanned { v: GradientStop { color, offset }, span }| {
                if offset.unwrap().get() > 1.0 || offset.unwrap().get() < 0.0 {
                    bail!(*span, "offset must be between 0 and 1");
                }
                Ok((*color, offset.unwrap()))
            })
            .collect::<SourceResult<Vec<_>>>()?;

        if out[0].1 != Ratio::zero() {
            bail!(
                stops[0].span,
                "first stop must have an offset of 0";
                hint: "try setting this stop to `0%`"
            );
        }

        if out[out.len() - 1].1 != Ratio::one() {
            bail!(
                stops[out.len() - 1].span,
                "last stop must have an offset of 100%";
                hint: "try setting this stop to `100%`"
            );
        }

        return Ok(out);
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

/// Sample the stops at a given position.
fn sample_stops(stops: &[(Color, Ratio)], mixing_space: ColorSpace, t: f64) -> Color {
    let t = t.clamp(0.0, 1.0);
    let mut j = stops.partition_point(|(_, ratio)| ratio.get() < t);

    if j == 0 {
        while stops.get(j + 1).is_some_and(|(_, r)| r.is_zero()) {
            j += 1;
        }
        return stops[j].0;
    }

    let (col_0, pos_0) = stops[j - 1];
    let (col_1, pos_1) = stops[j];
    let t = (t - pos_0.get()) / (pos_1.get() - pos_0.get());

    Color::mix_iter(
        [WeightedColor::new(col_0, 1.0 - t), WeightedColor::new(col_1, t)],
        mixing_space,
    )
    .unwrap()
}
