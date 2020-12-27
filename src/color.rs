//! Color handling.

use std::fmt::{self, Debug, Formatter};
use std::str::FromStr;

/// A color in a dynamic format.
#[derive(Copy, Clone, Eq, PartialEq)]
pub enum Color {
    /// An 8-bit RGBA color: `#423abaff`.
    Rgba(RgbaColor),
}

impl Debug for Color {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        match self {
            Self::Rgba(c) => c.fmt(f),
        }
    }
}

/// An 8-bit RGBA color: `#423abaff`.
#[derive(Copy, Clone, Eq, PartialEq)]
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
    /// Constructs a new RGBA color.
    pub fn new(r: u8, g: u8, b: u8, a: u8) -> Self {
        Self { r, g, b, a }
    }
}

impl FromStr for RgbaColor {
    type Err = ParseRgbaError;

    /// Constructs a new color from a hex string like `7a03c2`. Do not specify a
    /// leading `#`.
    fn from_str(hex_str: &str) -> Result<Self, Self::Err> {
        if !hex_str.is_ascii() {
            return Err(ParseRgbaError);
        }

        let len = hex_str.len();
        let long = len == 6 || len == 8;
        let short = len == 3 || len == 4;
        let alpha = len == 4 || len == 8;

        if !long && !short {
            return Err(ParseRgbaError);
        }

        let mut values: [u8; 4] = [255; 4];

        for elem in if alpha { 0 .. 4 } else { 0 .. 3 } {
            let item_len = if long { 2 } else { 1 };
            let pos = elem * item_len;

            let item = &hex_str[pos .. (pos + item_len)];
            values[elem] = u8::from_str_radix(item, 16).map_err(|_| ParseRgbaError)?;

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
pub struct ParseRgbaError;

impl std::error::Error for ParseRgbaError {}

impl fmt::Display for ParseRgbaError {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        f.pad("invalid color")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_color_strings() {
        fn test(hex: &str, r: u8, g: u8, b: u8, a: u8) {
            assert_eq!(RgbaColor::from_str(hex), Ok(RgbaColor::new(r, g, b, a)));
        }

        test("f61243ff", 0xf6, 0x12, 0x43, 0xff);
        test("b3d8b3", 0xb3, 0xd8, 0xb3, 0xff);
        test("fCd2a9AD", 0xfc, 0xd2, 0xa9, 0xad);
        test("233", 0x22, 0x33, 0x33, 0xff);
        test("111b", 0x11, 0x11, 0x11, 0xbb);
    }
}
