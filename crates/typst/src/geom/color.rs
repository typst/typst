use ecow::{eco_format, EcoString};
use std::str::FromStr;

use super::*;
use crate::diag::{bail, At, SourceResult};
use crate::eval::{cast, Args, Array, Cast, Func, Str};
use crate::syntax::Spanned;

/// A color in a specific color space.
///
/// Typst supports:
/// - sRGB through the [`rgb` function]($rgb)
/// - Device CMYK through [`cmyk` function]($cmyk)
/// - D65 Gray through the [`luma` function]($luma)
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
#[ty(scope)]
#[derive(Copy, Clone, Eq, PartialEq, Hash)]
pub enum Color {
    /// An 8-bit luma color.
    Luma(LumaColor),
    /// An 8-bit RGBA color.
    Rgba(RgbaColor),
    /// An 8-bit CMYK color.
    Cmyk(CmykColor),
}

impl Color {
    /// Convert this color to RGBA.
    pub fn to_rgba(self) -> RgbaColor {
        match self {
            Self::Luma(luma) => luma.to_rgba(),
            Self::Rgba(rgba) => rgba,
            Self::Cmyk(cmyk) => cmyk.to_rgba(),
        }
    }
}

#[scope]
impl Color {
    pub const BLACK: Self = Self::Rgba(RgbaColor::new(0x00, 0x00, 0x00, 0xFF));
    pub const GRAY: Self = Self::Rgba(RgbaColor::new(0xAA, 0xAA, 0xAA, 0xFF));
    pub const SILVER: Self = Self::Rgba(RgbaColor::new(0xDD, 0xDD, 0xDD, 0xFF));
    pub const WHITE: Self = Self::Rgba(RgbaColor::new(0xFF, 0xFF, 0xFF, 0xFF));
    pub const NAVY: Self = Self::Rgba(RgbaColor::new(0x00, 0x1f, 0x3f, 0xFF));
    pub const BLUE: Self = Self::Rgba(RgbaColor::new(0x00, 0x74, 0xD9, 0xFF));
    pub const AQUA: Self = Self::Rgba(RgbaColor::new(0x7F, 0xDB, 0xFF, 0xFF));
    pub const TEAL: Self = Self::Rgba(RgbaColor::new(0x39, 0xCC, 0xCC, 0xFF));
    pub const EASTERN: Self = Self::Rgba(RgbaColor::new(0x23, 0x9D, 0xAD, 0xFF));
    pub const PURPLE: Self = Self::Rgba(RgbaColor::new(0xB1, 0x0D, 0xC9, 0xFF));
    pub const FUCHSIA: Self = Self::Rgba(RgbaColor::new(0xF0, 0x12, 0xBE, 0xFF));
    pub const MAROON: Self = Self::Rgba(RgbaColor::new(0x85, 0x14, 0x4b, 0xFF));
    pub const RED: Self = Self::Rgba(RgbaColor::new(0xFF, 0x41, 0x36, 0xFF));
    pub const ORANGE: Self = Self::Rgba(RgbaColor::new(0xFF, 0x85, 0x1B, 0xFF));
    pub const YELLOW: Self = Self::Rgba(RgbaColor::new(0xFF, 0xDC, 0x00, 0xFF));
    pub const OLIVE: Self = Self::Rgba(RgbaColor::new(0x3D, 0x99, 0x70, 0xFF));
    pub const GREEN: Self = Self::Rgba(RgbaColor::new(0x2E, 0xCC, 0x40, 0xFF));
    pub const LIME: Self = Self::Rgba(RgbaColor::new(0x01, 0xFF, 0x70, 0xFF));

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
        LumaColor::new(gray.0).into()
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
            let Component(a) = args.eat()?.unwrap_or(Component(255));
            RgbaColor::new(r, g, b, a).into()
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
        CmykColor::new(cyan.0, magenta.0, yellow.0, key.0).into()
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
            Self::Luma(_) => Self::luma_data().into(),
            Self::Rgba(_) => Self::rgb_data().into(),
            Self::Cmyk(_) => Self::cmyk_data().into(),
        }
    }

    /// Returns the color's RGB(A) hex representation (such as `#ffaa32` or
    /// `#020304fe`). The alpha component (last two digits in `#020304fe`) is
    /// omitted if it is equal to `ff` (255 / 100%).
    #[func]
    pub fn to_hex(self) -> EcoString {
        self.to_rgba().to_hex()
    }

    /// Converts this color to sRGB and returns its components (R, G, B, A) as
    /// an array of [integers]($int).
    #[func(name = "to-rgba")]
    pub fn to_rgba_array(self) -> Array {
        self.to_rgba().to_array()
    }

    /// Converts this color to Digital CMYK and returns its components
    /// (C, M, Y, K) as an array of [ratios]($ratio). Note that this function
    /// will throw an error when applied to an [rgb]($rgb) color, since its
    /// conversion to CMYK is not available.
    #[func]
    pub fn to_cmyk(self) -> StrResult<Array> {
        match self {
            Self::Luma(luma) => Ok(luma.to_cmyk().to_array()),
            Self::Rgba(_) => {
                bail!("cannot obtain cmyk values from rgba color")
            }
            Self::Cmyk(cmyk) => Ok(cmyk.to_array()),
        }
    }

    /// If this color was created with [luma]($luma), returns the
    /// [integer]($int) value used on construction. Otherwise (for [rgb]($rgb)
    /// and [cmyk]($cmyk) colors), throws an error.
    #[func]
    pub fn to_luma(self) -> StrResult<u8> {
        match self {
            Self::Luma(luma) => Ok(luma.0),
            Self::Rgba(_) => {
                bail!("cannot obtain the luma value of rgba color")
            }
            Self::Cmyk(_) => {
                bail!("cannot obtain the luma value of cmyk color")
            }
        }
    }

    /// Lightens a color by a given factor.
    #[func]
    pub fn lighten(
        self,
        /// The factor to lighten the color by.
        factor: Ratio,
    ) -> Color {
        match self {
            Self::Luma(luma) => Self::Luma(luma.lighten(factor)),
            Self::Rgba(rgba) => Self::Rgba(rgba.lighten(factor)),
            Self::Cmyk(cmyk) => Self::Cmyk(cmyk.lighten(factor)),
        }
    }

    /// Darkens a color by a given factor.
    #[func]
    pub fn darken(
        self,
        /// The factor to darken the color by.
        factor: Ratio,
    ) -> Color {
        match self {
            Self::Luma(luma) => Self::Luma(luma.darken(factor)),
            Self::Rgba(rgba) => Self::Rgba(rgba.darken(factor)),
            Self::Cmyk(cmyk) => Self::Cmyk(cmyk.darken(factor)),
        }
    }

    /// Produces the negative of the color.
    #[func]
    pub fn negate(self) -> Color {
        match self {
            Self::Luma(luma) => Self::Luma(luma.negate()),
            Self::Rgba(rgba) => Self::Rgba(rgba.negate()),
            Self::Cmyk(cmyk) => Self::Cmyk(cmyk.negate()),
        }
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
            let v = rgba_to_vec4(color.to_rgba(), space);
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
        Ok(vec4_to_rgba(mixed, space).into())
    }
}

impl Debug for Color {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        match self {
            Self::Luma(c) => Debug::fmt(c, f),
            Self::Rgba(c) => Debug::fmt(c, f),
            Self::Cmyk(c) => Debug::fmt(c, f),
        }
    }
}

/// A color with a weight.
pub struct WeightedColor(Color, f32);

cast! {
    WeightedColor,
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
fn rgba_to_vec4(color: RgbaColor, space: ColorSpace) -> [f32; 4] {
    match space {
        ColorSpace::Oklab => {
            let RgbaColor { r, g, b, a } = color;
            let oklab = oklab::srgb_to_oklab(oklab::RGB { r, g, b });
            [oklab.l, oklab.a, oklab.b, a as f32 / 255.0]
        }
        ColorSpace::Srgb => {
            let RgbaColor { r, g, b, a } = color;
            [r as f32 / 255.0, g as f32 / 255.0, b as f32 / 255.0, a as f32 / 255.0]
        }
    }
}

/// Convert four components in the given color space to RGBA.
fn vec4_to_rgba(vec: [f32; 4], space: ColorSpace) -> RgbaColor {
    match space {
        ColorSpace::Oklab => {
            let [l, a, b, alpha] = vec;
            let oklab::RGB { r, g, b } = oklab::oklab_to_srgb(oklab::Oklab { l, a, b });
            RgbaColor { r, g, b, a: (alpha * 255.0).round() as u8 }
        }
        ColorSpace::Srgb => {
            let [r, g, b, a] = vec;
            RgbaColor {
                r: (r * 255.0).round() as u8,
                g: (g * 255.0).round() as u8,
                b: (b * 255.0).round() as u8,
                a: (a * 255.0).round() as u8,
            }
        }
    }
}

/// A color space for mixing.
#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash, Cast)]
pub enum ColorSpace {
    /// A perceptual color space.
    Oklab,
    /// The standard RGB color space.
    Srgb,
}

/// An 8-bit grayscale color.
#[derive(Copy, Clone, Eq, PartialEq, Hash)]
pub struct LumaColor(pub u8);

impl LumaColor {
    /// Construct a new luma color.
    pub const fn new(luma: u8) -> Self {
        Self(luma)
    }

    /// Convert to an opque RGBA color.
    pub const fn to_rgba(self) -> RgbaColor {
        RgbaColor::new(self.0, self.0, self.0, u8::MAX)
    }

    /// Convert to CMYK as a fraction of true black.
    pub fn to_cmyk(self) -> CmykColor {
        CmykColor::new(
            round_u8(self.0 as f64 * 0.75),
            round_u8(self.0 as f64 * 0.68),
            round_u8(self.0 as f64 * 0.67),
            round_u8(self.0 as f64 * 0.90),
        )
    }

    /// Lighten this color by a factor.
    pub fn lighten(self, factor: Ratio) -> Self {
        let inc = round_u8((u8::MAX - self.0) as f64 * factor.get());
        Self(self.0.saturating_add(inc))
    }

    /// Darken this color by a factor.
    pub fn darken(self, factor: Ratio) -> Self {
        let dec = round_u8(self.0 as f64 * factor.get());
        Self(self.0.saturating_sub(dec))
    }

    /// Negate this color.
    pub fn negate(self) -> Self {
        Self(u8::MAX - self.0)
    }
}

impl Debug for LumaColor {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        write!(f, "luma({})", self.0)
    }
}

impl From<LumaColor> for Color {
    fn from(luma: LumaColor) -> Self {
        Self::Luma(luma)
    }
}

/// An 8-bit RGBA color.
#[derive(Copy, Clone, Eq, PartialEq, Hash)]
pub struct RgbaColor {
    /// Red channel.
    pub r: u8,
    /// Green channel.
    pub g: u8,
    /// Blue channel.
    pub b: u8,
    /// Alpha channel.
    pub a: u8,
}

impl RgbaColor {
    /// Construct a new RGBA color.
    pub const fn new(r: u8, g: u8, b: u8, a: u8) -> Self {
        Self { r, g, b, a }
    }

    /// Lighten this color by a factor.
    ///
    /// The alpha channel is not affected.
    pub fn lighten(self, factor: Ratio) -> Self {
        let lighten =
            |c: u8| c.saturating_add(round_u8((u8::MAX - c) as f64 * factor.get()));
        Self {
            r: lighten(self.r),
            g: lighten(self.g),
            b: lighten(self.b),
            a: self.a,
        }
    }

    /// Darken this color by a factor.
    ///
    /// The alpha channel is not affected.
    pub fn darken(self, factor: Ratio) -> Self {
        let darken = |c: u8| c.saturating_sub(round_u8(c as f64 * factor.get()));
        Self {
            r: darken(self.r),
            g: darken(self.g),
            b: darken(self.b),
            a: self.a,
        }
    }

    /// Negate this color.
    ///
    /// The alpha channel is not affected.
    pub fn negate(self) -> Self {
        Self {
            r: u8::MAX - self.r,
            g: u8::MAX - self.g,
            b: u8::MAX - self.b,
            a: self.a,
        }
    }

    /// Converts this color to a RGB Hex Code.
    pub fn to_hex(self) -> EcoString {
        if self.a != 255 {
            eco_format!("#{:02x}{:02x}{:02x}{:02x}", self.r, self.g, self.b, self.a)
        } else {
            eco_format!("#{:02x}{:02x}{:02x}", self.r, self.g, self.b)
        }
    }

    /// Converts this color to an array of R, G, B, A components.
    pub fn to_array(self) -> Array {
        array![self.r, self.g, self.b, self.a]
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

        Ok(Self::new(values[0], values[1], values[2], values[3]))
    }
}

impl Debug for RgbaColor {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        if f.alternate() {
            write!(f, "rgba({}, {}, {}, {})", self.r, self.g, self.b, self.a,)?;
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
#[derive(Copy, Clone, Eq, PartialEq, Hash)]
pub struct CmykColor {
    /// The cyan component.
    pub c: u8,
    /// The magenta component.
    pub m: u8,
    /// The yellow component.
    pub y: u8,
    /// The key (black) component.
    pub k: u8,
}

impl CmykColor {
    /// Construct a new CMYK color.
    pub const fn new(c: u8, m: u8, y: u8, k: u8) -> Self {
        Self { c, m, y, k }
    }

    /// Convert this color to RGBA.
    pub fn to_rgba(self) -> RgbaColor {
        let k = self.k as f64 / 255.0;
        let f = |c| {
            let c = c as f64 / 255.0;
            round_u8(255.0 * (1.0 - c) * (1.0 - k))
        };

        RgbaColor { r: f(self.c), g: f(self.m), b: f(self.y), a: 255 }
    }

    /// Lighten this color by a factor.
    pub fn lighten(self, factor: Ratio) -> Self {
        let lighten = |c: u8| c.saturating_sub(round_u8(c as f64 * factor.get()));
        Self {
            c: lighten(self.c),
            m: lighten(self.m),
            y: lighten(self.y),
            k: lighten(self.k),
        }
    }

    /// Darken this color by a factor.
    pub fn darken(self, factor: Ratio) -> Self {
        let darken =
            |c: u8| c.saturating_add(round_u8((u8::MAX - c) as f64 * factor.get()));
        Self {
            c: darken(self.c),
            m: darken(self.m),
            y: darken(self.y),
            k: darken(self.k),
        }
    }

    /// Negate this color.
    ///
    /// Does not affect the key component.
    pub fn negate(self) -> Self {
        Self {
            c: u8::MAX - self.c,
            m: u8::MAX - self.m,
            y: u8::MAX - self.y,
            k: self.k,
        }
    }

    /// Converts this color to an array of C, M, Y, K components.
    pub fn to_array(self) -> Array {
        // convert to ratio
        let g = |c| Ratio::new(c as f64 / 255.0);
        array![g(self.c), g(self.m), g(self.y), g(self.k)]
    }
}

impl Debug for CmykColor {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        let g = |c| 100.0 * (c as f64 / 255.0);
        write!(
            f,
            "cmyk({:.1}%, {:.1}%, {:.1}%, {:.1}%)",
            g(self.c),
            g(self.m),
            g(self.y),
            g(self.k),
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

/// An integer or ratio component.
pub struct Component(u8);

cast! {
    Component,
    v: i64 => match v {
        0 ..= 255 => Self(v as u8),
        _ => bail!("number must be between 0 and 255"),
    },
    v: Ratio => if (0.0 ..= 1.0).contains(&v.get()) {
        Self((v.get() * 255.0).round() as u8)
    } else {
        bail!("ratio must be between 0% and 100%");
    },
}

/// A component that must be a ratio.
pub struct RatioComponent(u8);

cast! {
    RatioComponent,
    v: Ratio => if (0.0 ..= 1.0).contains(&v.get()) {
        Self((v.get() * 255.0).round() as u8)
    } else {
        bail!("ratio must be between 0% and 100%");
    },
}

/// Convert to the closest u8.
fn round_u8(value: f64) -> u8 {
    value.round() as u8
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_color_strings() {
        #[track_caller]
        fn test(hex: &str, r: u8, g: u8, b: u8, a: u8) {
            assert_eq!(RgbaColor::from_str(hex), Ok(RgbaColor::new(r, g, b, a)));
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
