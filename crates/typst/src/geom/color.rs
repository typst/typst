use std::str::FromStr;

use super::*;

/// A color in a dynamic format.
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

    /// Convert this color to RGBA.
    pub fn to_rgba(self) -> RgbaColor {
        match self {
            Self::Luma(luma) => luma.to_rgba(),
            Self::Rgba(rgba) => rgba,
            Self::Cmyk(cmyk) => cmyk.to_rgba(),
        }
    }

    /// Lighten this color by the given factor.
    pub fn lighten(self, factor: Ratio) -> Self {
        match self {
            Self::Luma(luma) => Self::Luma(luma.lighten(factor)),
            Self::Rgba(rgba) => Self::Rgba(rgba.lighten(factor)),
            Self::Cmyk(cmyk) => Self::Cmyk(cmyk.lighten(factor)),
        }
    }

    /// Darken this color by the given factor.
    pub fn darken(self, factor: Ratio) -> Self {
        match self {
            Self::Luma(luma) => Self::Luma(luma.darken(factor)),
            Self::Rgba(rgba) => Self::Rgba(rgba.darken(factor)),
            Self::Cmyk(cmyk) => Self::Cmyk(cmyk.darken(factor)),
        }
    }

    /// Negate this color.
    pub fn negate(self) -> Self {
        match self {
            Self::Luma(luma) => Self::Luma(luma.negate()),
            Self::Rgba(rgba) => Self::Rgba(rgba.negate()),
            Self::Cmyk(cmyk) => Self::Cmyk(cmyk.negate()),
        }
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
            write!(f, "rgb(\"#{:02x}{:02x}{:02x}", self.r, self.g, self.b)?;
            if self.a != 255 {
                write!(f, "{:02x}", self.a)?;
            }
            write!(f, "\")")?;
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
