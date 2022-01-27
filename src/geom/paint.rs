use std::fmt::Display;
use std::str::FromStr;

use syntect::highlighting::Color as SynColor;

use super::*;

/// How a fill or stroke should be painted.
#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
pub enum Paint {
    /// A solid color.
    Solid(Color),
}

impl<T> From<T> for Paint
where
    T: Into<Color>,
{
    fn from(t: T) -> Self {
        Self::Solid(t.into())
    }
}

/// A color in a dynamic format.
#[derive(Copy, Clone, Eq, PartialEq, Hash)]
pub enum Color {
    /// An 8-bit RGBA color.
    Rgba(RgbaColor),
}

impl Debug for Color {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        match self {
            Self::Rgba(c) => Debug::fmt(c, f),
        }
    }
}

impl<T> From<T> for Color
where
    T: Into<RgbaColor>,
{
    fn from(rgba: T) -> Self {
        Self::Rgba(rgba.into())
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
    /// Black color.
    pub const BLACK: Self = Self { r: 0, g: 0, b: 0, a: 255 };

    /// White color.
    pub const WHITE: Self = Self { r: 255, g: 255, b: 255, a: 255 };

    /// Construct a new RGBA color.
    pub fn new(r: u8, g: u8, b: u8, a: u8) -> Self {
        Self { r, g, b, a }
    }

    /// Construct a new, opaque gray color.
    pub fn gray(luma: u8) -> Self {
        Self::new(luma, luma, luma, 255)
    }
}

impl FromStr for RgbaColor {
    type Err = RgbaError;

    /// Constructs a new color from hex strings like the following:
    /// - `#aef` (shorthand, with leading hashtag),
    /// - `7a03c2` (without alpha),
    /// - `abcdefff` (with alpha).
    ///
    /// The hashtag is optional and both lower and upper case are fine.
    fn from_str(hex_str: &str) -> Result<Self, Self::Err> {
        let hex_str = hex_str.strip_prefix('#').unwrap_or(hex_str);
        if !hex_str.is_ascii() {
            return Err(RgbaError);
        }

        let len = hex_str.len();
        let long = len == 6 || len == 8;
        let short = len == 3 || len == 4;
        let alpha = len == 4 || len == 8;

        if !long && !short {
            return Err(RgbaError);
        }

        let mut values: [u8; 4] = [255; 4];

        for elem in if alpha { 0 .. 4 } else { 0 .. 3 } {
            let item_len = if long { 2 } else { 1 };
            let pos = elem * item_len;

            let item = &hex_str[pos .. (pos + item_len)];
            values[elem] = u8::from_str_radix(item, 16).map_err(|_| RgbaError)?;

            if short {
                // Duplicate number for shorthand notation, i.e. `a` -> `aa`
                values[elem] += values[elem] * 16;
            }
        }

        Ok(Self::new(values[0], values[1], values[2], values[3]))
    }
}

impl From<SynColor> for RgbaColor {
    fn from(color: SynColor) -> Self {
        Self::new(color.r, color.b, color.g, color.a)
    }
}

impl Debug for RgbaColor {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        if f.alternate() {
            write!(
                f,
                "rgba({:02}, {:02}, {:02}, {:02})",
                self.r, self.g, self.b, self.a,
            )?;
        } else {
            write!(f, "#{:02x}{:02x}{:02x}", self.r, self.g, self.b)?;
            if self.a != 255 {
                write!(f, "{:02x}", self.a)?;
            }
        }
        Ok(())
    }
}

/// The error when parsing an [`RgbaColor`] from a string fails.
#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub struct RgbaError;

impl Display for RgbaError {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        f.pad("invalid hex string")
    }
}

impl std::error::Error for RgbaError {}

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
        fn test(hex: &str) {
            assert_eq!(RgbaColor::from_str(hex), Err(RgbaError));
        }

        test("12345");
        test("a5");
        test("14B2AH");
        test("f075ff011");
    }
}
