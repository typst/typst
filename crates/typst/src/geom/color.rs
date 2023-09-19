use std::str::FromStr;

use ecow::{eco_format, EcoString};
use palette::encoding::{self, Linear};
use palette::{Darken, Desaturate, FromColor, Lighten, RgbHue, Saturate, ShiftHue};
use typst_syntax::Span;

use super::scalar::F32Scalar;
use super::*;
use crate::diag::{bail, At, SourceDiagnostic, SourceResult};
use crate::eval::{cast, Args, Array, Str};
use crate::syntax::Spanned;

// Type aliases for `palette` internal types in f32.
type Oklab = palette::oklab::Oklaba<f32>;
type LinearRgba = palette::rgb::Rgba<Linear<encoding::Srgb>, f32>;
type Rgba = palette::rgb::Rgba<encoding::Srgb, f32>;
type Hsl = palette::hsl::Hsla<encoding::Srgb, f32>;
type Hsv = palette::hsv::Hsva<encoding::Srgb, f32>;
type Luma = palette::luma::Luma<encoding::Srgb, f32>;

/// A color in a specific color space.
///
/// Typst supports:
/// - sRGB through the [`rgb` function]($rgb)
/// - Device CMYK through [`cmyk` function]($cmyk)
/// - D65 Gray through the [`luma` function]($luma)
/// - Oklab through the [`oklab` function]($oklab)
/// - Linear RGB through the [`color.linear-rgb` function]($color.linear-rgb)
/// - HSL through the [`color.hsl` function]($color.hsl)
/// - HSV through the [`color.hsv` function]($color.hsv)
///
/// Typst provides the following built-in colors:
///
/// `black`, `gray`, `silver`, `white`, `navy`, `blue`, `aqua`, `teal`,
/// `eastern`, `purple`, `fuchsia`, `maroon`, `red`, `orange`, `yellow`,
/// `olive`, `green`, and `lime`.
///
/// # Example
/// The predefined colors and the color constructors are available globally and
/// also in the color type's scope, so you can write either of the following
/// two:
/// ```example
/// #rect(fill: aqua)
/// #rect(fill: color.aqua)
/// ```
#[derive(Copy, Clone)]
#[ty(scope)]
pub enum Color {
    /// A 32-bit luma color.
    Luma(Luma),
    /// A 32-bit L*a*b* color in the Oklab color space.
    Oklab(Oklab),
    /// A 32-bit RGBA color.
    Rgba(Rgba),
    /// A 32-bit linear RGB color.
    LinearRgb(LinearRgba),
    /// A 32-bit CMYK color.
    Cmyk(Cmyk),
    /// A 32-bit HSL color.
    Hsl(Hsl),
    /// A 32-bit HSV color.
    Hsv(Hsv),
}

impl PartialEq for Color {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            // Lower precision for comparison to avoid rounding errors.
            // Keeps backward compatibility with previous versions of Typst.
            (Self::Rgba(_), Self::Rgba(_)) => self.to_hex() == other.to_hex(),
            (Self::Luma(l0), Self::Luma(r0)) => l0 == r0,
            (Self::Oklab(l0), Self::Oklab(r0)) => l0 == r0,
            (Self::LinearRgb(l0), Self::LinearRgb(r0)) => l0 == r0,
            (Self::Cmyk(l0), Self::Cmyk(r0)) => l0 == r0,
            (Self::Hsl(l0), Self::Hsl(r0)) => l0 == r0,
            (Self::Hsv(l0), Self::Hsv(r0)) => l0 == r0,
            _ => false,
        }
    }
}

impl Eq for Color {}

impl Hash for Color {
    fn hash<H: Hasher>(&self, state: &mut H) {
        core::mem::discriminant(self).hash(state);

        let [x, y, z, w] = self.to_vec4();
        x.to_bits().hash(state);
        y.to_bits().hash(state);
        z.to_bits().hash(state);
        w.to_bits().hash(state);
    }
}

#[scope]
impl Color {
    pub const BLACK: Self = Self::Luma(Luma::new(0.0));
    pub const GRAY: Self = Self::Luma(Luma::new(0.6666666));
    pub const WHITE: Self = Self::Luma(Luma::new(1.0));
    pub const SILVER: Self = Self::Luma(Luma::new(0.8666667));
    pub const NAVY: Self = Self::Rgba(Rgba::new(0.0, 0.121569, 0.247059, 1.0));
    pub const BLUE: Self = Self::Rgba(Rgba::new(0.0, 0.454902, 0.85098, 1.0));
    pub const AQUA: Self = Self::Rgba(Rgba::new(0.4980392, 0.858823, 1.0, 1.0));
    pub const TEAL: Self = Self::Rgba(Rgba::new(0.223529, 0.8, 0.8, 1.0));
    pub const EASTERN: Self = Self::Rgba(Rgba::new(0.13725, 0.615686, 0.678431, 1.0));
    pub const PURPLE: Self = Self::Rgba(Rgba::new(0.694118, 0.050980, 0.788235, 1.0));
    pub const FUCHSIA: Self = Self::Rgba(Rgba::new(0.941177, 0.070588, 0.745098, 1.0));
    pub const MAROON: Self = Self::Rgba(Rgba::new(0.521569, 0.078431, 0.294118, 1.0));
    pub const RED: Self = Self::Rgba(Rgba::new(1.0, 0.254902, 0.211765, 1.0));
    pub const ORANGE: Self = Self::Rgba(Rgba::new(1.0, 0.521569, 0.105882, 1.0));
    pub const YELLOW: Self = Self::Rgba(Rgba::new(1.0, 0.8627451, 0.0, 1.0));
    pub const OLIVE: Self = Self::Rgba(Rgba::new(0.239216, 0.6, 0.4392157, 1.0));
    pub const GREEN: Self = Self::Rgba(Rgba::new(0.1803922, 0.8, 0.2509804, 1.0));
    pub const LIME: Self = Self::Rgba(Rgba::new(0.0039216, 1.0, 0.4392157, 1.0));

    /// Create a grayscale color.
    ///
    /// A grayscale color is represented internally by a single `lightness` component.
    ///
    /// These components are also available using the [`components`]($color.components)
    /// method.
    ///
    /// ```example
    /// #for x in range(250, step: 50) {
    ///   box(square(fill: luma(x)))
    /// }
    /// ```
    #[func]
    pub fn luma(
        /// The real arguments (the other arguments are just for the docs, this
        /// function is a bit involved, so we parse the arguments manually).
        args: Args,
        /// The lightness component.
        #[external]
        lightness: Component,
        /// The color to convert to grayscale.
        #[external]
        color: Color,
    ) -> SourceResult<Color> {
        let mut args = args;
        Ok(if let Some(color) = args.find::<Color>()? {
            color.to_luma()
        } else {
            let Component(gray) =
                args.expect("gray component").unwrap_or(Component(Ratio::one()));
            Self::Luma(Luma::new(gray.get() as f32))
        })
    }

    /// Create an [Oklab](https://bottosson.github.io/posts/oklab/) color.
    ///
    /// This color space is well suited for the following use cases:
    /// - Color manipulation such as saturating while keeping perceived hue
    /// - Creating grayscale images with uniform perceived lightness
    /// - Creating smooth and uniform color transition and gradients
    ///
    /// A linear Oklab color is represented internally by an array of four components:
    /// - lightness ([`ratio`]($ratio))
    /// - a ([`float`]($float) in the range `[-0.4..0.4]`)
    /// - b ([`float`]($float) in the range `[-0.4..0.4]`)
    /// - alpha ([`ratio`]($ratio))
    ///
    /// These components are also available using the [`components`]($color.components)
    /// method.
    ///
    /// ```example
    /// #square(
    ///   fill: oklab(27%, 20%, -3%, 50%)
    /// )
    /// ```
    #[func]
    pub fn oklab(
        /// The real arguments (the other arguments are just for the docs, this
        /// function is a bit involved, so we parse the arguments manually).
        args: Args,
        /// The cyan component.
        #[external]
        lightness: RatioComponent,
        /// The magenta component.
        #[external]
        a: ABComponent,
        /// The yellow component.
        #[external]
        b: ABComponent,
        /// The key component.
        #[external]
        alpha: RatioComponent,
        /// The color to convert to Oklab.
        #[external]
        color: Color,
    ) -> SourceResult<Color> {
        let mut args = args;
        Ok(if let Some(color) = args.find::<Color>()? {
            color.to_oklab()
        } else {
            let RatioComponent(l) = args.expect("lightness component")?;
            let ABComponent(a) = args.expect("A component")?;
            let ABComponent(b) = args.expect("B component")?;
            let RatioComponent(alpha) =
                args.eat()?.unwrap_or(RatioComponent(Ratio::one()));
            Self::Oklab(Oklab::new(
                l.get() as f32,
                a.get() as f32,
                b.get() as f32,
                alpha.get() as f32,
            ))
        })
    }

    /// Create an RGB(A) color with linear luma.
    ///
    /// This color space is similar to Srgb<f32>, but with the distinction that
    /// the color component are not gamma corrected. This makes it easier to
    /// perform color operations such as blending and interpolation. Although,
    /// you should prefer to use the [`oklab` function]($oklab) for these.
    ///
    /// A linear RGB(A) color is represented internally by an array of four components:
    /// - red ([`ratio`]($ratio))
    /// - green ([`ratio`]($ratio))
    /// - blue ([`ratio`]($ratio))
    /// - alpha ([`ratio`]($ratio))
    ///
    /// These components are also available using the [`components`]($color.components)
    /// method.
    ///
    /// ```example
    /// #square(
    ///   fill: color.linear-rgb(30%, 50%, 10%)
    /// )
    /// ```
    #[func(title = "Linear RGB")]
    pub fn linear_rgb(
        /// The real arguments (the other arguments are just for the docs, this
        /// function is a bit involved, so we parse the arguments manually).
        args: Args,
        /// The red component.
        #[external]
        red: Component,
        /// The green component.
        #[external]
        green: Component,
        /// The blue component.
        #[external]
        blue: Component,
        /// The alpha component.
        #[external]
        alpha: Component,
        /// The color to convert to linear RGB(A).
        #[external]
        color: Color,
    ) -> SourceResult<Color> {
        let mut args = args;
        Ok(if let Some(color) = args.find::<Color>()? {
            color.to_linear_rgb()
        } else {
            let Component(r) = args.expect("red component")?;
            let Component(g) = args.expect("green component")?;
            let Component(b) = args.expect("blue component")?;
            let Component(a) = args.eat()?.unwrap_or(Component(Ratio::one()));
            Self::LinearRgb(LinearRgba::new(
                r.get() as f32,
                g.get() as f32,
                b.get() as f32,
                a.get() as f32,
            ))
        })
    }

    /// Create an RGB(A) color.
    ///
    /// The color is specified in the sRGB color space.
    ///
    /// An RGB(A) color is represented internally by an array of four components:
    /// - red ([`ratio`]($ratio))
    /// - green ([`ratio`]($ratio))
    /// - blue ([`ratio`]($ratio))
    /// - alpha ([`ratio`]($ratio))
    ///
    /// These components are also available using the [`components`]($color.components)
    /// method.
    ///
    /// ```example
    /// #square(fill: rgb("#b1f2eb"))
    /// #square(fill: rgb(87, 127, 230))
    /// #square(fill: rgb(25%, 13%, 65%))
    /// ```
    #[func(title = "RGB")]
    pub fn rgb(
        /// The real arguments (the other arguments are just for the docs, this
        /// function is a bit involved, so we parse the arguments manually).
        args: Args,
        /// The color in hexadecimal notation.
        ///
        /// Accepts three, four, six or eight hexadecimal digits and optionally
        /// a leading hashtag.
        ///
        /// If this string is given, the individual components should not be given.
        ///
        /// ```example
        /// #text(16pt, rgb("#239dad"))[
        ///   *Typst*
        /// ]
        /// ```
        #[external]
        hex: Str,
        /// The red component.
        #[external]
        red: Component,
        /// The green component.
        #[external]
        green: Component,
        /// The blue component.
        #[external]
        blue: Component,
        /// The alpha component.
        #[external]
        alpha: Component,
        /// The color to convert to RGB(A).
        #[external]
        color: Color,
    ) -> SourceResult<Color> {
        let mut args = args;
        Ok(if let Some(string) = args.find::<Spanned<Str>>()? {
            Self::from_str(&string.v).at(string.span)?
        } else if let Some(color) = args.find::<Color>()? {
            color.to_rgba()
        } else {
            let Component(r) = args.expect("red component")?;
            let Component(g) = args.expect("green component")?;
            let Component(b) = args.expect("blue component")?;
            let Component(a) = args.eat()?.unwrap_or(Component(Ratio::one()));
            Self::Rgba(Rgba::new(
                r.get() as f32,
                g.get() as f32,
                b.get() as f32,
                a.get() as f32,
            ))
        })
    }

    /// Create a CMYK color.
    ///
    /// This is useful if you want to target a specific printer. The conversion
    /// to RGB for display preview might differ from how your printer reproduces
    /// the color.
    ///
    /// An HSL color is represented internally by an array of four components:
    /// - cyan ([`ratio`]($ratio))
    /// - magenta ([`ratio`]($ratio))
    /// - yellow ([`ratio`]($ratio))
    /// - key ([`ratio`]($ratio))
    ///
    /// These components are also available using the [`components`]($color.components)
    /// method.
    ///
    /// ```example
    /// #square(
    ///   fill: cmyk(27%, 0%, 3%, 5%)
    /// )
    /// ```
    #[func(title = "CMYK")]
    pub fn cmyk(
        /// The real arguments (the other arguments are just for the docs, this
        /// function is a bit involved, so we parse the arguments manually).
        args: Args,
        /// The cyan component.
        #[external]
        cyan: RatioComponent,
        /// The magenta component.
        #[external]
        magenta: RatioComponent,
        /// The yellow component.
        #[external]
        yellow: RatioComponent,
        /// The key component.
        #[external]
        key: RatioComponent,
        /// The color to convert to CMYK.
        #[external]
        color: Color,
    ) -> SourceResult<Color> {
        let mut args = args;
        Ok(if let Some(color) = args.find::<Color>()? {
            color.to_cmyk()
        } else {
            let RatioComponent(c) = args.expect("cyan component")?;
            let RatioComponent(m) = args.expect("magenta component")?;
            let RatioComponent(y) = args.expect("yellow component")?;
            let RatioComponent(k) = args.expect("key/black component")?;
            Self::Cmyk(Cmyk::new(
                c.get() as f32,
                m.get() as f32,
                y.get() as f32,
                k.get() as f32,
            ))
        })
    }

    /// Create an HSL color.
    ///
    /// This color space is useful for specifying colors by hue, saturation and
    /// lightness. It is also useful for color manipulation, such as saturating
    /// while keeping perceived hue.
    ///
    /// An HSL color is represented internally by an array of four components:
    /// - hue ([`angle`]($angle))
    /// - saturation ([`ratio`]($ratio))
    /// - lightness ([`ratio`]($ratio))
    /// - alpha ([`ratio`]($ratio))
    ///
    /// These components are also available using the [`components`]($color.components)
    /// method.
    ///
    /// ```example
    /// #square(
    ///   fill: color.hsl(30deg, 50%, 60%)
    /// )
    /// ```
    #[func(title = "HSL")]
    pub fn hsl(
        /// The real arguments (the other arguments are just for the docs, this
        /// function is a bit involved, so we parse the arguments manually).
        args: Args,
        /// The hue angle.
        #[external]
        hue: Angle,
        /// The saturation component.
        #[external]
        saturation: Component,
        /// The lightness component.
        #[external]
        lightness: Component,
        /// The alpha component.
        #[external]
        alpha: Component,
        /// The color to convert to HSL.
        #[external]
        color: Color,
    ) -> SourceResult<Color> {
        let mut args = args;
        Ok(if let Some(color) = args.find::<Color>()? {
            color.to_hsl()
        } else {
            let h: Angle = args.expect("hue component")?;
            let Component(s) = args.expect("saturation component")?;
            let Component(l) = args.expect("lightness component")?;
            let Component(a) = args.eat()?.unwrap_or(Component(Ratio::one()));
            Self::Hsl(Hsl::new(
                RgbHue::from_degrees(h.to_deg() as f32),
                s.get() as f32,
                l.get() as f32,
                a.get() as f32,
            ))
        })
    }

    /// Create an HSV color.
    ///
    /// This color space is useful for specifying colors by hue, saturation and
    /// value. It is also useful for color manipulation, such as saturating
    /// while keeping perceived hue.
    ///
    /// An HSV color is represented internally by an array of four components:
    /// - hue ([`angle`]($angle))
    /// - saturation ([`ratio`]($ratio))
    /// - value ([`ratio`]($ratio))
    /// - alpha ([`ratio`]($ratio))
    ///
    /// These components are also available using the [`components`]($color.components)
    /// method.
    ///
    /// ```example
    /// #square(
    ///   fill: color.hsv(30deg, 50%, 60%)
    /// )
    /// ```
    #[func(title = "HSV")]
    pub fn hsv(
        /// The real arguments (the other arguments are just for the docs, this
        /// function is a bit involved, so we parse the arguments manually).
        args: Args,
        /// The hue angle.
        #[external]
        hue: Angle,
        /// The saturation component.
        #[external]
        saturation: Component,
        /// The value component.
        #[external]
        value: Component,
        /// The alpha component.
        #[external]
        alpha: Component,
        /// The color to convert to HSL.
        #[external]
        color: Color,
    ) -> SourceResult<Color> {
        let mut args = args;
        Ok(if let Some(color) = args.find::<Color>()? {
            color.to_hsv()
        } else {
            let h: Angle = args.expect("hue component")?;
            let Component(s) = args.expect("saturation component")?;
            let Component(v) = args.expect("value component")?;
            let Component(a) = args.eat()?.unwrap_or(Component(Ratio::one()));
            Self::Hsv(Hsv::new(
                RgbHue::from_degrees(h.to_deg() as f32),
                s.get() as f32,
                v.get() as f32,
                a.get() as f32,
            ))
        })
    }

    /// Converts this color into its components.
    ///
    /// The size and values of this array depends on the color space. You can
    /// obtain the color space using [`space`]($color.space). Below is a table of
    /// the color spaces and their components:
    ///
    /// |       Color space       |     C1    |     C2     |     C3    |   C4   |
    /// |-------------------------|-----------|------------|-----------|--------|
    /// | [`luma`]($color.luma)   | Lightness |            |           |        |
    /// | [`oklab`]($color.oklab) | Lightness |    `a`     |    `b`    |  Alpha |
    /// | [`linear-rgb`]($color.linear-rgb) | Red  |   Green |    Blue |  Alpha |
    /// | [`rgb`]($color.rgb)     |    Red    |   Green    |    Blue   |  Alpha |
    /// | [`cmyk`]($color.cmyk)   |    Cyan   |   Magenta  |   Yellow  |  Key   |
    /// | [`hsl`]($color.hsl)     |     Hue   | Saturation | Lightness |  Alpha |
    /// | [`hsv`]($color.hsv)     |     Hue   | Saturation |   Value   |  Alpha |
    ///
    /// For the meaning and type of each individual value, see the documentation of
    /// the corresponding color space. The alpha component is optional and only
    /// included if the `alpha` argument is `true`. The length of the returned array
    /// depends on the number of components and whether the alpha component is
    /// included.
    ///
    /// ```example
    /// // note that the alpha component is included by default
    /// #(rgb(40%, 60%, 80%).components() == (40%, 60%, 80%, 100%))
    /// ```
    #[func]
    pub fn components(
        self,
        /// Whether to include the alpha component.
        #[default(true)]
        alpha: bool,
    ) -> Array {
        match self {
            Self::Luma(c) => array![Ratio::new(c.luma as f64)],
            Self::Oklab(c) => {
                if alpha {
                    array![
                        Ratio::new(c.l as f64),
                        (c.a as f64 * 1000.0).round() / 1000.0,
                        (c.b as f64 * 1000.0).round() / 1000.0,
                        Ratio::new(c.alpha as f64),
                    ]
                } else {
                    array![
                        Ratio::new(c.l as f64),
                        (c.a as f64 * 1000.0).round() / 1000.0,
                        (c.b as f64 * 1000.0).round() / 1000.0,
                    ]
                }
            }
            Self::LinearRgb(c) => {
                if alpha {
                    array![
                        Ratio::new(c.red as f64),
                        Ratio::new(c.green as f64),
                        Ratio::new(c.blue as f64),
                        Ratio::new(c.alpha as f64),
                    ]
                } else {
                    array![
                        Ratio::new(c.red as f64),
                        Ratio::new(c.green as f64),
                        Ratio::new(c.blue as f64),
                    ]
                }
            }
            Self::Rgba(c) => {
                if alpha {
                    array![
                        Ratio::new(c.red as f64),
                        Ratio::new(c.green as f64),
                        Ratio::new(c.blue as f64),
                        Ratio::new(c.alpha as f64),
                    ]
                } else {
                    array![
                        Ratio::new(c.red as f64),
                        Ratio::new(c.green as f64),
                        Ratio::new(c.blue as f64),
                    ]
                }
            }
            Self::Cmyk(c) => array![
                Ratio::new(c.c.get() as f64),
                Ratio::new(c.m.get() as f64),
                Ratio::new(c.y.get() as f64),
                Ratio::new(c.k.get() as f64),
            ],
            Self::Hsl(c) => {
                if alpha {
                    array![
                        Angle::deg(c.hue.into_degrees().rem_euclid(360.0) as f64),
                        Ratio::new(c.saturation as f64),
                        Ratio::new(c.lightness as f64),
                        Ratio::new(c.alpha as f64),
                    ]
                } else {
                    array![
                        Angle::deg(c.hue.into_degrees().rem_euclid(360.0) as f64),
                        Ratio::new(c.saturation as f64),
                        Ratio::new(c.lightness as f64),
                    ]
                }
            }
            Self::Hsv(c) => {
                if alpha {
                    array![
                        Angle::deg(c.hue.into_degrees().rem_euclid(360.0) as f64),
                        Ratio::new(c.saturation as f64),
                        Ratio::new(c.value as f64),
                        Ratio::new(c.alpha as f64),
                    ]
                } else {
                    array![
                        Angle::deg(c.hue.into_degrees().rem_euclid(360.0) as f64),
                        Ratio::new(c.saturation as f64),
                        Ratio::new(c.value as f64),
                    ]
                }
            }
        }
    }

    /// Returns the constructor function for this color's space:
    /// - [`oklab`]($color.oklab)
    /// - [`luma`]($color.luma)
    /// - [`linear-rgb`]($color.linear-rgb)
    /// - [`rgb`]($color.rgb)
    /// - [`cmyk`]($color.cmyk)
    /// - [`hsl`]($color.hsl)
    /// - [`hsv`]($color.hsv)
    ///
    /// ```example
    /// #let color = cmyk(1%, 2%, 3%, 4%)
    /// #(color.space() == cmyk)
    /// ```
    #[func]
    pub fn space(self) -> ColorSpace {
        match self {
            Self::Luma(_) => ColorSpace::D65Gray,
            Self::Oklab(_) => ColorSpace::Oklab,
            Self::LinearRgb(_) => ColorSpace::LinearRgb,
            Self::Rgba(_) => ColorSpace::Srgb,
            Self::Cmyk(_) => ColorSpace::Cmyk,
            Self::Hsl(_) => ColorSpace::Hsl,
            Self::Hsv(_) => ColorSpace::Hsv,
        }
    }

    /// Returns the color's RGB(A) hex representation (such as `#ffaa32` or
    /// `#020304fe`). The alpha component (last two digits in `#020304fe`) is
    /// omitted if it is equal to `ff` (255 / 100%).
    #[func]
    pub fn to_hex(self) -> EcoString {
        let [r, g, b, a] = self.to_rgba().to_vec4_u8();
        if a != 255 {
            eco_format!("#{:02x}{:02x}{:02x}{:02x}", r, g, b, a)
        } else {
            eco_format!("#{:02x}{:02x}{:02x}", r, g, b)
        }
    }

    /// Lightens a color by a given factor.
    #[func]
    pub fn lighten(
        self,
        /// The factor to lighten the color by.
        factor: Ratio,
    ) -> Color {
        let factor = factor.get() as f32;
        match self {
            Self::Luma(c) => Self::Luma(c.lighten(factor)),
            Self::Oklab(c) => Self::Oklab(c.lighten(factor)),
            Self::LinearRgb(c) => Self::LinearRgb(c.lighten(factor)),
            Self::Rgba(c) => Self::Rgba(c.lighten(factor)),
            Self::Cmyk(c) => Self::Cmyk(c.lighten(factor)),
            Self::Hsl(c) => Self::Hsl(c.lighten(factor)),
            Self::Hsv(c) => Self::Hsv(c.lighten(factor)),
        }
    }

    /// Darkens a color by a given factor.
    #[func]
    pub fn darken(
        self,
        /// The factor to darken the color by.
        factor: Ratio,
    ) -> Color {
        let factor = factor.get() as f32;
        match self {
            Self::Luma(c) => Self::Luma(c.darken(factor)),
            Self::Oklab(c) => Self::Oklab(c.darken(factor)),
            Self::LinearRgb(c) => Self::LinearRgb(c.darken(factor)),
            Self::Rgba(c) => Self::Rgba(c.darken(factor)),
            Self::Cmyk(c) => Self::Cmyk(c.darken(factor)),
            Self::Hsl(c) => Self::Hsl(c.darken(factor)),
            Self::Hsv(c) => Self::Hsv(c.darken(factor)),
        }
    }

    /// Increases the saturation of a color by a given factor.
    #[func]
    pub fn saturate(
        self,
        /// The call span
        span: Span,
        /// The factor to saturate the color by.
        factor: Ratio,
    ) -> SourceResult<Color> {
        Ok(match self {
            Self::Luma(_) => {
                let mut diagnostic =
                    SourceDiagnostic::error(span, "cannot saturate grayscale color");
                diagnostic.hint("try converting your color to RGB first");
                return Err(Box::new(vec![diagnostic]));
            }
            Self::Oklab(_) => self.to_hsv().saturate(span, factor)?.to_oklab(),
            Self::LinearRgb(_) => self.to_hsv().saturate(span, factor)?.to_linear_rgb(),
            Self::Rgba(_) => self.to_hsv().saturate(span, factor)?.to_rgba(),
            Self::Cmyk(_) => self.to_hsv().saturate(span, factor)?.to_cmyk(),
            Self::Hsl(c) => Self::Hsl(c.saturate(factor.get() as f32)),
            Self::Hsv(c) => Self::Hsv(c.saturate(factor.get() as f32)),
        })
    }

    /// Decreases the saturation of a color by a given factor.
    #[func]
    pub fn desaturate(
        self,
        /// The call span
        span: Span,
        /// The factor to desaturate the color by.
        factor: Ratio,
    ) -> SourceResult<Color> {
        Ok(match self {
            Self::Luma(_) => {
                let mut diagnostic =
                    SourceDiagnostic::error(span, "cannot desaturate grayscale color");
                diagnostic.hint("try converting your color to RGB first");
                return Err(Box::new(vec![diagnostic]));
            }
            Self::Oklab(_) => self.to_hsv().desaturate(span, factor)?.to_oklab(),
            Self::LinearRgb(_) => self.to_hsv().desaturate(span, factor)?.to_linear_rgb(),
            Self::Rgba(_) => self.to_hsv().desaturate(span, factor)?.to_rgba(),
            Self::Cmyk(_) => self.to_hsv().desaturate(span, factor)?.to_cmyk(),
            Self::Hsl(c) => Self::Hsl(c.desaturate(factor.get() as f32)),
            Self::Hsv(c) => Self::Hsv(c.desaturate(factor.get() as f32)),
        })
    }

    /// Produces the negative of the color.
    #[func]
    pub fn negate(self) -> Color {
        match self {
            Self::Luma(c) => Self::Luma(Luma::new(1.0 - c.luma)),
            Self::Oklab(c) => Self::Oklab(Oklab::new(c.l, 1.0 - c.a, 1.0 - c.b, c.alpha)),
            Self::LinearRgb(c) => Self::LinearRgb(LinearRgba::new(
                1.0 - c.red,
                1.0 - c.green,
                1.0 - c.blue,
                c.alpha,
            )),
            Self::Rgba(c) => {
                Self::Rgba(Rgba::new(1.0 - c.red, 1.0 - c.green, 1.0 - c.blue, c.alpha))
            }
            Self::Cmyk(c) => Self::Cmyk(Cmyk::new(
                1.0 - c.c.get(),
                1.0 - c.m.get(),
                1.0 - c.y.get(),
                c.k.get(),
            )),
            Self::Hsl(c) => Self::Hsl(Hsl::new(
                RgbHue::from_degrees(360.0 - c.hue.into_degrees()),
                c.saturation,
                c.lightness,
                c.alpha,
            )),
            Self::Hsv(c) => Self::Hsv(Hsv::new(
                RgbHue::from_degrees(360.0 - c.hue.into_degrees()),
                c.saturation,
                c.value,
                c.alpha,
            )),
        }
    }

    /// Rotates the hue of the color by a given angle.
    #[func]
    pub fn rotate(
        self,
        /// The call span
        span: Span,
        /// The angle to rotate the hue by.
        angle: Angle,
    ) -> SourceResult<Color> {
        Ok(match self {
            Self::Luma(_) => {
                let mut diagnostic =
                    SourceDiagnostic::error(span, "cannot rotate grayscale color");
                diagnostic.hint("try converting your color to RGB first");
                return Err(Box::new(vec![diagnostic]));
            }
            Self::Oklab(_) => self.to_hsv().rotate(span, angle)?.to_oklab(),
            Self::LinearRgb(_) => self.to_hsv().rotate(span, angle)?.to_linear_rgb(),
            Self::Rgba(_) => self.to_hsv().rotate(span, angle)?.to_rgba(),
            Self::Cmyk(_) => self.to_hsv().rotate(span, angle)?.to_cmyk(),
            Self::Hsl(c) => Self::Hsl(c.shift_hue(angle.to_deg() as f32)),
            Self::Hsv(c) => Self::Hsv(c.shift_hue(angle.to_deg() as f32)),
        })
    }

    /// Create a color by mixing two or more colors.
    ///
    /// ```example
    /// #set block(height: 20pt, width: 100%)
    /// #block(fill: red.mix(blue))
    /// #block(fill: red.mix(blue, space: rgb))
    /// #block(fill: color.mix(red, blue, white))
    /// #block(fill: color.mix((red, 70%), (blue, 30%)))
    /// ```
    #[func]
    pub fn mix(
        /// The colors, optionally with weights, specified as a pair (array of
        /// length two) of color and weight (float or ratio).
        ///
        /// The weights do not need to add to `{100%}`, they are relative to the
        /// sum of all weights.
        #[variadic]
        colors: Vec<WeightedColor>,
        /// The color space to mix in. By default, this happens in a perceptual
        /// color space ([`oklab`]($color.oklab)).
        #[named]
        #[default(ColorSpace::Oklab)]
        space: ColorSpace,
    ) -> StrResult<Color> {
        let mut total = 0.0;
        let mut acc = [0.0; 4];

        for WeightedColor(color, weight) in colors.into_iter() {
            let weight = weight as f32;
            let v = color.to_space(space).to_vec4();
            acc[0] += weight * v[0];
            acc[1] += weight * v[1];
            acc[2] += weight * v[2];
            acc[3] += weight * v[3];
            total += weight;
        }

        if total <= 0.0 {
            bail!("sum of weights must be positive");
        }

        let m = acc.map(|v| v / total);
        Ok(match space {
            ColorSpace::Oklab => Color::Oklab(Oklab::new(m[0], m[1], m[2], m[3])),
            ColorSpace::Srgb => Color::Rgba(Rgba::new(m[0], m[1], m[2], m[3])),
            ColorSpace::LinearRgb => {
                Color::LinearRgb(LinearRgba::new(m[0], m[1], m[2], m[3]))
            }
            ColorSpace::Hsl => {
                Color::Hsl(Hsl::new(RgbHue::from_degrees(m[0]), m[1], m[2], m[3]))
            }
            ColorSpace::Hsv => {
                Color::Hsv(Hsv::new(RgbHue::from_degrees(m[0]), m[1], m[2], m[3]))
            }
            ColorSpace::Cmyk => Color::Cmyk(Cmyk::new(m[0], m[1], m[2], m[3])),
            ColorSpace::D65Gray => Color::Luma(Luma::new(m[0])),
        })
    }
}

impl Color {
    /// Construct a new RGBA color from 8-bit values.
    pub fn from_u8(r: u8, g: u8, b: u8, a: u8) -> Self {
        Self::Rgba(Rgba::new(
            r as f32 / 255.0,
            g as f32 / 255.0,
            b as f32 / 255.0,
            a as f32 / 255.0,
        ))
    }

    /// Converts a 32-bit integer to an RGBA color.
    #[inline]
    pub fn from_u32(color: u32) -> Self {
        Self::from_u8(
            ((color >> 24) & 0xFF) as u8,
            ((color >> 16) & 0xFF) as u8,
            ((color >> 8) & 0xFF) as u8,
            (color & 0xFF) as u8,
        )
    }

    pub fn alpha(&self) -> Option<f32> {
        match self {
            Color::Luma(_) | Color::Cmyk(_) => None,
            Color::Oklab(c) => Some(c.alpha),
            Color::Rgba(c) => Some(c.alpha),
            Color::LinearRgb(c) => Some(c.alpha),
            Color::Hsl(c) => Some(c.alpha),
            Color::Hsv(c) => Some(c.alpha),
        }
    }

    pub fn with_alpha(mut self, alpha: f32) -> Self {
        match &mut self {
            Color::Luma(_) | Color::Cmyk(_) => {}
            Color::Oklab(c) => c.alpha = alpha,
            Color::Rgba(c) => c.alpha = alpha,
            Color::LinearRgb(c) => c.alpha = alpha,
            Color::Hsl(c) => c.alpha = alpha,
            Color::Hsv(c) => c.alpha = alpha,
        }

        self
    }

    pub fn to_vec4(&self) -> [f32; 4] {
        match self {
            Color::Luma(c) => [c.luma; 4],
            Color::Oklab(c) => [c.l, c.a, c.b, c.alpha],
            Color::Rgba(c) => [c.red, c.green, c.blue, c.alpha],
            Color::LinearRgb(c) => [c.red, c.green, c.blue, c.alpha],
            Color::Cmyk(c) => [c.c.get(), c.m.get(), c.y.get(), c.k.get()],
            Color::Hsl(c) => [
                c.hue.into_degrees().rem_euclid(360.0),
                c.saturation,
                c.lightness,
                c.alpha,
            ],
            Color::Hsv(c) => {
                [c.hue.into_degrees().rem_euclid(360.0), c.saturation, c.value, c.alpha]
            }
        }
    }

    pub fn to_vec4_u8(&self) -> [u8; 4] {
        self.to_vec4().map(|x| (x * 255.0).round() as u8)
    }

    pub fn to_space(self, space: ColorSpace) -> Self {
        match space {
            ColorSpace::Oklab => self.to_oklab(),
            ColorSpace::Srgb => self.to_rgba(),
            ColorSpace::LinearRgb => self.to_linear_rgb(),
            ColorSpace::Hsl => self.to_hsl(),
            ColorSpace::Hsv => self.to_hsv(),
            ColorSpace::Cmyk => self.to_cmyk(),
            ColorSpace::D65Gray => self.to_luma(),
        }
    }

    pub fn to_luma(self) -> Self {
        Self::Luma(match self {
            Self::Luma(c) => c,
            Self::Oklab(c) => Luma::from_color(c),
            Self::Rgba(c) => Luma::from_color(c),
            Self::LinearRgb(c) => Luma::from_color(c),
            Self::Cmyk(c) => Luma::from_color(c.to_rgba()),
            Self::Hsl(c) => Luma::from_color(c),
            Self::Hsv(c) => Luma::from_color(c),
        })
    }

    pub fn to_oklab(self) -> Self {
        Self::Oklab(match self {
            Self::Luma(c) => Oklab::from_color(c),
            Self::Oklab(c) => c,
            Self::Rgba(c) => Oklab::from_color(c),
            Self::LinearRgb(c) => Oklab::from_color(c),
            Self::Cmyk(c) => Oklab::from_color(c.to_rgba()),
            Self::Hsl(c) => Oklab::from_color(c),
            Self::Hsv(c) => Oklab::from_color(c),
        })
    }

    pub fn to_linear_rgb(self) -> Self {
        Self::LinearRgb(match self {
            Self::Luma(c) => LinearRgba::from_color(c),
            Self::Oklab(c) => LinearRgba::from_color(c),
            Self::Rgba(c) => LinearRgba::from_color(c),
            Self::LinearRgb(c) => c,
            Self::Cmyk(c) => LinearRgba::from_color(c.to_rgba()),
            Self::Hsl(c) => LinearRgba::from_color(Rgba::from_color(c)),
            Self::Hsv(c) => LinearRgba::from_color(Rgba::from_color(c)),
        })
    }

    pub fn to_rgba(self) -> Self {
        Self::Rgba(match self {
            Self::Luma(c) => Rgba::from_color(c),
            Self::Oklab(c) => Rgba::from_color(c),
            Self::Rgba(c) => c,
            Self::LinearRgb(c) => Rgba::from_linear(c),
            Self::Cmyk(c) => c.to_rgba(),
            Self::Hsl(c) => Rgba::from_color(c),
            Self::Hsv(c) => Rgba::from_color(c),
        })
    }

    pub fn to_cmyk(self) -> Self {
        Self::Cmyk(match self {
            Self::Luma(c) => Cmyk::from_luma(c),
            Self::Oklab(c) => Cmyk::from_rgba(Rgba::from_color(c)),
            Self::Rgba(c) => Cmyk::from_rgba(c),
            Self::LinearRgb(c) => Cmyk::from_rgba(Rgba::from_linear(c)),
            Self::Cmyk(c) => c,
            Self::Hsl(c) => Cmyk::from_rgba(Rgba::from_color(c)),
            Self::Hsv(c) => Cmyk::from_rgba(Rgba::from_color(c)),
        })
    }

    pub fn to_hsl(self) -> Self {
        Self::Hsl(match self {
            Self::Luma(c) => Hsl::from_color(c),
            Self::Oklab(c) => Hsl::from_color(c),
            Self::Rgba(c) => Hsl::from_color(c),
            Self::LinearRgb(c) => Hsl::from_color(Rgba::from_linear(c)),
            Self::Cmyk(c) => Hsl::from_color(c.to_rgba()),
            Self::Hsl(c) => c,
            Self::Hsv(c) => Hsl::from_color(c),
        })
    }

    pub fn to_hsv(self) -> Self {
        Self::Hsv(match self {
            Self::Luma(c) => Hsv::from_color(c),
            Self::Oklab(c) => Hsv::from_color(c),
            Self::Rgba(c) => Hsv::from_color(c),
            Self::LinearRgb(c) => Hsv::from_color(Rgba::from_linear(c)),
            Self::Cmyk(c) => Hsv::from_color(c.to_rgba()),
            Self::Hsl(c) => Hsv::from_color(c),
            Self::Hsv(c) => c,
        })
    }
}

impl Debug for Color {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        match self {
            Self::Luma(c) => write!(f, "luma({:?})", Ratio::new(c.luma as _)),
            Self::Rgba(_) => write!(f, "rgb({:?})", self.to_hex()),
            Self::LinearRgb(c) => {
                if c.alpha == 1.0 {
                    write!(
                        f,
                        "color.linear-rgb({:?}, {:?}, {:?})",
                        Ratio::new(c.red as _),
                        Ratio::new(c.green as _),
                        Ratio::new(c.blue as _),
                    )
                } else {
                    write!(
                        f,
                        "color.linear-rgb({:?}, {:?}, {:?}, {:?})",
                        Ratio::new(c.red as _),
                        Ratio::new(c.green as _),
                        Ratio::new(c.blue as _),
                        Ratio::new(c.alpha as _),
                    )
                }
            }
            Self::Cmyk(c) => {
                write!(
                    f,
                    "rgb({:?}, {:?}, {:?}, {:?})",
                    Ratio::new(c.c.get() as _),
                    Ratio::new(c.m.get() as _),
                    Ratio::new(c.y.get() as _),
                    Ratio::new(c.k.get() as _),
                )
            }
            Self::Oklab(c) => {
                if c.alpha == 1.0 {
                    write!(
                        f,
                        "oklab({:?}, {:.3}, {:.3})",
                        Ratio::new(c.l as _),
                        (c.a * 1000.0).round() / 1000.0,
                        (c.b * 1000.0).round() / 1000.0,
                    )
                } else {
                    write!(
                        f,
                        "oklab({:?}, {:?}, {:?}, {:?})",
                        Ratio::new(c.l as _),
                        (c.a * 1000.0).round() / 1000.0,
                        (c.b * 1000.0).round() / 1000.0,
                        Ratio::new(c.alpha as _),
                    )
                }
            }
            Self::Hsl(c) => {
                if c.alpha == 1.0 {
                    write!(
                        f,
                        "color.hsl({:?}, {:?}, {:?})",
                        Angle::deg(c.hue.into_degrees().rem_euclid(360.0) as _),
                        Ratio::new(c.saturation as _),
                        Ratio::new(c.lightness as _),
                    )
                } else {
                    write!(
                        f,
                        "color.hsl({:?}, {:?}, {:?}, {:?})",
                        Angle::deg(c.hue.into_degrees().rem_euclid(360.0) as _),
                        Ratio::new(c.saturation as _),
                        Ratio::new(c.lightness as _),
                        Ratio::new(c.alpha as _),
                    )
                }
            }
            Self::Hsv(c) => {
                if c.alpha == 1.0 {
                    write!(
                        f,
                        "color.hsv({:?}, {:?}, {:?})",
                        Angle::deg(c.hue.into_degrees().rem_euclid(360.0) as _),
                        Ratio::new(c.saturation as _),
                        Ratio::new(c.value as _),
                    )
                } else {
                    write!(
                        f,
                        "color.hsv({:?}, {:?}, {:?}, {:?})",
                        Angle::deg(c.hue.into_degrees().rem_euclid(360.0) as _),
                        Ratio::new(c.saturation as _),
                        Ratio::new(c.value as _),
                        Ratio::new(c.alpha as _),
                    )
                }
            }
        }
    }
}

impl FromStr for Color {
    type Err = &'static str;

    /// Constructs a new color from hex strings like the following:
    /// - `#aef` (shorthand, with leading hashtag),
    /// - `7a03c2` (without alpha),
    /// - `abcdefff` (with alpha).
    ///
    /// The hashtag is optional and both lower and upper case are fine.
    fn from_str(hex_str: &str) -> Result<Self, Self::Err> {
        let hex_str = hex_str.strip_prefix('#').unwrap_or(hex_str);
        if hex_str.chars().any(|c| !c.is_ascii_hexdigit()) {
            return Err("color string contains non-hexadecimal letters");
        }

        let len = hex_str.len();
        let long = len == 6 || len == 8;
        let short = len == 3 || len == 4;
        let alpha = len == 4 || len == 8;
        if !long && !short {
            return Err("color string has wrong length");
        }

        let mut values: [u8; 4] = [u8::MAX; 4];
        for elem in if alpha { 0..4 } else { 0..3 } {
            let item_len = if long { 2 } else { 1 };
            let pos = elem * item_len;

            let item = &hex_str[pos..(pos + item_len)];
            values[elem] = u8::from_str_radix(item, 16).unwrap();

            if short {
                // Duplicate number for shorthand notation, i.e. `a` -> `aa`
                values[elem] += values[elem] * 16;
            }
        }

        Ok(Self::from_u8(values[0], values[1], values[2], values[3]))
    }
}

/// A color with a weight.
pub struct WeightedColor(Color, f64);

impl WeightedColor {
    pub const fn new(color: Color, weight: f64) -> Self {
        Self(color, weight)
    }

    pub fn color(&self) -> Color {
        self.0
    }

    pub fn weight(&self) -> f64 {
        self.1
    }
}

cast! {
    WeightedColor,
    self => array![self.0, Value::Float(self.1 as _)].into_value(),
    v: Color => Self(v, 1.0),
    v: Array => {
        let mut iter = v.into_iter();
        match (iter.next(), iter.next(), iter.next()) {
            (Some(c), Some(w), None) => Self(c.cast()?, w.cast::<Weight>()?.0),
            _ => bail!("expected a color or color-weight pair"),
        }
    }
}

/// A weight for color mixing.
struct Weight(f64);

cast! {
    Weight,
    v: f64 => Self(v),
    v: Ratio => Self(v.get()),
}

/// A color space for mixing.
#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
pub enum ColorSpace {
    /// A perceptual color space.
    Oklab,

    /// The standard RGB color space.
    Srgb,

    /// The D65-gray color space.
    D65Gray,

    /// The linear RGB color space.
    LinearRgb,

    /// The HSL color space.
    Hsl,

    /// The HSV color space.
    Hsv,

    /// The CMYK color space.
    Cmyk,
}

cast! {
    ColorSpace,
    self => match self {
        Self::Oklab => Color::oklab_data(),
        Self::Srgb => Color::rgb_data(),
        Self::D65Gray => Color::luma_data(),
        Self::LinearRgb => Color::linear_rgb_data(),
        Self::Hsl => Color::hsl_data(),
        Self::Hsv => Color::hsv_data(),
        Self::Cmyk => Color::cmyk_data(),
    }.into_value(),
    v: Value => {
        let Value::Func(func) = v else {
            bail!(
                "expected `rgb`, `luma`, `cmyk`, `oklab`, `color.linear-rgb`, `color.hsl`, or `color.hsv`, found {}",
                v.ty()
            );
        };

        // Here comparing the function pointer since it's `Eq`
        // whereas the `NativeFuncData` is not.
        if func == Color::oklab_data() {
            Self::Oklab
        } else if func == Color::rgb_data() {
            Self::Srgb
        } else if func == Color::luma_data() {
            Self::D65Gray
        } else if func == Color::linear_rgb_data() {
            Self::LinearRgb
        } else if func == Color::hsl_data() {
            Self::Hsl
        } else if func == Color::hsv_data() {
            Self::Hsv
        } else if func == Color::cmyk_data() {
            Self::Cmyk
        } else {
            bail!(
                "expected `rgb`, `luma`, `cmyk`, `oklab`, `color.linear-rgb`, `color.hsl`, or `color.hsv`"
            )
        }
    },
}

/// A component that must be a ratio.
pub struct RatioComponent(Ratio);

cast! {
    RatioComponent,
    self => self.0.into_value(),
    v: Ratio => if (0.0 ..= 1.0).contains(&v.get()) {
        Self(v)
    } else {
        bail!("ratio must be between 0% and 100%");
    },
}

/// A component that must be a ratio between -40% and 40%.
pub struct ABComponent(Ratio);

cast! {
    ABComponent,
    v: Ratio => if (-0.4 ..= 0.4).contains(&v.get()) {
        Self(v)
    } else {
        bail!("ratio must be between -40% and 40%");
    },
    v: f64 => if (-0.4 ..= 0.4).contains(&v) {
        Self(Ratio::new(v))
    } else {
        bail!("ratio must be between -0.4 and 0.4");
    },
}

/// An integer or ratio component.
pub struct Component(Ratio);

cast! {
    Component,
    self => self.0.into_value(),
    v: i64 => match v {
        0 ..= 255 => Self(Ratio::new(v as f64 / 255.0)),
        _ => bail!("number must be between 0 and 255"),
    },
    v: Ratio => if (0.0 ..= 1.0).contains(&v.get()) {
        Self(v)
    } else {
        bail!("ratio must be between 0% and 100%");
    },
}
/// An 8-bit CMYK color.
#[derive(Copy, Clone, PartialEq, Eq, Hash)]
pub struct Cmyk {
    /// The cyan component.
    pub c: F32Scalar,
    /// The magenta component.
    pub m: F32Scalar,
    /// The yellow component.
    pub y: F32Scalar,
    /// The key (black) component.
    pub k: F32Scalar,
}

impl Cmyk {
    fn new(c: f32, m: f32, y: f32, k: f32) -> Self {
        Self { c: c.into(), m: m.into(), y: y.into(), k: k.into() }
    }

    fn from_luma(luma: Luma) -> Self {
        let l = luma.luma;
        Cmyk::new(l * 0.75, l * 0.68, l * 0.67, l * 0.90)
    }

    fn from_rgba(rgba: Rgba) -> Self {
        let r = rgba.red;
        let g = rgba.green;
        let b = rgba.blue;

        let k = 1.0 - r.max(g).max(b);
        if k == 1.0 {
            return Cmyk::new(0.0, 0.0, 0.0, 1.0);
        }

        let c = (1.0 - r - k) / (1.0 - k);
        let m = (1.0 - g - k) / (1.0 - k);
        let y = (1.0 - b - k) / (1.0 - k);

        Cmyk::new(c, m, y, k)
    }

    fn to_rgba(self) -> Rgba {
        let r = (1.0 - self.c.get()) * (1.0 - self.k.get());
        let g = (1.0 - self.m.get()) * (1.0 - self.k.get());
        let b = (1.0 - self.y.get()) * (1.0 - self.k.get());

        Rgba::new(r, g, b, 1.0)
    }

    fn lighten(self, factor: f32) -> Self {
        let lighten = |u: F32Scalar| (u.get() - u.get() * factor).clamp(0.0, 1.0);
        Self::new(lighten(self.c), lighten(self.m), lighten(self.y), lighten(self.k))
    }

    fn darken(self, factor: f32) -> Self {
        let darken = |u: F32Scalar| (u.get() + (1.0 - u.get()) * factor).clamp(0.0, 1.0);
        Self::new(darken(self.c), darken(self.m), darken(self.y), darken(self.k))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_color_strings() {
        #[track_caller]
        fn test(hex: &str, r: u8, g: u8, b: u8, a: u8) {
            assert_eq!(Color::from_str(hex), Ok(Color::from_u8(r, g, b, a)));
        }

        test("f61243ff", 0xf6, 0x12, 0x43, 255);
        test("b3d8b3", 0xb3, 0xd8, 0xb3, 255);
        test("fCd2a9AD", 0xfc, 0xd2, 0xa9, 0xad);
        test("233", 0x22, 0x33, 0x33, 255);
        test("111b", 0x11, 0x11, 0x11, 0xbb);
    }

    #[test]
    fn test_parse_invalid_colors() {
        #[track_caller]
        fn test(hex: &str, message: &str) {
            assert_eq!(Color::from_str(hex), Err(message));
        }

        test("a5", "color string has wrong length");
        test("12345", "color string has wrong length");
        test("f075ff011", "color string has wrong length");
        test("hmmm", "color string contains non-hexadecimal letters");
        test("14B2AH", "color string contains non-hexadecimal letters");
    }
}
