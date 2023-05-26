use typst_macros::Cast;

use super::{Color, RgbaColor};

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Ord, PartialOrd, Hash)]
#[derive(Cast)]
pub enum ColorSpace {
    #[default]
    Oklab,
    Srgb,
}

struct OklabColor {
    lightness: f64,
    a: f64,
    b: f64,
    alpha: f64,
}

fn lerp(v0: f64, v1: f64, t: f64) -> f64 {
    t.mul_add(v1 - v0, v0)
}

fn mix_srgb(color1: RgbaColor, color2: RgbaColor, t: f64) -> RgbaColor {
    RgbaColor {
        r: lerp(color1.r as f64, color2.r as f64, t).round() as u8,
        g: lerp(color1.g as f64, color2.g as f64, t).round() as u8,
        b: lerp(color1.b as f64, color2.b as f64, t).round() as u8,
        a: lerp(color1.a as f64, color2.a as f64, t).round() as u8,
    }
}

fn mix_oklab(color1: OklabColor, color2: OklabColor, t: f64) -> OklabColor {
    OklabColor {
        lightness: lerp(color1.lightness, color2.lightness, t),
        a: lerp(color1.a, color2.a, t),
        b: lerp(color1.b, color2.b, t),
        alpha: lerp(color1.alpha, color2.alpha, t),
    }
}

// https://bottosson.github.io/posts/oklab

fn to_oklab(color: RgbaColor) -> OklabColor {
    let r = color.r as f64 / 255.0;
    let g = color.g as f64 / 255.0;
    let b = color.b as f64 / 255.0;
    let alpha = color.a as f64 / 255.0;

    let l = 0.4122214708 * r + 0.5363325363 * g + 0.0514459929 * b;
    let m = 0.2119034982 * r + 0.6806995451 * g + 0.1073969566 * b;
    let s = 0.0883024619 * r + 0.2817188376 * g + 0.6299787005 * b;

    let l_ = l.cbrt();
    let m_ = m.cbrt();
    let s_ = s.cbrt();

    OklabColor {
        lightness: 0.2104542553 * l_ + 0.7936177850 * m_ - 0.0040720468 * s_,
        a: 1.9779984951 * l_ - 2.4285922050 * m_ + 0.4505937099 * s_,
        b: 0.0259040371 * l_ + 0.7827717662 * m_ - 0.8086757660 * s_,
        alpha,
    }
}

fn from_oklab(color: OklabColor) -> RgbaColor {
    let l_ = color.lightness + 0.3963377774 * color.a + 0.2158037573 * color.b;
    let m_ = color.lightness - 0.1055613458 * color.a - 0.0638541728 * color.b;
    let s_ = color.lightness - 0.0894841775 * color.a - 1.2914855480 * color.b;

    let l = l_ * l_ * l_;
    let m = m_ * m_ * m_;
    let s = s_ * s_ * s_;

    let r = 4.0767416621 * l - 3.3077115913 * m + 0.2309699292 * s;
    let g = -1.2684380046 * l + 2.6097574011 * m - 0.3413193965 * s;
    let b = -0.0041960863 * l - 0.7034186147 * m + 1.7076147010 * s;

    RgbaColor {
        r: (r * 255.0).round() as u8,
        g: (g * 255.0).round() as u8,
        b: (b * 255.0).round() as u8,
        a: (color.alpha * 255.0).round() as u8,
    }
}

pub fn mix_rgba(
    color1: RgbaColor,
    color2: RgbaColor,
    t: f64,
    space: ColorSpace,
) -> RgbaColor {
    match space {
        ColorSpace::Oklab => from_oklab(mix_oklab(to_oklab(color1), to_oklab(color2), t)),
        ColorSpace::Srgb => mix_srgb(color1, color2, t),
    }
}

pub fn mix_color(color1: Color, color2: Color, t: f64, space: ColorSpace) -> Color {
    mix_rgba(color1.to_rgba(), color2.to_rgba(), t, space).into()
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_mix_srgb() {
        let red = Color::Rgba(RgbaColor::new(0xff, 0x00, 0x00, 0xff));
        let green = Color::Rgba(RgbaColor::new(0x00, 0xff, 0x00, 0xff));
        assert_eq!(
            mix_color(red, green, 0.5, ColorSpace::Srgb),
            Color::Rgba(RgbaColor::new(0x80, 0x80, 0, 0xff))
        );
        assert_eq!(mix_color(red, green, 0.0, ColorSpace::Srgb), red);
        assert_eq!(mix_color(red, green, 1.0, ColorSpace::Srgb), green);
    }

    #[test]
    fn test_mix_oklab() {
        let red = Color::Rgba(RgbaColor::new(0xff, 0x00, 0x00, 0xff));
        let green = Color::Rgba(RgbaColor::new(0x00, 0xff, 0x00, 0xff));
        assert_eq!(
            mix_color(red, green, 0.5, ColorSpace::Oklab),
            Color::Rgba(RgbaColor::new(0xa1, 0x64, 0x00, 0xff))
        );
        assert_eq!(mix_color(red, green, 0.0, ColorSpace::Oklab), red);
        assert_eq!(mix_color(red, green, 1.0, ColorSpace::Oklab), green);
    }
}
