use std::fmt::Display;
use std::str::FromStr;

use ecow::{eco_format, EcoString};
use palette::encoding::{self, Linear};
use palette::{Darken, Desaturate, FromColor, Lighten, RgbHue, Saturate, ShiftHue};

use super::*;
use crate::diag::{bail, At, SourceResult};
use crate::eval::{cast, Args, Array, Func, Str};
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
#[derive(Copy, Clone, Eq, PartialEq, Hash)]
#[ty(scope)]
pub enum Color {
    /// A 64-bit luma color.
    Luma(LumaColor),
    /// A 64-bit L*a*b* color in the Oklab color space.
    Oklab(OklabColor),
    /// A 64-bit linear RGB color.
    LinearRgb(LinearRgbColor),
    /// A 64-bit RGBA color.
    Rgba(RgbaColor),
    /// A 64-bit CMYK color.
    Cmyk(CmykColor),
    /// A 64-bit HSL color.
    Hsl(HslColor),
    /// A 64-bit HSV color.
    Hsv(HsvColor),
}

impl Colorful for Color {
    fn components(self) -> [f32; 4] {
        match self {
            Self::Luma(color) => color.components(),
            Self::Oklab(color) => color.components(),
            Self::LinearRgb(color) => color.components(),
            Self::Rgba(color) => color.components(),
            Self::Cmyk(color) => color.components(),
            Self::Hsl(color) => color.components(),
            Self::Hsv(color) => color.components(),
        }
    }

    fn from_vec4(_vec: [f32; 4]) -> Self {
        unimplemented!("cannot build a generic color from a vector")
    }

    fn to_rgba(self) -> RgbaColor {
        match self {
            Self::Luma(color) => color.to_rgba(),
            Self::Oklab(color) => color.to_rgba(),
            Self::LinearRgb(color) => color.to_rgba(),
            Self::Rgba(color) => color,
            Self::Cmyk(color) => color.to_rgba(),
            Self::Hsl(color) => color.to_rgba(),
            Self::Hsv(color) => color.to_rgba(),
        }
    }

    fn to_oklab(self) -> OklabColor {
        match self {
            Self::Luma(color) => color.to_oklab(),
            Self::Oklab(color) => color,
            Self::LinearRgb(color) => color.to_oklab(),
            Self::Rgba(color) => color.to_oklab(),
            Self::Cmyk(color) => color.to_oklab(),
            Self::Hsl(color) => color.to_oklab(),
            Self::Hsv(color) => color.to_oklab(),
        }
    }

    fn to_linear_rgb(self) -> LinearRgbColor {
        match self {
            Self::Luma(color) => color.to_linear_rgb(),
            Self::Oklab(color) => color.to_linear_rgb(),
            Self::LinearRgb(color) => color,
            Self::Rgba(color) => color.to_linear_rgb(),
            Self::Cmyk(color) => color.to_linear_rgb(),
            Self::Hsl(color) => color.to_linear_rgb(),
            Self::Hsv(color) => color.to_linear_rgb(),
        }
    }

    fn to_hsl(self) -> HslColor {
        match self {
            Self::Luma(color) => color.to_hsl(),
            Self::Oklab(color) => color.to_hsl(),
            Self::LinearRgb(color) => color.to_hsl(),
            Self::Rgba(color) => color.to_hsl(),
            Self::Cmyk(color) => color.to_hsl(),
            Self::Hsl(color) => color,
            Self::Hsv(color) => color.to_hsl(),
        }
    }

    fn to_hsv(self) -> HsvColor {
        match self {
            Self::Luma(color) => color.to_hsv(),
            Self::Oklab(color) => color.to_hsv(),
            Self::LinearRgb(color) => color.to_hsv(),
            Self::Rgba(color) => color.to_hsv(),
            Self::Cmyk(color) => color.to_hsv(),
            Self::Hsl(color) => color.to_hsv(),
            Self::Hsv(color) => color,
        }
    }

    fn to_cmyk(self) -> CmykColor {
        match self {
            Self::Luma(color) => color.to_cmyk(),
            Self::Oklab(color) => color.to_cmyk(),
            Self::LinearRgb(color) => color.to_cmyk(),
            Self::Rgba(color) => color.to_cmyk(),
            Self::Cmyk(color) => color,
            Self::Hsl(color) => color.to_cmyk(),
            Self::Hsv(color) => color.to_cmyk(),
        }
    }

    fn to_luma(self) -> LumaColor {
        match self {
            Self::Luma(color) => color,
            Self::Oklab(color) => color.to_luma(),
            Self::LinearRgb(color) => color.to_luma(),
            Self::Rgba(color) => color.to_luma(),
            Self::Cmyk(color) => color.to_luma(),
            Self::Hsl(color) => color.to_luma(),
            Self::Hsv(color) => color.to_luma(),
        }
    }

    fn lighten(self, factor: Ratio) -> Self {
        match self {
            Self::Luma(color) => Self::Luma(color.lighten(factor)),
            Self::Oklab(color) => Self::Oklab(color.lighten(factor)),
            Self::LinearRgb(color) => Self::LinearRgb(color.lighten(factor)),
            Self::Rgba(color) => Self::Rgba(color.lighten(factor)),
            Self::Cmyk(color) => Self::Cmyk(color.lighten(factor)),
            Self::Hsl(color) => Self::Hsl(color.lighten(factor)),
            Self::Hsv(color) => Self::Hsv(color.lighten(factor)),
        }
    }

    fn darken(self, factor: Ratio) -> Self {
        match self {
            Self::Luma(color) => Self::Luma(color.darken(factor)),
            Self::Oklab(color) => Self::Oklab(color.darken(factor)),
            Self::LinearRgb(color) => Self::LinearRgb(color.darken(factor)),
            Self::Rgba(color) => Self::Rgba(color.darken(factor)),
            Self::Cmyk(color) => Self::Cmyk(color.darken(factor)),
            Self::Hsl(color) => Self::Hsl(color.darken(factor)),
            Self::Hsv(color) => Self::Hsv(color.darken(factor)),
        }
    }

    fn saturate(self, factor: Ratio) -> Self {
        match self {
            Self::Luma(color) => Self::Luma(color.saturate(factor)),
            Self::Oklab(color) => Self::Oklab(color.saturate(factor)),
            Self::LinearRgb(color) => Self::LinearRgb(color.saturate(factor)),
            Self::Rgba(color) => Self::Rgba(color.saturate(factor)),
            Self::Cmyk(color) => Self::Cmyk(color.saturate(factor)),
            Self::Hsl(color) => Self::Hsl(color.saturate(factor)),
            Self::Hsv(color) => Self::Hsv(color.saturate(factor)),
        }
    }

    fn desaturate(self, factor: Ratio) -> Self {
        match self {
            Self::Luma(color) => Self::Luma(color.desaturate(factor)),
            Self::Oklab(color) => Self::Oklab(color.desaturate(factor)),
            Self::LinearRgb(color) => Self::LinearRgb(color.desaturate(factor)),
            Self::Rgba(color) => Self::Rgba(color.desaturate(factor)),
            Self::Cmyk(color) => Self::Cmyk(color.desaturate(factor)),
            Self::Hsl(color) => Self::Hsl(color.desaturate(factor)),
            Self::Hsv(color) => Self::Hsv(color.desaturate(factor)),
        }
    }

    fn hue_rotate(self, hue: Angle) -> Self {
        match self {
            Self::Luma(color) => Self::Luma(color.hue_rotate(hue)),
            Self::Oklab(color) => Self::Oklab(color.hue_rotate(hue)),
            Self::LinearRgb(color) => Self::LinearRgb(color.hue_rotate(hue)),
            Self::Rgba(color) => Self::Rgba(color.hue_rotate(hue)),
            Self::Cmyk(color) => Self::Cmyk(color.hue_rotate(hue)),
            Self::Hsl(color) => Self::Hsl(color.hue_rotate(hue)),
            Self::Hsv(color) => Self::Hsv(color.hue_rotate(hue)),
        }
    }

    fn negate(self) -> Self {
        match self {
            Self::Luma(color) => Self::Luma(color.negate()),
            Self::Oklab(color) => Self::Oklab(color.negate()),
            Self::LinearRgb(color) => Self::LinearRgb(color.negate()),
            Self::Rgba(color) => Self::Rgba(color.negate()),
            Self::Cmyk(color) => Self::Cmyk(color.negate()),
            Self::Hsl(color) => Self::Hsl(color.negate()),
            Self::Hsv(color) => Self::Hsv(color.negate()),
        }
    }

    fn alpha(self) -> Option<f32> {
        match self {
            Self::Luma(color) => color.alpha(),
            Self::Oklab(color) => color.alpha(),
            Self::LinearRgb(color) => color.alpha(),
            Self::Rgba(color) => color.alpha(),
            Self::Cmyk(color) => color.alpha(),
            Self::Hsl(color) => color.alpha(),
            Self::Hsv(color) => color.alpha(),
        }
    }

    fn with_alpha(self, alpha: f32) -> Self {
        match self {
            Self::Luma(color) => Self::Luma(color.with_alpha(alpha)),
            Self::Oklab(color) => Self::Oklab(color.with_alpha(alpha)),
            Self::LinearRgb(color) => Self::LinearRgb(color.with_alpha(alpha)),
            Self::Rgba(color) => Self::Rgba(color.with_alpha(alpha)),
            Self::Cmyk(color) => Self::Cmyk(color.with_alpha(alpha)),
            Self::Hsl(color) => Self::Hsl(color.with_alpha(alpha)),
            Self::Hsv(color) => Self::Hsv(color.with_alpha(alpha)),
        }
    }

    fn to_array(self, alpha: bool) -> Array {
        match self {
            Color::Luma(color) => color.to_array(alpha),
            Color::Oklab(color) => color.to_array(alpha),
            Color::LinearRgb(color) => color.to_array(alpha),
            Color::Rgba(color) => color.to_array(alpha),
            Color::Cmyk(color) => color.to_array(alpha),
            Color::Hsl(color) => color.to_array(alpha),
            Color::Hsv(color) => color.to_array(alpha),
        }
    }
}

#[scope]
impl Color {
    pub const BLACK: Self = Self::Luma(LumaColor(F32Scalar(0.0)));
    pub const GRAY: Self = Self::Luma(LumaColor(F32Scalar(0.6666666)));
    pub const WHITE: Self = Self::Luma(LumaColor(F32Scalar(1.0)));
    pub const SILVER: Self = Self::Luma(LumaColor(F32Scalar(0.8666667)));
    pub const NAVY: Self = Self::Rgba(RgbaColor::new(0.0, 0.121569, 0.247059, 1.0));
    pub const BLUE: Self = Self::Rgba(RgbaColor::new(0.0, 0.454902, 0.85098, 1.0));
    pub const AQUA: Self = Self::Rgba(RgbaColor::new(0.4980392, 0.858823, 1.0, 1.0));
    pub const TEAL: Self = Self::Rgba(RgbaColor::new(0.223529, 0.8, 0.8, 1.0));
    pub const EASTERN: Self =
        Self::Rgba(RgbaColor::new(0.13725, 0.615686, 0.678431, 1.0));
    pub const PURPLE: Self =
        Self::Rgba(RgbaColor::new(0.694118, 0.050980, 0.788235, 1.0));
    pub const FUCHSIA: Self =
        Self::Rgba(RgbaColor::new(0.941177, 0.070588, 0.745098, 1.0));
    pub const MAROON: Self =
        Self::Rgba(RgbaColor::new(0.521569, 0.078431, 0.294118, 1.0));
    pub const RED: Self = Self::Rgba(RgbaColor::new(1.0, 0.254902, 0.211765, 1.0));
    pub const ORANGE: Self = Self::Rgba(RgbaColor::new(1.0, 0.521569, 0.105882, 1.0));
    pub const YELLOW: Self = Self::Rgba(RgbaColor::new(1.0, 0.8627451, 0.0, 1.0));
    pub const OLIVE: Self = Self::Rgba(RgbaColor::new(0.239216, 0.6, 0.4392157, 1.0));
    pub const GREEN: Self = Self::Rgba(RgbaColor::new(0.1803922, 0.8, 0.2509804, 1.0));
    pub const LIME: Self = Self::Rgba(RgbaColor::new(0.0039216, 1.0, 0.4392157, 1.0));

    /// Create a grayscale color.
    ///
    /// A grayscale color is represented internally by an array of one components:
    /// - lightness ([`ratio`]($ratio))
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
        /// The gray component.
        #[external]
        gray: Component,
        /// The color to convert to grayscale.
        #[external]
        color: Color,
    ) -> SourceResult<Color> {
        let mut args = args;
        Ok(if let Some(color) = args.find::<Color>()? {
            color.to_luma().into()
        } else {
            let Component(gray) =
                args.expect("gray component").unwrap_or(Component(Ratio::one()));
            LumaColor(gray.get().into()).into()
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
            color.to_oklab().into()
        } else {
            let RatioComponent(l) = args.expect("lightness component")?;
            let ABComponent(a) = args.expect("A component")?;
            let ABComponent(b) = args.expect("B component")?;
            let RatioComponent(alpha) =
                args.eat()?.unwrap_or(RatioComponent(Ratio::one()));
            OklabColor::new(
                l.get() as f32,
                a.get() as f32,
                b.get() as f32,
                alpha.get() as f32,
            )
            .into()
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
            color.to_linear_rgb().into()
        } else {
            let Component(r) = args.expect("red component")?;
            let Component(g) = args.expect("green component")?;
            let Component(b) = args.expect("blue component")?;
            let Component(a) = args.eat()?.unwrap_or(Component(Ratio::one()));
            LinearRgbColor::new(
                r.get() as f32,
                g.get() as f32,
                b.get() as f32,
                a.get() as f32,
            )
            .into()
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
            RgbaColor::from_str(&string.v).at(string.span)?.into()
        } else if let Some(color) = args.find::<Color>()? {
            color.to_rgba().into()
        } else {
            let Component(r) = args.expect("red component")?;
            let Component(g) = args.expect("green component")?;
            let Component(b) = args.expect("blue component")?;
            let Component(a) = args.eat()?.unwrap_or(Component(Ratio::one()));
            RgbaColor::new(r.get() as f32, g.get() as f32, b.get() as f32, a.get() as f32)
                .into()
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
            color.to_cmyk().into()
        } else {
            let RatioComponent(c) = args.expect("cyan component")?;
            let RatioComponent(m) = args.expect("magenta component")?;
            let RatioComponent(y) = args.expect("yellow component")?;
            let RatioComponent(k) = args.expect("key/black component")?;
            CmykColor::new(c.get() as f32, m.get() as f32, y.get() as f32, k.get() as f32)
                .into()
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
            color.to_hsl().into()
        } else {
            let h: Angle = args.expect("hue component")?;
            let Component(s) = args.expect("saturation component")?;
            let Component(l) = args.expect("lightness component")?;
            let Component(a) = args.eat()?.unwrap_or(Component(Ratio::one()));
            HslColor::new(
                h.to_deg() as f32,
                s.get() as f32,
                l.get() as f32,
                a.get() as f32,
            )
            .into()
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
            color.to_hsv().into()
        } else {
            let h: Angle = args.expect("hue component")?;
            let Component(s) = args.expect("saturation component")?;
            let Component(v) = args.expect("value component")?;
            let Component(a) = args.eat()?.unwrap_or(Component(Ratio::one()));
            HsvColor::new(
                h.to_deg() as f32,
                s.get() as f32,
                v.get() as f32,
                a.get() as f32,
            )
            .into()
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
        <Self as Colorful>::to_array(self, alpha)
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
        <Self as Colorful>::to_hex(self)
    }

    /// Lightens a color by a given factor.
    #[func]
    pub fn lighten(
        self,
        /// The factor to lighten the color by.
        factor: Ratio,
    ) -> Color {
        <Self as Colorful>::lighten(self, factor)
    }

    /// Darkens a color by a given factor.
    #[func]
    pub fn darken(
        self,
        /// The factor to darken the color by.
        factor: Ratio,
    ) -> Color {
        <Self as Colorful>::darken(self, factor)
    }

    /// Increases the saturation of a color by a given factor.
    #[func]
    pub fn saturate(
        self,
        /// The factor to saturate the color by.
        factor: Ratio,
    ) -> Color {
        <Self as Colorful>::saturate(self, factor)
    }

    /// Decreases the saturation of a color by a given factor.
    #[func]
    pub fn desaturate(
        self,
        /// The factor to desaturate the color by.
        factor: Ratio,
    ) -> Color {
        <Self as Colorful>::desaturate(self, factor)
    }

    /// Produces the negative of the color.
    #[func]
    pub fn negate(self) -> Color {
        <Self as Colorful>::negate(self)
    }

    /// Rotates the hue of the color by a given angle.
    #[func]
    pub fn rotate(self, angle: Angle) -> Color {
        <Self as Colorful>::hue_rotate(self, angle)
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
            let v = Colorful::components(color.to_space(space));
            acc[0] += weight * v[0];
            acc[1] += weight * v[1];
            acc[2] += weight * v[2];
            acc[3] += weight * v[3];
            total += weight;
        }

        if total <= 0.0 {
            bail!("sum of weights must be positive");
        }

        let mixed = acc.map(|v| v / total);
        Ok(match space {
            ColorSpace::Oklab => Color::Oklab(OklabColor::from_vec4(mixed)),
            ColorSpace::Srgb => Color::Rgba(RgbaColor::from_vec4(mixed)),
            ColorSpace::LinearRgb => Color::LinearRgb(LinearRgbColor::from_vec4(mixed)),
            ColorSpace::Hsl => Color::Hsl(HslColor::from_vec4(mixed)),
            ColorSpace::Hsv => Color::Hsv(HsvColor::from_vec4(mixed)),
            ColorSpace::Cmyk => Color::Cmyk(CmykColor::from_vec4(mixed)),
            ColorSpace::D65Gray => Color::Luma(LumaColor::from_vec4(mixed)),
        })
    }
}

impl Debug for Color {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        match self {
            Self::Luma(c) => Debug::fmt(c, f),
            Self::Rgba(c) => Debug::fmt(c, f),
            Self::Cmyk(c) => Debug::fmt(c, f),
            Self::Oklab(c) => Debug::fmt(c, f),
            Self::LinearRgb(c) => Debug::fmt(c, f),
            Self::Hsl(c) => Debug::fmt(c, f),
            Self::Hsv(c) => Debug::fmt(c, f),
        }
    }
}

/// A trait containing all common functionality of colors.
pub trait Colorful: Sized + Copy + Into<Color> {
    /// Convert the color to a four-component vector.
    fn components(self) -> [f32; 4];

    /// Convert a four-component vector to a color.
    fn from_vec4(vec: [f32; 4]) -> Self;

    /// Convert the color to an RGBA color.
    fn to_rgba(self) -> RgbaColor;

    /// Convert the color to an Oklab color.
    fn to_oklab(self) -> OklabColor;

    /// Convert the color to an RGBA color with linear RGB components.
    fn to_linear_rgb(self) -> LinearRgbColor;

    /// Convert the color to an HSL color.
    fn to_hsl(self) -> HslColor;

    /// Convert the color to an HSV color.
    fn to_hsv(self) -> HsvColor;

    /// Convert the color to a CMYK color.
    fn to_cmyk(self) -> CmykColor;

    /// Convert the color to a grayscale color.
    fn to_luma(self) -> LumaColor;

    /// Lighten the color by the given factor.
    fn lighten(self, factor: Ratio) -> Self;

    /// Darken the color by the given factor.
    fn darken(self, factor: Ratio) -> Self;

    /// Saturate the color by the given factor.
    fn saturate(self, factor: Ratio) -> Self;

    /// Desaturate the color by the given factor.
    fn desaturate(self, factor: Ratio) -> Self;

    /// Rotate the hue of the color by the given angle.
    fn hue_rotate(self, hue: Angle) -> Self;

    /// Negate the color.
    fn negate(self) -> Self;

    /// Get the alpha component of the color, if any.
    fn alpha(self) -> Option<f32>;

    /// Convert the color into a hexadecimal RGB(A) string.
    fn to_hex(self) -> EcoString {
        self.to_rgba().to_hex()
    }

    /// Override the alpha component of the color.
    fn with_alpha(self, alpha: f32) -> Self {
        let [x, y, z, _] = self.components();

        if self.alpha().is_some() {
            Self::from_vec4([x, y, z, alpha])
        } else {
            self
        }
    }

    /// Convert the color into a typst array.
    fn to_array(self, alpha: bool) -> Array;

    /// Converts the color into the give color space.
    fn to_space(self, color_space: ColorSpace) -> Color {
        match color_space {
            ColorSpace::Oklab => self.to_oklab().into(),
            ColorSpace::Srgb => self.to_rgba().into(),
            ColorSpace::D65Gray => self.to_luma().into(),
            ColorSpace::LinearRgb => self.to_linear_rgb().into(),
            ColorSpace::Cmyk => self.to_cmyk().into(),
            ColorSpace::Hsl => self.to_hsl().into(),
            ColorSpace::Hsv => self.to_hsv().into(),
        }
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
        if !matches!(v, Value::Func(_)) {
            bail!(
                "expected `rgb`, `luma`, `cmyk`, `oklab`, `color.linear-rgb`, `color.hsl`, or `color.hsv`, found {}",
                v.ty()
            );
        }

        let func = v
            .cast::<Func>()
            .unwrap()
            .native()
            .ok_or(
                "expected `rgb`, `luma`, `cmyk`, `oklab`, `color.linear-rgb`, `color.hsl`, or `color.hsv`"
            )?
            .function;

        // Here comparing the function pointer since it's `Eq`
        // whereas the `NativeFuncData` is not.
        if func == Color::oklab_data().function {
            Self::Oklab
        } else if func == Color::rgb_data().function {
            Self::Srgb
        } else if func == Color::luma_data().function {
            Self::D65Gray
        } else if func == Color::linear_rgb_data().function {
            Self::LinearRgb
        } else if func == Color::hsl_data().function {
            Self::Hsl
        } else if func == Color::hsv_data().function {
            Self::Hsv
        } else if func == Color::cmyk_data().function {
            Self::Cmyk
        } else {
            bail!(
                "expected `rgb`, `luma`, `cmyk`, `oklab`, `color.linear-rgb`, `color.hsl`, or `color.hsv`"
            )
        }
    },
}

/// A grayscale color.
#[derive(Copy, Clone, Eq, PartialEq, Hash)]
pub struct LumaColor(pub F32Scalar);

impl Debug for LumaColor {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        write!(f, "luma({:?})", Ratio::new(self.0.get() as f64))
    }
}

impl From<LumaColor> for Color {
    fn from(luma: LumaColor) -> Self {
        Self::Luma(luma)
    }
}

impl From<Luma> for LumaColor {
    fn from(luma: Luma) -> Self {
        Self::new(luma.luma)
    }
}

impl From<LumaColor> for Luma {
    fn from(luma: LumaColor) -> Self {
        Self::new(luma.0.into())
    }
}

impl LumaColor {
    pub fn new(l: f32) -> Self {
        Self(l.into())
    }
}

impl Colorful for LumaColor {
    fn components(self) -> [f32; 4] {
        [self.0.get(); 4]
    }

    fn from_vec4(vec: [f32; 4]) -> Self {
        Self::new(vec[0])
    }

    fn to_array(self, _: bool) -> Array {
        array![Ratio::new(self.0.get() as f64)]
    }

    fn alpha(self) -> Option<f32> {
        None
    }

    fn to_rgba(self) -> RgbaColor {
        Rgba::from_color(Luma::from(self)).into()
    }

    fn to_oklab(self) -> OklabColor {
        Oklab::from_color(Luma::from(self)).into()
    }

    fn to_linear_rgb(self) -> LinearRgbColor {
        LinearRgba::from_color(Luma::from(self)).into()
    }

    fn to_hsl(self) -> HslColor {
        Hsl::from_color(Luma::from(self)).into()
    }

    fn to_hsv(self) -> HsvColor {
        Hsv::from_color(Luma::from(self)).into()
    }

    fn to_cmyk(self) -> CmykColor {
        let l = self.0.get();
        CmykColor::new(l * 0.75, l * 0.68, l * 0.67, l * 0.90)
    }

    fn to_luma(self) -> LumaColor {
        self
    }

    fn lighten(self, factor: Ratio) -> Self {
        let l = self.0.get();
        let inc = (1.0 - l) * factor.get() as f32;
        Self::new(l.add(inc).clamp(0.0, 1.0))
    }

    fn darken(self, factor: Ratio) -> Self {
        let l = self.0.get();
        let inc = l * factor.get() as f32;
        Self::new(l.sub(inc).clamp(0.0, 1.0))
    }

    fn saturate(self, factor: Ratio) -> Self {
        self.to_hsv().saturate(factor).to_luma()
    }

    fn desaturate(self, factor: Ratio) -> Self {
        self.to_hsv().desaturate(factor).to_luma()
    }

    fn hue_rotate(self, hue: Angle) -> Self {
        self.to_hsv().hue_rotate(hue).to_luma()
    }

    fn negate(self) -> Self {
        Self::new(1.0 - self.0.get())
    }
}

#[derive(Copy, Clone, PartialEq, Eq, Hash)]
pub struct OklabColor {
    /// The lightness of the color.
    /// In the range [0, 1].
    pub l: F32Scalar,

    /// The `a` component of the color.
    /// In the range [-0.4, 0.4].
    pub a: F32Scalar,

    /// The `b` component of the color.
    /// In the range [-0.4, 0.4].
    pub b: F32Scalar,

    /// The alpha component of the color.
    /// In the range [0, 1].
    pub alpha: F32Scalar,
}

impl From<OklabColor> for Oklab {
    fn from(color: OklabColor) -> Self {
        Self::new(color.l.get(), color.a.get(), color.b.get(), color.alpha.get())
    }
}

impl From<Oklab> for OklabColor {
    fn from(color: Oklab) -> Self {
        Self::new(color.l, color.a, color.b, color.alpha)
    }
}

impl OklabColor {
    pub fn new(l: f32, a: f32, b: f32, alpha: f32) -> Self {
        Self {
            l: l.into(),
            a: a.into(),
            b: b.into(),
            alpha: alpha.into(),
        }
    }
}

impl Colorful for OklabColor {
    fn components(self) -> [f32; 4] {
        [self.l.get(), self.a.get(), self.b.get(), self.alpha.get()]
    }

    fn from_vec4(vec: [f32; 4]) -> Self {
        Self::new(vec[0], vec[1], vec[2], vec[3])
    }

    fn to_array(self, alpha: bool) -> Array {
        // Also perform some precision fix for f32 -> f64 conversion
        if alpha {
            array![
                Ratio::new(self.l.get() as f64),
                (self.a.get() as f64 * 1000.0).round() / 1000.0,
                (self.b.get() as f64 * 1000.0).round() / 1000.0,
                Ratio::new(self.alpha.get() as f64)
            ]
        } else {
            array![
                Ratio::new(self.l.get() as f64),
                (self.a.get() as f64 * 1000.0).round() / 1000.0,
                (self.b.get() as f64 * 1000.0).round() / 1000.0,
            ]
        }
    }

    fn alpha(self) -> Option<f32> {
        Some(self.alpha.get())
    }

    fn to_rgba(self) -> RgbaColor {
        Rgba::from_color(Oklab::from(self)).into()
    }

    fn to_oklab(self) -> OklabColor {
        self
    }

    fn to_linear_rgb(self) -> LinearRgbColor {
        LinearRgba::from_color(Oklab::from(self)).into()
    }

    fn to_hsl(self) -> HslColor {
        Hsl::from_color(Oklab::from(self)).into()
    }

    fn to_hsv(self) -> HsvColor {
        Hsv::from_color(Oklab::from(self)).into()
    }

    fn to_cmyk(self) -> CmykColor {
        self.to_rgba().to_cmyk()
    }

    fn to_luma(self) -> LumaColor {
        Luma::from_color(Oklab::from(self)).into()
    }

    fn lighten(self, factor: Ratio) -> Self {
        Oklab::from(self).lighten(factor.get() as f32).into()
    }

    fn darken(self, factor: Ratio) -> Self {
        Oklab::from(self).darken(factor.get() as f32).into()
    }

    fn saturate(self, factor: Ratio) -> Self {
        self.to_hsv().saturate(factor).to_oklab()
    }

    fn desaturate(self, factor: Ratio) -> Self {
        self.to_hsv().desaturate(factor).to_oklab()
    }

    fn hue_rotate(self, hue: Angle) -> Self {
        self.to_hsv().hue_rotate(hue).to_oklab()
    }

    fn negate(self) -> Self {
        self.to_rgba().negate().to_oklab()
    }
}

impl Debug for OklabColor {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        if self.alpha == 1.0 {
            write!(
                f,
                "oklab({:?}, {:.4}, {:.4})",
                Ratio::new(self.l.get() as f64),
                self.a,
                self.b
            )
        } else {
            write!(
                f,
                "oklab({:?}, {:.4}, {:.4}, {:?})",
                Ratio::new(self.l.get() as f64),
                self.a,
                self.b,
                Ratio::new(self.alpha.get() as f64)
            )
        }
    }
}

impl From<OklabColor> for Color {
    fn from(oklab: OklabColor) -> Self {
        Self::Oklab(oklab)
    }
}

#[derive(Copy, Clone, PartialEq, Eq, Hash)]
pub struct LinearRgbColor {
    /// The red component of the color.
    /// In the range [0, 1].
    pub r: F32Scalar,

    /// The green component of the color.
    /// In the range [0, 1].
    pub g: F32Scalar,

    /// The blue component of the color.
    /// In the range [0, 1].
    pub b: F32Scalar,

    /// The alpha component of the color.
    /// In the range [0, 1].
    pub a: F32Scalar,
}

impl From<LinearRgbColor> for LinearRgba {
    fn from(color: LinearRgbColor) -> Self {
        Self::new(color.r.get(), color.g.get(), color.b.get(), color.a.get())
    }
}

impl From<LinearRgba> for LinearRgbColor {
    fn from(color: LinearRgba) -> Self {
        Self::new(color.red, color.green, color.blue, color.alpha)
    }
}

impl Colorful for LinearRgbColor {
    fn components(self) -> [f32; 4] {
        [self.r.get(), self.g.get(), self.b.get(), self.a.get()]
    }

    fn from_vec4(vec: [f32; 4]) -> Self {
        Self::new(vec[0], vec[1], vec[2], vec[3])
    }

    fn alpha(self) -> Option<f32> {
        Some(self.a.get())
    }

    fn to_rgba(self) -> RgbaColor {
        Rgba::from_color(LinearRgba::from(self)).into()
    }

    fn to_oklab(self) -> OklabColor {
        Oklab::from_color(LinearRgba::from(self)).into()
    }

    fn to_linear_rgb(self) -> LinearRgbColor {
        self
    }

    fn to_hsl(self) -> HslColor {
        self.to_rgba().to_hsl()
    }

    fn to_hsv(self) -> HsvColor {
        self.to_rgba().to_hsv()
    }

    fn to_cmyk(self) -> CmykColor {
        self.to_rgba().to_cmyk()
    }

    fn to_luma(self) -> LumaColor {
        Luma::from_color(LinearRgba::from(self)).into()
    }

    fn lighten(self, factor: Ratio) -> Self {
        LinearRgba::from(self).lighten(factor.get() as f32).into()
    }

    fn darken(self, factor: Ratio) -> Self {
        LinearRgba::from(self).darken(factor.get() as f32).into()
    }

    fn saturate(self, factor: Ratio) -> Self {
        self.to_hsv().saturate(factor).to_linear_rgb()
    }

    fn desaturate(self, factor: Ratio) -> Self {
        self.to_hsv().desaturate(factor).to_linear_rgb()
    }

    fn hue_rotate(self, hue: Angle) -> Self {
        self.to_hsv().hue_rotate(hue).to_linear_rgb()
    }

    fn negate(self) -> Self {
        Self::new(
            1.0 - self.r.get(),
            1.0 - self.g.get(),
            1.0 - self.b.get(),
            self.a.get(),
        )
    }

    fn to_array(self, alpha: bool) -> Array {
        if alpha {
            array![
                Ratio::new(self.r.get() as f64),
                Ratio::new(self.g.get() as f64),
                Ratio::new(self.b.get() as f64),
                Ratio::new(self.a.get() as f64),
            ]
        } else {
            array![
                Ratio::new(self.r.get() as f64),
                Ratio::new(self.g.get() as f64),
                Ratio::new(self.b.get() as f64),
            ]
        }
    }
}

impl LinearRgbColor {
    pub fn new(r: f32, g: f32, b: f32, a: f32) -> Self {
        Self { r: r.into(), g: g.into(), b: b.into(), a: a.into() }
    }
}

impl Debug for LinearRgbColor {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        if self.a == 1.0 {
            write!(
                f,
                "linear-rgb({:?}, {:?}, {:?})",
                Ratio::new(self.r.get() as f64),
                Ratio::new(self.g.get() as f64),
                Ratio::new(self.b.get() as f64),
            )
        } else {
            write!(
                f,
                "linear-rgb({:?}, {:?}, {:?}, {:?})",
                Ratio::new(self.r.get() as f64),
                Ratio::new(self.g.get() as f64),
                Ratio::new(self.b.get() as f64),
                Ratio::new(self.a.get() as f64),
            )
        }
    }
}

impl From<LinearRgbColor> for Color {
    fn from(linear_srgb: LinearRgbColor) -> Self {
        Self::LinearRgb(linear_srgb)
    }
}

/// An 32-bit RGBA color.
#[derive(Copy, Clone, Eq)]
pub struct RgbaColor {
    /// Red channel.
    r: F32Scalar,
    /// Green channel.
    g: F32Scalar,
    /// Blue channel.
    b: F32Scalar,
    /// Alpha channel.
    a: F32Scalar,
}

impl Hash for RgbaColor {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.r.hash(state);
        self.g.hash(state);
        self.b.hash(state);
        self.a.hash(state);
    }
}

impl PartialEq for RgbaColor {
    fn eq(&self, other: &Self) -> bool {
        self.to_vec4_u8() == other.to_vec4_u8()
    }
}

impl From<RgbaColor> for Rgba {
    fn from(value: RgbaColor) -> Self {
        let [r, g, b, a] = value.components();
        Self::new(r, g, b, a)
    }
}

impl From<Rgba> for RgbaColor {
    fn from(rgba: Rgba) -> Self {
        Self::new(rgba.red, rgba.green, rgba.blue, rgba.alpha)
    }
}

impl Colorful for RgbaColor {
    fn components(self) -> [f32; 4] {
        [self.r.get(), self.g.get(), self.b.get(), self.a.get()]
    }

    fn from_vec4(vec: [f32; 4]) -> Self {
        Self::new(vec[0], vec[1], vec[2], vec[3])
    }

    fn to_array(self, alpha: bool) -> Array {
        let [r, g, b, a] = self.components();
        if alpha {
            array![
                Ratio::new(r as f64),
                Ratio::new(g as f64),
                Ratio::new(b as f64),
                Ratio::new(a as f64),
            ]
        } else {
            array![Ratio::new(r as f64), Ratio::new(g as f64), Ratio::new(b as f64),]
        }
    }

    fn alpha(self) -> Option<f32> {
        Some(self.a.get())
    }

    fn to_rgba(self) -> RgbaColor {
        self
    }

    fn to_oklab(self) -> OklabColor {
        Oklab::from_color(Rgba::from(self)).into()
    }

    fn to_linear_rgb(self) -> LinearRgbColor {
        LinearRgba::from_color(Rgba::from(self)).into()
    }

    fn to_hsl(self) -> HslColor {
        Hsl::from_color(Rgba::from(self)).into()
    }

    fn to_hsv(self) -> HsvColor {
        Hsv::from_color(Rgba::from(self)).into()
    }

    fn to_cmyk(self) -> CmykColor {
        let [r, g, b, _] = self.components();

        let k = 1.0 - r.max(g).max(b);
        if k == 1.0 {
            return CmykColor::new(0.0, 0.0, 0.0, 1.0);
        }

        let c = (1.0 - r - k) / (1.0 - k);
        let m = (1.0 - g - k) / (1.0 - k);
        let y = (1.0 - b - k) / (1.0 - k);

        CmykColor::new(c, m, y, k)
    }

    fn to_luma(self) -> LumaColor {
        Luma::from_color(Rgba::from(self)).into()
    }

    fn lighten(self, factor: Ratio) -> Self {
        Rgba::from(self).lighten(factor.get() as f32).into()
    }

    fn darken(self, factor: Ratio) -> Self {
        Rgba::from(self).darken(factor.get() as f32).into()
    }

    fn saturate(self, factor: Ratio) -> Self {
        self.to_hsv().saturate(factor).to_rgba()
    }

    fn desaturate(self, factor: Ratio) -> Self {
        self.to_hsv().desaturate(factor).to_rgba()
    }

    fn hue_rotate(self, hue: Angle) -> Self {
        self.to_hsv().hue_rotate(hue).to_rgba()
    }

    fn negate(self) -> Self {
        Self::new(
            1.0 - self.r.get(),
            1.0 - self.g.get(),
            1.0 - self.b.get(),
            self.a.get(),
        )
    }

    fn to_hex(self) -> EcoString {
        let [r, g, b, a] = self.to_vec4_u8();
        if a != 255 {
            eco_format!("#{:02x}{:02x}{:02x}{:02x}", r, g, b, a)
        } else {
            eco_format!("#{:02x}{:02x}{:02x}", r, g, b)
        }
    }
}

impl RgbaColor {
    /// Construct a new RGBA color.
    pub const fn new(r: f32, g: f32, b: f32, a: f32) -> Self {
        Self {
            r: F32Scalar(r),
            g: F32Scalar(g),
            b: F32Scalar(b),
            a: F32Scalar(a),
        }
    }

    /// Construct a new RGBA color from 8-bit values.
    pub fn from_u8(r: u8, g: u8, b: u8, a: u8) -> Self {
        Self::new(r as f32 / 255.0, g as f32 / 255.0, b as f32 / 255.0, a as f32 / 255.0)
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

    /// Converts the color into four 8-bit values.
    #[inline]
    pub fn to_vec4_u8(self) -> [u8; 4] {
        let f = |r: F32Scalar| (r.get() * 255.0).round() as u8;
        [f(self.r), f(self.g), f(self.b), f(self.a)]
    }
}

impl FromStr for RgbaColor {
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

impl Debug for RgbaColor {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        let [r, g, b, a] = self.components();
        if f.alternate() {
            write!(
                f,
                "rgba({:?}, {:?}, {:?}, {:?})",
                Ratio::new(r as f64),
                Ratio::new(g as f64),
                Ratio::new(b as f64),
                Ratio::new(a as f64),
            )?;
        } else {
            write!(f, "rgb(\"{}\")", self.to_hex())?;
        }
        Ok(())
    }
}

impl<T: Into<RgbaColor>> From<T> for Color {
    fn from(rgba: T) -> Self {
        Self::Rgba(rgba.into())
    }
}

cast! {
    RgbaColor,
    self => Value::Color(self.into()),
}

/// An 8-bit CMYK color.
#[derive(Copy, Clone, PartialEq, Eq, Hash)]
pub struct CmykColor {
    /// The cyan component.
    pub c: F32Scalar,
    /// The magenta component.
    pub m: F32Scalar,
    /// The yellow component.
    pub y: F32Scalar,
    /// The key (black) component.
    pub k: F32Scalar,
}

impl Colorful for CmykColor {
    fn components(self) -> [f32; 4] {
        [self.c.get(), self.m.get(), self.y.get(), self.k.get()]
    }

    fn from_vec4(vec: [f32; 4]) -> Self {
        Self::new(vec[0], vec[1], vec[2], vec[3])
    }

    fn to_array(self, _: bool) -> Array {
        let [c, m, y, k] = self.components();
        array![
            Ratio::new(c as f64),
            Ratio::new(m as f64),
            Ratio::new(y as f64),
            Ratio::new(k as f64),
        ]
    }

    fn alpha(self) -> Option<f32> {
        None
    }

    fn to_rgba(self) -> RgbaColor {
        let r = (1.0 - self.c.get()) * (1.0 - self.k.get());
        let g = (1.0 - self.m.get()) * (1.0 - self.k.get());
        let b = (1.0 - self.y.get()) * (1.0 - self.k.get());

        RgbaColor::new(r, g, b, 1.0)
    }

    fn to_oklab(self) -> OklabColor {
        self.to_rgba().to_oklab()
    }

    fn to_linear_rgb(self) -> LinearRgbColor {
        self.to_rgba().to_linear_rgb()
    }

    fn to_hsl(self) -> HslColor {
        self.to_rgba().to_hsl()
    }

    fn to_hsv(self) -> HsvColor {
        self.to_rgba().to_hsv()
    }

    fn to_cmyk(self) -> CmykColor {
        self
    }

    fn to_luma(self) -> LumaColor {
        self.to_rgba().to_luma()
    }

    fn lighten(self, factor: Ratio) -> Self {
        let lighten =
            |u: F32Scalar| (u.get() - u.get() * factor.get() as f32).clamp(0.0, 1.0);
        Self::new(lighten(self.c), lighten(self.m), lighten(self.y), lighten(self.k))
    }

    fn darken(self, factor: Ratio) -> Self {
        let darken = |u: F32Scalar| {
            (u.get() + (1.0 - u.get()) * factor.get() as f32).clamp(0.0, 1.0)
        };
        Self::new(darken(self.c), darken(self.m), darken(self.y), darken(self.k))
    }

    fn saturate(self, factor: Ratio) -> Self {
        self.to_hsv().saturate(factor).to_cmyk()
    }

    fn desaturate(self, factor: Ratio) -> Self {
        self.to_hsv().desaturate(factor).to_cmyk()
    }

    fn hue_rotate(self, hue: Angle) -> Self {
        self.to_hsv().hue_rotate(hue).to_cmyk()
    }

    fn negate(self) -> Self {
        Self::new(
            1.0 - self.c.get(),
            1.0 - self.m.get(),
            1.0 - self.y.get(),
            self.k.get(),
        )
    }
}

impl CmykColor {
    /// Construct a new CMYK color.
    pub fn new(c: f32, m: f32, y: f32, k: f32) -> Self {
        Self { c: c.into(), m: m.into(), y: y.into(), k: k.into() }
    }
}

impl Debug for CmykColor {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        write!(
            f,
            "cmyk({:?}, {:?}, {:?}, {:?})",
            Ratio::new(self.c.get() as f64),
            Ratio::new(self.m.get() as f64),
            Ratio::new(self.y.get() as f64),
            Ratio::new(self.k.get() as f64),
        )
    }
}

impl From<CmykColor> for Color {
    fn from(cmyk: CmykColor) -> Self {
        Self::Cmyk(cmyk)
    }
}

cast! {
    CmykColor,
    self => Value::Color(self.into()),
}

/// A 32-bit HslA color.
#[derive(Copy, Clone, PartialEq, Eq, Hash)]
pub struct HslColor {
    /// The hue angle in degrees.
    pub h: F32Scalar,
    pub s: F32Scalar,
    pub l: F32Scalar,
    pub a: F32Scalar,
}

impl From<HslColor> for Color {
    fn from(value: HslColor) -> Self {
        Self::Hsl(value)
    }
}

impl Debug for HslColor {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        if self.a == 1.0 {
            write!(
                f,
                "hsl({:?}, {:?}, {:?})",
                Angle::deg(self.h.get() as f64),
                Ratio::new(self.s.get() as f64),
                Ratio::new(self.l.get() as f64),
            )
        } else {
            write!(
                f,
                "hsl({:?}, {:?}, {:?}, {:?})",
                Angle::deg(self.h.get() as f64),
                Ratio::new(self.s.get() as f64),
                Ratio::new(self.l.get() as f64),
                Ratio::new(self.a.get() as f64),
            )
        }
    }
}

impl From<HslColor> for Hsl {
    fn from(hsl: HslColor) -> Self {
        Self::new(RgbHue::new(hsl.h.get()), hsl.s.get(), hsl.l.get(), hsl.a.get())
    }
}

impl From<Hsl> for HslColor {
    fn from(hsl: Hsl) -> Self {
        Self::new(hsl.hue.into_degrees(), hsl.saturation, hsl.lightness, hsl.alpha)
    }
}

impl HslColor {
    pub fn new(h: f32, s: f32, l: f32, a: f32) -> Self {
        Self { h: h.into(), s: s.into(), l: l.into(), a: a.into() }
    }
}

impl Colorful for HslColor {
    fn components(self) -> [f32; 4] {
        [self.h.get().rem_euclid(360.0), self.s.get(), self.l.get(), self.a.get()]
    }

    fn from_vec4(vec: [f32; 4]) -> Self {
        Self::new(vec[0], vec[1], vec[2], vec[3])
    }

    fn to_array(self, alpha: bool) -> Array {
        if alpha {
            array![
                Angle::deg(self.h.get() as f64),
                Ratio::new(self.s.get() as f64),
                Ratio::new(self.l.get() as f64),
                Ratio::new(self.a.get() as f64),
            ]
        } else {
            array![
                Angle::deg(self.h.get() as f64),
                Ratio::new(self.s.get() as f64),
                Ratio::new(self.l.get() as f64),
            ]
        }
    }

    fn to_rgba(self) -> RgbaColor {
        Rgba::from_color(Hsl::from(self)).into()
    }

    fn to_oklab(self) -> OklabColor {
        Oklab::from_color(Hsl::from(self)).into()
    }

    fn to_linear_rgb(self) -> LinearRgbColor {
        self.to_rgba().to_linear_rgb()
    }

    fn to_hsl(self) -> HslColor {
        self
    }

    fn to_hsv(self) -> HsvColor {
        Hsv::from_color(Hsl::from(self)).into()
    }

    fn to_cmyk(self) -> CmykColor {
        self.to_rgba().to_cmyk()
    }

    fn to_luma(self) -> LumaColor {
        Luma::from_color(Hsl::from(self)).into()
    }

    fn lighten(self, factor: Ratio) -> Self {
        Hsl::from(self).lighten(factor.get() as f32).into()
    }

    fn darken(self, factor: Ratio) -> Self {
        Hsl::from(self).darken(factor.get() as f32).into()
    }

    fn saturate(self, factor: Ratio) -> Self {
        Hsl::from(self).saturate(factor.get() as f32).into()
    }

    fn desaturate(self, factor: Ratio) -> Self {
        Hsl::from(self).desaturate(factor.get() as f32).into()
    }

    fn hue_rotate(self, hue: Angle) -> Self {
        Hsl::from(self).shift_hue(hue.to_deg() as f32).into()
    }

    fn negate(self) -> Self {
        Self::new(360.0 - self.h.get(), self.s.get(), self.l.get(), self.a.get())
    }

    fn alpha(self) -> Option<f32> {
        Some(self.a.get())
    }
}

/// A 32-bit HslA color.
#[derive(Copy, Clone, PartialEq, Eq, Hash)]
pub struct HsvColor {
    pub h: F32Scalar,
    pub s: F32Scalar,
    pub v: F32Scalar,
    pub a: F32Scalar,
}

impl From<HsvColor> for Color {
    fn from(value: HsvColor) -> Self {
        Color::Hsv(value)
    }
}

impl Debug for HsvColor {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        if self.a == 1.0 {
            write!(
                f,
                "hsv({:?}, {:?}, {:?})",
                Angle::deg(self.h.get() as f64),
                Ratio::new(self.s.get() as f64),
                Ratio::new(self.v.get() as f64),
            )
        } else {
            write!(
                f,
                "hsv({:?}, {:?}, {:?}, {:?})",
                Angle::deg(self.h.get() as f64),
                Ratio::new(self.s.get() as f64),
                Ratio::new(self.v.get() as f64),
                Ratio::new(self.a.get() as f64),
            )
        }
    }
}

impl From<HsvColor> for Hsv {
    fn from(hsv: HsvColor) -> Self {
        Self::new(hsv.h.get(), hsv.s.get(), hsv.v.get(), hsv.a.get())
    }
}

impl From<Hsv> for HsvColor {
    fn from(hsl: Hsv) -> Self {
        Self::new(hsl.hue.into_degrees(), hsl.saturation, hsl.value, hsl.alpha)
    }
}

impl HsvColor {
    pub fn new(h: f32, s: f32, v: f32, a: f32) -> Self {
        Self { h: h.into(), s: s.into(), v: v.into(), a: a.into() }
    }
}

impl Colorful for HsvColor {
    fn components(self) -> [f32; 4] {
        [self.h.get().rem_euclid(360.0), self.s.get(), self.v.get(), self.a.get()]
    }

    fn from_vec4(vec: [f32; 4]) -> Self {
        Self::new(vec[0], vec[1], vec[2], vec[3])
    }

    fn to_array(self, alpha: bool) -> Array {
        if alpha {
            array![
                Angle::deg(self.h.get() as f64),
                Ratio::new(self.s.get() as f64),
                Ratio::new(self.v.get() as f64),
                Ratio::new(self.a.get() as f64),
            ]
        } else {
            array![
                Angle::deg(self.h.get() as f64),
                Ratio::new(self.s.get() as f64),
                Ratio::new(self.v.get() as f64),
            ]
        }
    }

    fn to_rgba(self) -> RgbaColor {
        Rgba::from_color(Hsv::from(self)).into()
    }

    fn to_oklab(self) -> OklabColor {
        Oklab::from_color(Hsv::from(self)).into()
    }

    fn to_linear_rgb(self) -> LinearRgbColor {
        self.to_rgba().to_linear_rgb()
    }

    fn to_hsl(self) -> HslColor {
        Hsl::from_color(Hsv::from(self)).into()
    }

    fn to_hsv(self) -> HsvColor {
        self
    }

    fn to_cmyk(self) -> CmykColor {
        self.to_rgba().to_cmyk()
    }

    fn to_luma(self) -> LumaColor {
        Luma::from_color(Hsv::from(self)).into()
    }

    fn lighten(self, factor: Ratio) -> Self {
        Hsv::from(self).lighten(factor.get() as f32).into()
    }

    fn darken(self, factor: Ratio) -> Self {
        Hsv::from(self).darken(factor.get() as f32).into()
    }

    fn saturate(self, factor: Ratio) -> Self {
        Hsv::from(self).saturate(factor.get() as f32).into()
    }

    fn desaturate(self, factor: Ratio) -> Self {
        Hsv::from(self).desaturate(factor.get() as f32).into()
    }

    fn hue_rotate(self, hue: Angle) -> Self {
        Hsv::from(self).shift_hue(hue.to_deg() as f32).into()
    }

    fn negate(self) -> Self {
        Self::new(
            360.0 - self.h.get(),
            1.0 - self.s.get(),
            1.0 - self.v.get(),
            self.a.get(),
        )
    }

    fn alpha(self) -> Option<f32> {
        Some(self.a.get())
    }
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_color_strings() {
        #[track_caller]
        fn test(hex: &str, r: u8, g: u8, b: u8, a: u8) {
            assert_eq!(RgbaColor::from_str(hex), Ok(RgbaColor::from_u8(r, g, b, a)));
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
            assert_eq!(RgbaColor::from_str(hex), Err(message));
        }

        test("a5", "color string has wrong length");
        test("12345", "color string has wrong length");
        test("f075ff011", "color string has wrong length");
        test("hmmm", "color string contains non-hexadecimal letters");
        test("14B2AH", "color string contains non-hexadecimal letters");
    }
}

/// A 32-bit float that implements `Eq`, `Ord` and `Hash`.
///
/// Panics if it's `NaN` during any of those operations.
#[derive(Default, Copy, Clone)]
pub struct F32Scalar(f32);

impl F32Scalar {
    /// Get the underlying float.
    #[inline]
    pub fn get(self) -> f32 {
        self.0
    }
}

impl From<f32> for F32Scalar {
    fn from(float: f32) -> Self {
        Self(float)
    }
}

impl From<f64> for F32Scalar {
    fn from(float: f64) -> Self {
        Self(float as f32)
    }
}

impl From<F32Scalar> for f32 {
    fn from(scalar: F32Scalar) -> Self {
        scalar.0
    }
}

impl Debug for F32Scalar {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        Debug::fmt(&self.0, f)
    }
}

impl Display for F32Scalar {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        Display::fmt(&self.0, f)
    }
}

impl Eq for F32Scalar {}

impl PartialEq for F32Scalar {
    fn eq(&self, other: &Self) -> bool {
        assert!(!self.0.is_nan() && !other.0.is_nan(), "float is NaN");
        self.0 == other.0
    }
}

impl PartialEq<f32> for F32Scalar {
    fn eq(&self, other: &f32) -> bool {
        self == &Self(*other)
    }
}

impl Hash for F32Scalar {
    fn hash<H: Hasher>(&self, state: &mut H) {
        debug_assert!(!self.0.is_nan(), "float is NaN");
        self.0.to_bits().hash(state);
    }
}
