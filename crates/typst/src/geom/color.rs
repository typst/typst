use ecow::{eco_format, EcoString};
use palette::{Darken, Desaturate, FromColor, Lighten, Saturate, ShiftHue};
use std::str::FromStr;
use typst_syntax::Spanned;

use super::*;
use crate::diag::{bail, At, SourceResult};
use crate::eval::{cast, Args, Array, Cast, IntoValue, Str, Func};

/// The precision by which colors are represented.
const PRECISION: f32 = 1000.0;

/// A trait containing all common functionality of colors.
pub trait ColorExt: Sized + Copy + Into<Color> {
    /// The number of components in the color.
    const COMPONENTS: usize;

    /// Convert the color to a four-component vector.
    fn to_vec4(self) -> [f32; 4];

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
        let [x, y, z, _] = self.to_vec4();

        if self.alpha().is_some() {
            Self::from_vec4([x, y, z, alpha])
        } else {
            self
        }
    }

    /// Convert the color into a typst array.
    fn to_array(self) -> Array {
        let vec = self
            .to_vec4()
            .into_iter()
            .map(|x| x as f64)
            .map(IntoValue::into_value)
            .take(Self::COMPONENTS);

        if self.alpha() == Some(1.0) {
            Array::from_iter(vec.take(Self::COMPONENTS - 1))
        } else {
            Array::from_iter(vec)
        }
    }
}

/// A color in a specific color space.
///
/// Typst supports:
/// - sRGB through the [`rgb` function]($rgb)
/// - Device CMYK through [`cmyk` function]($cmyk)
/// - D65 Gray through the [`luma` function]($luma)
/// - Oklab through the [`oklab` function]($oklab)
/// - Linear RGB through the [`linear-rgb` function]($linear-rgb)
/// - HSL through the [`hsl` function]($hsl)
/// - HSV through the [`hsv` function]($hsv)
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
    /// An 8-bit luma color.
    Luma(LumaColor),

    /// A 32-bit L*a*b* color in the Oklab color space.
    Oklab(OklabColor),

    /// A 32-bit linear RGB color.
    LinearRgb(LinearRgbColor),

    /// An 8-bit RGBA color.
    Rgba(RgbaColor),

    /// An 8-bit CMYK color.
    Cmyk(CmykColor),

    /// A 32-bit HSL color.
    Hsl(HslColor),

    /// A 32-bit HSV color.
    Hsv(HsvColor),
}

impl ColorExt for Color {
    const COMPONENTS: usize = 4;

    fn to_vec4(self) -> [f32; 4] {
        match self {
            Self::Luma(color) => color.to_vec4(),
            Self::Oklab(color) => color.to_vec4(),
            Self::LinearRgb(color) => color.to_vec4(),
            Self::Rgba(color) => color.to_vec4(),
            Self::Cmyk(color) => color.to_vec4(),
            Self::Hsl(color) => color.to_vec4(),
            Self::Hsv(color) => color.to_vec4(),
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
}

#[scope]
impl Color {
    pub const BLACK: Self = Self::Luma(LumaColor(Ratio::zero()));
    pub const GRAY: Self = Self::Luma(LumaColor(Ratio::new(0.6666666)));
    pub const SILVER: Self = Self::Rgba(RgbaColor::new_from_u8(0xDD, 0xDD, 0xDD, 0xFF));
    pub const WHITE: Self = Self::Luma(LumaColor(Ratio::one()));
    pub const NAVY: Self = Self::Rgba(RgbaColor::new_from_u8(0x00, 0x1f, 0x3f, 0xFF));
    pub const BLUE: Self = Self::Rgba(RgbaColor::new_from_u8(0x00, 0x74, 0xD9, 0xFF));
    pub const AQUA: Self = Self::Rgba(RgbaColor::new_from_u8(0x7F, 0xDB, 0xFF, 0xFF));
    pub const TEAL: Self = Self::Rgba(RgbaColor::new_from_u8(0x39, 0xCC, 0xCC, 0xFF));
    pub const EASTERN: Self = Self::Rgba(RgbaColor::new_from_u8(0x23, 0x9D, 0xAD, 0xFF));
    pub const PURPLE: Self = Self::Rgba(RgbaColor::new_from_u8(0xB1, 0x0D, 0xC9, 0xFF));
    pub const FUCHSIA: Self = Self::Rgba(RgbaColor::new_from_u8(0xF0, 0x12, 0xBE, 0xFF));
    pub const MAROON: Self = Self::Rgba(RgbaColor::new_from_u8(0x85, 0x14, 0x4b, 0xFF));
    pub const RED: Self = Self::Rgba(RgbaColor::new_from_u8(0xFF, 0x41, 0x36, 0xFF));
    pub const ORANGE: Self = Self::Rgba(RgbaColor::new_from_u8(0xFF, 0x85, 0x1B, 0xFF));
    pub const YELLOW: Self = Self::Rgba(RgbaColor::new_from_u8(0xFF, 0xDC, 0x00, 0xFF));
    pub const OLIVE: Self = Self::Rgba(RgbaColor::new_from_u8(0x3D, 0x99, 0x70, 0xFF));
    pub const GREEN: Self = Self::Rgba(RgbaColor::new_from_u8(0x2E, 0xCC, 0x40, 0xFF));
    pub const LIME: Self = Self::Rgba(RgbaColor::new_from_u8(0x01, 0xFF, 0x70, 0xFF));

    /// Create a grayscale color.
    ///
    /// ```example
    /// #for x in range(250, step: 50) {
    ///   box(square(fill: luma(x)))
    /// }
    /// ```
    #[func]
    pub fn luma(
        /// The gray component.
        gray: Component,
    ) -> Color {
        LumaColor::new(gray.0.get() as f32).into()
    }

    /// Create an [Oklab](https://bottosson.github.io/posts/oklab/) color.
    ///
    /// This color space is well suited for the following use cases:
    /// - Color manipulation such as saturating while keeping perceived hue
    /// - Creating grayscale images with uniform perceived lightness
    /// - Creating smooth and uniform color transition and gradients
    ///
    /// ```example
    /// #square(
    ///   fill: oklab(27%, 20%, -3%, 50%)
    /// )
    /// ```
    #[func]
    pub fn oklab(
        /// The cyan component.
        lightness: RatioComponent,
        /// The magenta component.
        a: ABComponent,
        /// The yellow component.
        b: ABComponent,
        /// The key component.
        alpha: RatioComponent,
    ) -> Color {
        OklabColor::new(
            lightness.0.get() as f32,
            a.0.get() as f32,
            b.0.get() as f32,
            alpha.0.get() as f32,
        )
        .into()
    }

    /// Create an RGB(A) color with linear luma.
    ///
    /// This color space is similar to sRGB, but with the distinction that
    /// the color component are not gamma corrected. This makes it easier to
    /// perform color operations such as blending and interpolation. Although,
    /// you should prefer to use the [`oklab` function]($oklab) for these.
    ///
    /// ```example
    /// #square(
    ///   fill: linear-rgb(30%, 50%, 10%)
    /// )
    /// ```
    #[func(title = "Linear RGB")]
    pub fn linear_rgb(
        /// The red component.
        red: Component,
        /// The green component.
        green: Component,
        /// The blue component.
        blue: Component,
        /// The alpha component.
        #[default(Component(Ratio::one()))]
        alpha: Component,
    ) -> Color {
        LinearRgbColor::new(
            red.0.get() as f32,
            green.0.get() as f32,
            blue.0.get() as f32,
            alpha.0.get() as f32,
        )
        .into()
    }

    /// Create an RGB(A) color.
    ///
    /// The color is specified in the sRGB color space.
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
    ) -> SourceResult<Color> {
        let mut args = args;
        Ok(if let Some(string) = args.find::<Spanned<Str>>()? {
            RgbaColor::from_str(&string.v).at(string.span)?.into()
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
    /// ```example
    /// #square(
    ///   fill: cmyk(27%, 0%, 3%, 5%)
    /// )
    /// ```
    #[func(title = "CMYK")]
    pub fn cmyk(
        /// The cyan component.
        cyan: RatioComponent,
        /// The magenta component.
        magenta: RatioComponent,
        /// The yellow component.
        yellow: RatioComponent,
        /// The key component.
        key: RatioComponent,
    ) -> Color {
        CmykColor::new(
            cyan.0.get() as f32,
            magenta.0.get() as f32,
            yellow.0.get() as f32,
            key.0.get() as f32,
        )
        .into()
    }

    /// Create an HSL color.
    ///
    /// This color space is useful for specifying colors by hue, saturation and
    /// lightness. It is also useful for color manipulation, such as saturating
    /// while keeping perceived hue.
    ///
    /// ```example
    /// #square(
    ///   fill: hsl(30deg, 50%, 60%)
    /// )
    /// ```
    #[func(title = "HSL")]
    pub fn hsl(
        /// The hue angle.
        hue: Angle,
        /// The saturation component.
        saturation: Component,
        /// The lightness component.
        lightness: Component,
        /// The alpha component.
        #[default(Component(Ratio::one()))]
        alpha: Component,
    ) -> Color {
        HslColor::new(
            hue,
            saturation.0.get() as f32,
            lightness.0.get() as f32,
            alpha.0.get() as f32,
        )
        .into()
    }

    /// Create an HSV color.
    ///
    /// This color space is useful for specifying colors by hue, saturation and
    /// value. It is also useful for color manipulation, such as saturating
    /// while keeping perceived hue.
    ///
    /// ```example
    /// #square(
    ///   fill: hsv(30deg, 50%, 60%)
    /// )
    /// ```
    #[func(title = "HSV")]
    pub fn hsv(
        /// The hue angle.
        hue: Angle,
        /// The saturation component.
        saturation: Component,
        /// The value component.
        value: Component,
        /// The alpha component.
        #[default(Component(Ratio::one()))]
        alpha: Component,
    ) -> Color {
        HsvColor::new(
            hue,
            saturation.0.get() as f32,
            value.0.get() as f32,
            alpha.0.get() as f32,
        )
        .into()
    }

    /// Gets the equivalent D65 Gray component of the color.
    /// 
    /// *Note*: color conversions can be lossy, this means that transforming a color
    /// to a different color space and back to the original color space may not
    /// yield the same color.
    #[func]
    pub fn as_luma(self) -> Ratio {
        <Self as ColorExt>::to_luma(self).0
    }

    /// Converts this color in a D65 Gray color.
    /// 
    /// *Note*: color conversions can be lossy, this means that transforming a color
    /// to a different color space and back to the original color space may not
    /// yield the same color.
    #[func]
    pub fn to_luma(self) -> Color {
        <Self as ColorExt>::to_luma(self).into()
    }

    /// Gets the equivalent Oklab color components of the color.
    /// 
    /// *Note*: color conversions can be lossy, this means that transforming a color
    /// to a different color space and back to the original color space may not
    /// yield the same color.
    #[func]
    pub fn as_oklab(self) -> Array {
        <Self as ColorExt>::to_oklab(self).to_array()
    }

    /// Converts this color in an Oklab color.
    /// 
    /// *Note*: color conversions can be lossy, this means that transforming a color
    /// to a different color space and back to the original color space may not
    /// yield the same color.
    #[func]
    pub fn to_oklab(self) -> Color {
        <Self as ColorExt>::to_oklab(self).into()
    }

    /// Gets the equivalent linear RGB color components of the color.
    /// 
    /// *Note*: color conversions can be lossy, this means that transforming a color
    /// to a different color space and back to the original color space may not
    /// yield the same color.
    #[func]
    pub fn as_linear_rgb(self) -> Array {
        <Self as ColorExt>::to_linear_rgb(self).to_array()
    }

    /// Converts this color in a linear RGB color.
    /// 
    /// *Note*: color conversions can be lossy, this means that transforming a color
    /// to a different color space and back to the original color space may not
    /// yield the same color.
    #[func]
    pub fn to_linear_rgb(self) -> Color {
        <Self as ColorExt>::to_linear_rgb(self).into()
    }

    /// Gets the equivalent RGBA color components of the color.
    /// 
    /// *Note*: color conversions can be lossy, this means that transforming a color
    /// to a different color space and back to the original color space may not
    /// yield the same color.
    #[func]
    pub fn as_rgba(self) -> Array {
        <Self as ColorExt>::to_rgba(self).to_array()
    }

    /// Converts this color in an RGBA color.
    /// 
    /// *Note*: color conversions can be lossy, this means that transforming a color
    /// to a different color space and back to the original color space may not
    /// yield the same color.
    #[func]
    pub fn to_rgba(self) -> Color {
        <Self as ColorExt>::to_rgba(self).into()
    }

    /// Gets the equivalent CMYK color components of the color.
    /// 
    /// *Note*: color conversions can be lossy, this means that transforming a color
    /// to a different color space and back to the original color space may not
    /// yield the same color.
    #[func]
    pub fn as_cmyk(self) -> Array {
        <Self as ColorExt>::to_cmyk(self).to_array()
    }

    /// Converts this color in a CMYK color.
    /// 
    /// *Note*: color conversions can be lossy, this means that transforming a color
    /// to a different color space and back to the original color space may not
    /// yield the same color.
    #[func]
    pub fn to_cmyk(self) -> Color {
        <Self as ColorExt>::to_cmyk(self).into()
    }

    /// Gets the equivalent HSL color components of the color.
    /// 
    /// *Note*: color conversions can be lossy, this means that transforming a color
    /// to a different color space and back to the original color space may not
    /// yield the same color.
    #[func]
    pub fn as_hsl(self) -> Array {
        <Self as ColorExt>::to_hsl(self).to_array()
    }

    /// Converts this color in an HSL color.
    /// 
    /// *Note*: color conversions can be lossy, this means that transforming a color
    /// to a different color space and back to the original color space may not
    /// yield the same color.
    #[func]
    pub fn to_hsl(self) -> Color {
        <Self as ColorExt>::to_hsl(self).into()
    }

    /// Gets the equivalent HSV color components of the color.
    /// 
    /// *Note*: color conversions can be lossy, this means that transforming a color
    /// to a different color space and back to the original color space may not
    /// yield the same color.
    #[func]
    pub fn as_hsv(self) -> Array {
        <Self as ColorExt>::to_hsv(self).to_array()
    }

    /// Returns the constructor function for this color's kind
    /// ([`rgb`]($color.rgb), [`cmyk`]($color.cmyk) or [`luma`]($color.luma)).
    ///
    /// ```example
    /// #let color = cmyk(1%, 2%, 3%, 4%)
    /// #(color.kind() == cmyk)
    /// ```
    #[func]
    pub fn kind(self) -> Func {
        match self {
            Self::Oklab(_) => Self::oklab_data().into(),
            Self::Luma(_) => Self::luma_data().into(),
            Self::Rgba(_) => Self::rgb_data().into(),
            Self::LinearRgb(_) => Self::linear_rgb_data().into(),
            Self::Cmyk(_) => Self::cmyk_data().into(),
            Self::Hsl(_) => Self::hsl_data().into(),
            Self::Hsv(_) => Self::hsv_data().into(),
        }
    }

    /// Returns the color's RGB(A) hex representation (such as `#ffaa32` or
    /// `#020304fe`). The alpha component (last two digits in `#020304fe`) is
    /// omitted if it is equal to `ff` (255 / 100%).
    #[func]
    pub fn to_hex(self) -> EcoString {
        <Self as ColorExt>::to_hex(self)
    }

    /// Lightens a color by a given factor.
    #[func]
    pub fn lighten(
        self,
        /// The factor to lighten the color by.
        factor: Ratio,
    ) -> Color {
        <Self as ColorExt>::lighten(self, factor)
    }

    /// Darkens a color by a given factor.
    #[func]
    pub fn darken(
        self,
        /// The factor to darken the color by.
        factor: Ratio,
    ) -> Color {
        <Self as ColorExt>::darken(self, factor)
    }

    /// Produces the negative of the color.
    #[func]
    pub fn negate(self) -> Color {
        <Self as ColorExt>::negate(self)
    }

    /// Create a color by mixing two or more colors.
    ///
    /// ```example
    /// #set block(height: 20pt, width: 100%)
    /// #block(fill: red.mix(blue))
    /// #block(fill: red.mix(blue, space: "srgb"))
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
        /// color space (Oklab).
        #[named]
        #[default(ColorSpace::Oklab)]
        space: ColorSpace,
    ) -> StrResult<Color> {
        let mut total = 0.0;
        let mut acc = [0.0; 4];

        for WeightedColor(color, weight) in colors.into_iter() {
            let v = color_to_vec4(color, space);
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
        Ok(vec4_to_color(mixed, space))
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

/// A color with a weight.
pub struct WeightedColor(Color, f32);

impl WeightedColor {
    pub const fn new(color: Color, weight: f32) -> Self {
        Self(color, weight)
    }

    pub fn color(&self) -> Color {
        self.0
    }

    pub fn weight(&self) -> f32 {
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
struct Weight(f32);

cast! {
    Weight,
    v: f64 => Self(v as f32),
    v: Ratio => Self(v.get() as f32),
}

/// Convert an RGBA color to four components in the given color space.
pub fn color_to_vec4(color: Color, space: ColorSpace) -> [f32; 4] {
    match space {
        ColorSpace::Oklab => color.to_oklab().to_vec4(),
        ColorSpace::Srgb => color.to_rgba().to_vec4(),
        ColorSpace::LinearRGB => color.to_linear_rgb().to_vec4(),
        ColorSpace::Hsl => color.to_hsl().to_vec4(),
        ColorSpace::Hsv => color.to_hsv().to_vec4(),
        ColorSpace::Cmyk => color.to_cmyk().to_vec4(),
        ColorSpace::D65Gray => color.to_luma().to_vec4(),
    }
}

/// Convert four components in the given color space to RGBA.
fn vec4_to_color(vec: [f32; 4], space: ColorSpace) -> Color {
    match space {
        ColorSpace::Oklab => Color::Oklab(OklabColor::from_vec4(vec)),
        ColorSpace::Srgb => Color::Rgba(RgbaColor::from_vec4(vec)),
        ColorSpace::LinearRGB => Color::LinearRgb(LinearRgbColor::from_vec4(vec)),
        ColorSpace::Hsl => Color::Hsl(HslColor::from_vec4(vec)),
        ColorSpace::Hsv => Color::Hsv(HsvColor::from_vec4(vec)),
        ColorSpace::Cmyk => Color::Cmyk(CmykColor::from_vec4(vec)),
        ColorSpace::D65Gray => Color::Luma(LumaColor::from_vec4(vec)),
    }
}

/// A color space for mixing.
#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash, Cast)]
pub enum ColorSpace {
    /// A perceptual color space.
    Oklab,

    /// The standard RGB color space.
    Srgb,

    /// The D65-gray color space.
    D65Gray,

    /// The linear RGB color space.
    LinearRGB,

    /// The HSL color space.
    Hsl,

    /// The HSV color space.
    Hsv,

    /// The CMYK color space.
    Cmyk,
}

/// A grayscale color.
#[derive(Copy, Clone, Hash, PartialEq, Eq)]
pub struct LumaColor(pub Ratio);

impl Debug for LumaColor {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        write!(f, "luma({:?})", self.0)
    }
}

impl From<LumaColor> for Color {
    fn from(luma: LumaColor) -> Self {
        Self::Luma(luma)
    }
}

impl From<palette::luma::SrgbLuma> for LumaColor {
    fn from(luma: palette::luma::SrgbLuma) -> Self {
        Self::new(luma.luma)
    }
}

impl From<LumaColor> for palette::luma::SrgbLuma {
    fn from(luma: LumaColor) -> Self {
        Self::new(luma.0.get() as _)
    }
}

impl LumaColor {
    pub fn new(l: f32) -> Self {
        Self(Ratio::new((l as f64).fix_precision()))
    }
}

impl ColorExt for LumaColor {
    const COMPONENTS: usize = 1;

    fn to_vec4(self) -> [f32; 4] {
        [self.0.get() as f32; 4]
    }

    fn from_vec4(vec: [f32; 4]) -> Self {
        Self::new(vec[0])
    }

    fn to_array(self) -> Array {
        array![self.0]
    }

    fn alpha(self) -> Option<f32> {
        None
    }

    fn to_rgba(self) -> RgbaColor {
        palette::rgb::Rgba::from_color(palette::luma::SrgbLuma::from(self)).into()
    }

    fn to_oklab(self) -> OklabColor {
        palette::Oklaba::from_color(palette::luma::SrgbLuma::from(self)).into()
    }

    fn to_linear_rgb(self) -> LinearRgbColor {
        palette::LinSrgba::from_color(palette::luma::SrgbLuma::from(self)).into()
    }

    fn to_hsl(self) -> HslColor {
        palette::Hsla::from_color(palette::luma::SrgbLuma::from(self)).into()
    }

    fn to_hsv(self) -> HsvColor {
        palette::Hsva::from_color(palette::luma::SrgbLuma::from(self)).into()
    }

    fn to_cmyk(self) -> CmykColor {
        let l = self.0.get() as f32;
        CmykColor::new(l * 0.75, l * 0.68, l * 0.67, l * 0.90)
    }

    fn to_luma(self) -> LumaColor {
        self
    }

    fn lighten(self, factor: Ratio) -> Self {
        let l = self.0.get() as f32;
        let inc = (1.0 - l) * factor.get() as f32;
        Self::new(l.add(inc).clamp(0.0, 1.0))
    }

    fn darken(self, factor: Ratio) -> Self {
        let l = self.0.get() as f32;
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
        Self::new(1.0 - self.0.get() as f32)
    }
}

#[derive(Copy, Clone, PartialEq)]
pub struct OklabColor {
    /// The lightness of the color.
    /// In the range [0, 1].
    pub l: f32,

    /// The `a` component of the color.
    /// In the range [-0.4, 0.4].
    pub a: f32,

    /// The `b` component of the color.
    /// In the range [-0.4, 0.4].
    pub b: f32,

    /// The alpha component of the color.
    /// In the range [0, 1].
    pub alpha: f32,
}

impl From<OklabColor> for palette::oklab::Oklaba {
    fn from(color: OklabColor) -> Self {
        Self::new(color.l, color.a, color.b, color.alpha)
    }
}

impl From<palette::oklab::Oklaba> for OklabColor {
    fn from(color: palette::oklab::Oklaba) -> Self {
        Self::new(color.l, color.a, color.b, color.alpha)
    }
}

impl OklabColor {
    pub fn new(l: f32, a: f32, b: f32, alpha: f32) -> Self {
        Self {
            l: l.fix_precision(),
            a: a.fix_precision(),
            b: b.fix_precision(),
            alpha: alpha.fix_precision(),
        }
    }
}

impl ColorExt for OklabColor {
    const COMPONENTS: usize = 4;

    fn to_vec4(self) -> [f32; 4] {
        [self.l, self.a, self.b, self.alpha]
    }

    fn from_vec4(vec: [f32; 4]) -> Self {
        Self::new(vec[0], vec[1], vec[2], vec[3])
    }

    fn to_array(self) -> Array {
        array![
            Ratio::new(self.l as f64),
            self.a as f64,
            self.b as f64,
            Ratio::new(self.alpha as f64),
        ]
    }

    fn alpha(self) -> Option<f32> {
        Some(self.alpha)
    }

    fn to_rgba(self) -> RgbaColor {
        palette::rgb::Rgba::from_color(palette::oklab::Oklaba::from(self)).into()
    }

    fn to_oklab(self) -> OklabColor {
        self
    }

    fn to_linear_rgb(self) -> LinearRgbColor {
        palette::LinSrgba::from_color(palette::oklab::Oklaba::from(self)).into()
    }

    fn to_hsl(self) -> HslColor {
        palette::Hsla::from_color(palette::oklab::Oklaba::from(self)).into()
    }

    fn to_hsv(self) -> HsvColor {
        palette::Hsva::from_color(palette::oklab::Oklaba::from(self)).into()
    }

    fn to_cmyk(self) -> CmykColor {
        self.to_rgba().to_cmyk()
    }

    fn to_luma(self) -> LumaColor {
        palette::SrgbLuma::from_color(palette::oklab::Oklaba::from(self)).into()
    }

    fn lighten(self, factor: Ratio) -> Self {
        palette::oklab::Oklaba::from(self).lighten(factor.get() as f32).into()
    }

    fn darken(self, factor: Ratio) -> Self {
        palette::oklab::Oklaba::from(self).darken(factor.get() as f32).into()
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

impl Eq for OklabColor {}

impl Hash for OklabColor {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.l.to_bits().hash(state);
        self.a.to_bits().hash(state);
        self.b.to_bits().hash(state);
        self.alpha.to_bits().hash(state);
    }
}

impl Debug for OklabColor {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        if self.alpha == 1.0 {
            write!(f, "oklab({:?}, {}, {})", Ratio::new(self.l as f64), self.a, self.b)
        } else {
            write!(
                f,
                "oklab({:?}, {}, {}, {:?})",
                Ratio::new(self.l as f64),
                self.a,
                self.b,
                Ratio::new(self.alpha as f64)
            )
        }
    }
}

impl From<OklabColor> for Color {
    fn from(oklab: OklabColor) -> Self {
        Self::Oklab(oklab)
    }
}

#[derive(Copy, Clone, PartialEq)]
pub struct LinearRgbColor {
    /// The red component of the color.
    /// In the range [0, 1].
    pub r: f32,

    /// The green component of the color.
    /// In the range [0, 1].
    pub g: f32,

    /// The blue component of the color.
    /// In the range [0, 1].
    pub b: f32,

    /// The alpha component of the color.
    /// In the range [0, 1].
    pub a: f32,
}

impl From<LinearRgbColor> for palette::rgb::LinSrgba {
    fn from(color: LinearRgbColor) -> Self {
        Self::new(color.r, color.g, color.b, color.a)
    }
}

impl From<palette::rgb::LinSrgba> for LinearRgbColor {
    fn from(color: palette::rgb::LinSrgba) -> Self {
        Self::new(color.red, color.green, color.blue, color.alpha)
    }
}

impl ColorExt for LinearRgbColor {
    const COMPONENTS: usize = 4;

    fn to_vec4(self) -> [f32; 4] {
        [self.r, self.g, self.b, self.a]
    }

    fn from_vec4(vec: [f32; 4]) -> Self {
        Self::new(vec[0], vec[1], vec[2], vec[3])
    }

    fn alpha(self) -> Option<f32> {
        Some(self.a)
    }

    fn to_rgba(self) -> RgbaColor {
        palette::rgb::Srgba::from_color(palette::rgb::LinSrgba::from(self)).into()
    }

    fn to_oklab(self) -> OklabColor {
        palette::oklab::Oklaba::from_color(palette::rgb::LinSrgba::from(self)).into()
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
        palette::SrgbLuma::from_color(palette::rgb::LinSrgba::from(self)).into()
    }

    fn lighten(self, factor: Ratio) -> Self {
        palette::rgb::LinSrgba::from(self).lighten(factor.get() as f32).into()
    }

    fn darken(self, factor: Ratio) -> Self {
        palette::rgb::LinSrgba::from(self).darken(factor.get() as f32).into()
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
        Self::new(1.0 - self.r, 1.0 - self.g, 1.0 - self.b, self.a)
    }
}

impl LinearRgbColor {
    pub fn new(r: f32, g: f32, b: f32, a: f32) -> Self {
        Self {
            r: r.fix_precision(),
            g: g.fix_precision(),
            b: b.fix_precision(),
            a: a.fix_precision(),
        }
    }
}

impl Eq for LinearRgbColor {}

impl Hash for LinearRgbColor {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.r.to_bits().hash(state);
        self.g.to_bits().hash(state);
        self.b.to_bits().hash(state);
        self.a.to_bits().hash(state);
    }
}

impl Debug for LinearRgbColor {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        if self.a == 1.0 {
            write!(
                f,
                "linear-rgb({:?}, {:?}, {:?})",
                Ratio::new(self.r as f64),
                Ratio::new(self.g as f64),
                Ratio::new(self.b as f64),
            )
        } else {
            write!(
                f,
                "linear-rgb({:?}, {:?}, {:?}, {:?})",
                Ratio::new(self.r as f64),
                Ratio::new(self.g as f64),
                Ratio::new(self.b as f64),
                Ratio::new(self.a as f64),
            )
        }
    }
}

impl From<LinearRgbColor> for Color {
    fn from(linear_srgb: LinearRgbColor) -> Self {
        Self::LinearRgb(linear_srgb)
    }
}

/// An 8-bit RGBA color.
#[derive(Copy, Clone)]
pub enum RgbaColor {
    Floating {
        /// Red channel.
        r: f32,
        /// Green channel.
        g: f32,
        /// Blue channel.
        b: f32,
        /// Alpha channel.
        a: f32,
    },
    Integer {
        /// Red channel.
        r: u8,
        /// Green channel.
        g: u8,
        /// Blue channel.
        b: u8,
        /// Alpha channel.
        a: u8,
    },
}

impl Hash for RgbaColor {
    fn hash<H: Hasher>(&self, state: &mut H) {
        let [r, g, b, a] = self.to_vec4();
        r.to_bits().hash(state);
        g.to_bits().hash(state);
        b.to_bits().hash(state);
        a.to_bits().hash(state);
    }
}

impl Eq for RgbaColor {}

impl PartialEq for RgbaColor {
    fn eq(&self, other: &Self) -> bool {
        let [r1, g1, b1, a1] = self.to_vec4();
        let [r2, g2, b2, a2] = other.to_vec4();
        round_u8(r1 * 255.0) == round_u8(r2 * 255.0)
            && round_u8(g1 * 255.0) == round_u8(g2 * 255.0)
            && round_u8(b1 * 255.0) == round_u8(b2 * 255.0)
            && round_u8(a1 * 255.0) == round_u8(a2 * 255.0)
    }
}

impl From<RgbaColor> for palette::rgb::Rgba {
    fn from(value: RgbaColor) -> Self {
        let [r, g, b, a] = value.to_vec4();
        Self::new(r, g, b, a)
    }
}

impl From<palette::rgb::Rgba> for RgbaColor {
    fn from(rgba: palette::rgb::Rgba) -> Self {
        Self::new(rgba.red, rgba.green, rgba.blue, rgba.alpha)
    }
}

impl ColorExt for RgbaColor {
    const COMPONENTS: usize = 4;

    fn to_vec4(self) -> [f32; 4] {
        match self {
            RgbaColor::Floating { r, g, b, a } => [r, g, b, a],
            RgbaColor::Integer { r, g, b, a } => {
                let f = |r| (r as f32 / 255.0).fix_precision();
                [f(r), f(g), f(b), f(a)]
            }
        }
    }

    fn from_vec4(vec: [f32; 4]) -> Self {
        Self::Floating { r: vec[0], g: vec[1], b: vec[2], a: vec[3] }
    }

    fn to_array(self) -> Array {
        let [r, g, b, a] = self.to_vec4();
        if a <= 1.0 {
            array![
                round_u8(r * 255.0),
                round_u8(g * 255.0),
                round_u8(b * 255.0),
                round_u8(a * 255.0)
            ]
        } else {
            array![round_u8(r * 255.0), round_u8(g * 255.0), round_u8(b * 255.0)]
        }
    }

    fn alpha(self) -> Option<f32> {
        Some(match self {
            RgbaColor::Floating { a, .. } => a,
            RgbaColor::Integer { a, .. } => a as f32 / 255.0,
        })
    }

    fn to_rgba(self) -> RgbaColor {
        self
    }

    fn to_oklab(self) -> OklabColor {
        palette::oklab::Oklaba::from_color(palette::rgb::Srgba::from(self)).into()
    }

    fn to_linear_rgb(self) -> LinearRgbColor {
        palette::rgb::LinSrgba::from_color(palette::rgb::Srgba::from(self)).into()
    }

    fn to_hsl(self) -> HslColor {
        palette::Hsla::from_color(palette::rgb::Srgba::from(self)).into()
    }

    fn to_hsv(self) -> HsvColor {
        palette::Hsva::from_color(palette::rgb::Srgba::from(self)).into()
    }

    fn to_cmyk(self) -> CmykColor {
        let [r, g, b, _] = self.to_vec4();

        let k = 1.0 - r.max(g).max(b);
        let c = (1.0 - r - k) / (1.0 - k);
        let m = (1.0 - g - k) / (1.0 - k);
        let y = (1.0 - b - k) / (1.0 - k);

        CmykColor::new(c, m, y, k)
    }

    fn to_luma(self) -> LumaColor {
        palette::SrgbLuma::from_color(palette::rgb::Srgba::from(self)).into()
    }

    fn lighten(self, factor: Ratio) -> Self {
        palette::rgb::Srgba::from(self).lighten(factor.get() as f32).into()
    }

    fn darken(self, factor: Ratio) -> Self {
        palette::rgb::Srgba::from(self).darken(factor.get() as f32).into()
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
        match self {
            Self::Floating { r, g, b, a } => Self::new(1.0 - r, 1.0 - g, 1.0 - b, a),
            Self::Integer { r, g, b, a } => {
                Self::Integer { r: u8::MAX - r, g: u8::MAX - g, b: u8::MAX - b, a }
            }
        }
    }

    fn to_hex(self) -> EcoString {
        let (r, g, b, a) = match self {
            Self::Floating { r, g, b, a } => (
                round_u8(r * 255.0),
                round_u8(g * 255.0),
                round_u8(b * 255.0),
                round_u8(a * 255.0),
            ),
            Self::Integer { r, g, b, a } => (r, g, b, a),
        };
        if self.alpha() != Some(1.0) {
            eco_format!("#{:02x}{:02x}{:02x}{:02x}", r, g, b, a)
        } else {
            eco_format!("#{:02x}{:02x}{:02x}", r, g, b)
        }
    }
}

impl RgbaColor {
    /// Construct a new RGBA color.
    pub fn new(r: f32, g: f32, b: f32, a: f32) -> Self {
        Self::Floating {
            r: r.fix_precision(),
            g: g.fix_precision(),
            b: b.fix_precision(),
            a: a.fix_precision(),
        }
    }

    /// Construct a new RGBA color from 8-bit values.
    pub const fn new_from_u8(r: u8, g: u8, b: u8, a: u8) -> Self {
        Self::Integer { r, g, b, a }
    }

    /// Converts a 32-bit integer to an RGBA color.
    #[inline]
    pub const fn from_u32(color: u32) -> Self {
        Self::new_from_u8(
            ((color >> 24) & 0xFF) as u8,
            ((color >> 16) & 0xFF) as u8,
            ((color >> 8) & 0xFF) as u8,
            (color & 0xFF) as u8,
        )
    }

    pub fn to_vec4_u8(self) -> [u8; 4] {
        match self {
            Self::Floating { r, g, b, a } => {
                let f = |r: f32| (r * 255.0).round() as u8;
                [f(r), f(g), f(b), f(a)]
            }
            Self::Integer { r, g, b, a } => [r, g, b, a],
        }
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

        Ok(Self::new_from_u8(values[0], values[1], values[2], values[3]))
    }
}

impl Debug for RgbaColor {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        let [r, g, b, a] = self.to_vec4();
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
#[derive(Copy, Clone, PartialEq)]
pub struct CmykColor {
    /// The cyan component.
    pub c: f32,
    /// The magenta component.
    pub m: f32,
    /// The yellow component.
    pub y: f32,
    /// The key (black) component.
    pub k: f32,
}

impl Hash for CmykColor {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.c.to_bits().hash(state);
        self.m.to_bits().hash(state);
        self.y.to_bits().hash(state);
        self.k.to_bits().hash(state);
    }
}

impl Eq for CmykColor {}

impl ColorExt for CmykColor {
    const COMPONENTS: usize = 4;

    fn to_vec4(self) -> [f32; 4] {
        [self.c, self.m, self.y, self.k]
    }

    fn from_vec4(vec: [f32; 4]) -> Self {
        Self::new(vec[0], vec[1], vec[2], vec[3])
    }

    fn to_array(self) -> Array {
        let [c, m, y, k] = self.to_vec4();
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
        let r = (1.0 - self.c) * (1.0 - self.k);
        let g = (1.0 - self.m) * (1.0 - self.k);
        let b = (1.0 - self.y) * (1.0 - self.k);

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
        let lighten = |u: f32| (u - u * factor.get() as f32).clamp(0.0, 1.0);
        Self::new(lighten(self.c), lighten(self.m), lighten(self.y), lighten(self.k))
    }

    fn darken(self, factor: Ratio) -> Self {
        let darken = |u: f32| (u + (1.0 - u) * factor.get() as f32).clamp(0.0, 1.0);
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
        Self::new(1.0 - self.c, 1.0 - self.m, 1.0 - self.y, self.k)
    }
}

impl CmykColor {
    /// Construct a new CMYK color.
    pub fn new(c: f32, m: f32, y: f32, k: f32) -> Self {
        Self {
            c: c.fix_precision(),
            m: m.fix_precision(),
            y: y.fix_precision(),
            k: k.fix_precision(),
        }
    }
}

impl Debug for CmykColor {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        write!(
            f,
            "cmyk({:?}, {:?}, {:?}, {:?})",
            Ratio::new(self.c as f64),
            Ratio::new(self.m as f64),
            Ratio::new(self.y as f64),
            Ratio::new(self.k as f64),
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
#[derive(Copy, Clone, PartialEq)]
pub struct HslColor {
    pub h: Angle,
    pub s: f32,
    pub l: f32,
    pub a: f32,
}

impl Eq for HslColor {}

impl From<HslColor> for Color {
    fn from(value: HslColor) -> Self {
        Self::Hsl(value)
    }
}

impl Hash for HslColor {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.h.hash(state);
        self.s.to_bits().hash(state);
        self.l.to_bits().hash(state);
        self.a.to_bits().hash(state);
    }
}

impl Debug for HslColor {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        if self.a == 1.0 {
            write!(
                f,
                "hsl({:?}, {:?}, {:?})",
                self.h,
                Ratio::new(self.s as f64),
                Ratio::new(self.l as f64),
            )
        } else {
            write!(
                f,
                "hsl({:?}, {:?}, {:?}, {:?})",
                self.h,
                Ratio::new(self.s as f64),
                Ratio::new(self.l as f64),
                Ratio::new(self.a as f64),
            )
        }
    }
}

impl From<HslColor> for palette::Hsla {
    fn from(hsl: HslColor) -> Self {
        Self::new(palette::RgbHue::new(hsl.h.to_rad() as f32), hsl.s, hsl.l, hsl.a)
    }
}

impl From<palette::Hsla> for HslColor {
    fn from(hsl: palette::Hsla) -> Self {
        Self::new(
            Angle::rad(hsl.hue.into_radians() as f64),
            hsl.saturation,
            hsl.lightness,
            hsl.alpha,
        )
    }
}

impl HslColor {
    pub fn new(h: Angle, s: f32, l: f32, a: f32) -> Self {
        Self {
            h: Angle::deg(h.to_deg().fix_precision()),
            s: s.fix_precision(),
            l: l.fix_precision(),
            a: a.fix_precision(),
        }
    }
}

impl ColorExt for HslColor {
    const COMPONENTS: usize = 4;

    fn to_vec4(self) -> [f32; 4] {
        [self.h.to_rad() as f32, self.s, self.l, self.a]
    }

    fn from_vec4(vec: [f32; 4]) -> Self {
        Self::new(Angle::rad(vec[0].fix_precision() as f64), vec[1], vec[2], vec[3])
    }

    fn to_array(self) -> Array {
        array![self.h, self.s as f64, self.l as f64, Ratio::new(self.a as f64),]
    }

    fn to_rgba(self) -> RgbaColor {
        palette::rgb::Rgba::from_color(palette::Hsla::from(self)).into()
    }

    fn to_oklab(self) -> OklabColor {
        palette::Oklaba::from_color(palette::Hsla::from(self)).into()
    }

    fn to_linear_rgb(self) -> LinearRgbColor {
        self.to_rgba().to_linear_rgb()
    }

    fn to_hsl(self) -> HslColor {
        self
    }

    fn to_hsv(self) -> HsvColor {
        palette::Hsva::from_color(palette::Hsla::from(self)).into()
    }

    fn to_cmyk(self) -> CmykColor {
        self.to_rgba().to_cmyk()
    }

    fn to_luma(self) -> LumaColor {
        palette::SrgbLuma::from_color(palette::Hsla::from(self)).into()
    }

    fn lighten(self, factor: Ratio) -> Self {
        palette::Hsla::from(self).lighten(factor.get() as f32).into()
    }

    fn darken(self, factor: Ratio) -> Self {
        palette::Hsla::from(self).darken(factor.get() as f32).into()
    }

    fn saturate(self, factor: Ratio) -> Self {
        palette::Hsla::from(self).saturate(factor.get() as f32).into()
    }

    fn desaturate(self, factor: Ratio) -> Self {
        palette::Hsla::from(self).desaturate(factor.get() as f32).into()
    }

    fn hue_rotate(self, hue: Angle) -> Self {
        palette::Hsla::from(self).shift_hue(hue.to_deg() as f32).into()
    }

    fn negate(self) -> Self {
        Self::new(Angle::deg(180.0) - self.h, self.s, self.l, self.a)
    }

    fn alpha(self) -> Option<f32> {
        Some(self.a)
    }
}

/// A 32-bit HslA color.
#[derive(Copy, Clone, PartialEq)]
pub struct HsvColor {
    pub h: Angle,
    pub s: f32,
    pub v: f32,
    pub a: f32,
}

impl Eq for HsvColor {}

impl From<HsvColor> for Color {
    fn from(value: HsvColor) -> Self {
        Color::Hsv(value)
    }
}

impl Hash for HsvColor {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.h.hash(state);
        self.s.to_bits().hash(state);
        self.v.to_bits().hash(state);
        self.a.to_bits().hash(state);
    }
}

impl Debug for HsvColor {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        if self.a == 1.0 {
            write!(
                f,
                "hsv({:?}, {:?}, {:?})",
                self.h,
                Ratio::new(self.s as f64),
                Ratio::new(self.v as f64),
            )
        } else {
            write!(
                f,
                "hsv({:?}, {:?}, {:?}, {:?})",
                self.h,
                Ratio::new(self.s as f64),
                Ratio::new(self.v as f64),
                Ratio::new(self.a as f64),
            )
        }
    }
}

impl From<HsvColor> for palette::Hsva {
    fn from(value: HsvColor) -> Self {
        Self::new(
            palette::RgbHue::new(value.h.to_deg() as f32),
            value.s,
            value.v,
            value.a,
        )
    }
}

impl From<palette::Hsva> for HsvColor {
    fn from(hsl: palette::Hsva) -> Self {
        Self::new(
            Angle::rad(hsl.hue.into_radians() as f64),
            hsl.saturation,
            hsl.value,
            hsl.alpha,
        )
    }
}

impl HsvColor {
    pub fn new(h: Angle, s: f32, v: f32, a: f32) -> Self {
        Self {
            h: Angle::deg(h.to_deg().fix_precision()),
            s: s.fix_precision(),
            v: v.fix_precision(),
            a: a.fix_precision(),
        }
    }
}

impl ColorExt for HsvColor {
    const COMPONENTS: usize = 4;

    fn to_vec4(self) -> [f32; 4] {
        [self.h.to_rad() as f32, self.s, self.v, self.a]
    }

    fn from_vec4(vec: [f32; 4]) -> Self {
        Self::new(Angle::rad(vec[0].fix_precision() as f64), vec[1], vec[2], vec[3])
    }

    fn to_array(self) -> Array {
        array![self.h, self.s as f64, self.v as f64, Ratio::new(self.a as f64),]
    }

    fn to_rgba(self) -> RgbaColor {
        palette::rgb::Rgba::from_color(palette::Hsva::from(self)).into()
    }

    fn to_oklab(self) -> OklabColor {
        palette::Oklaba::from_color(palette::Hsva::from(self)).into()
    }

    fn to_linear_rgb(self) -> LinearRgbColor {
        self.to_rgba().to_linear_rgb()
    }

    fn to_hsl(self) -> HslColor {
        palette::Hsla::from_color(palette::Hsva::from(self)).into()
    }

    fn to_hsv(self) -> HsvColor {
        self
    }

    fn to_cmyk(self) -> CmykColor {
        self.to_rgba().to_cmyk()
    }

    fn to_luma(self) -> LumaColor {
        palette::SrgbLuma::from_color(palette::Hsva::from(self)).into()
    }

    fn lighten(self, factor: Ratio) -> Self {
        palette::Hsva::from(self).lighten(factor.get() as f32).into()
    }

    fn darken(self, factor: Ratio) -> Self {
        palette::Hsva::from(self).darken(factor.get() as f32).into()
    }

    fn saturate(self, factor: Ratio) -> Self {
        palette::Hsva::from(self).saturate(factor.get() as f32).into()
    }

    fn desaturate(self, factor: Ratio) -> Self {
        palette::Hsva::from(self).desaturate(factor.get() as f32).into()
    }

    fn hue_rotate(self, hue: Angle) -> Self {
        palette::Hsva::from(self).shift_hue(hue.to_deg() as f32).into()
    }

    fn negate(self) -> Self {
        Self::new(Angle::deg(180.0) - self.h, self.s, self.v, self.a)
    }

    fn alpha(self) -> Option<f32> {
        Some(self.a)
    }
}

/// Convert to the closest u8.
fn round_u8(value: f32) -> u8 {
    value.round() as u8
}

trait FloatPrecision: Sized {
    fn fix_precision(self) -> Self;
}

impl FloatPrecision for f32 {
    fn fix_precision(self) -> Self {
        (self * PRECISION).round() / PRECISION
    }
}

impl FloatPrecision for f64 {
    fn fix_precision(self) -> Self {
        (self * PRECISION as f64).round() / PRECISION as f64
    }
}

/// A component that must be a ratio.
pub struct RatioComponent(Ratio);

cast! {
    RatioComponent,
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
        bail!("ratio must be between -40% and 40%");
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
            assert_eq!(RgbaColor::from_str(hex), Ok(RgbaColor::new_from_u8(r, g, b, a)));
        }

        test("f61243ff", 0xf6, 0x12, 0x43, 0xff);
        test("b3d8b3", 0xb3, 0xd8, 0xb3, 0xff);
        test("fCd2a9AD", 0xfc, 0xd2, 0xa9, 0xad);
        test("233", 0x22, 0x33, 0x33, 0xff);
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
