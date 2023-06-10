use typst_macros::{cast, Cast};

use super::*;
use crate::eval::Array;
use oklab::{Oklab, RGB};

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Ord, PartialOrd, Hash)]
#[derive(Cast)]
pub enum ColorSpace {
    #[default]
    Oklab,
    Srgb,
}

pub struct WeightedColor(Color, f32);

cast! {
    WeightedColor,
    v: Color => Self(v, 1.0),
    v: Array => {
        let mut iter = v.into_iter();
        match (iter.next(), iter.next(), iter.next()) {
            (Some(c), Some(w), None) => {
                let weight = match w {
                    Value::Int(n) => n as f32,
                    Value::Float(n) => n as f32,
                    Value::Ratio(n) => n.get() as f32,
                    _ => Err("weights must be integer, float or ratio")?,
                };
                Self(c.cast()?, weight)
            }
            _ => Err("expected a color or color-weight pair")?,
        }
    }
}

fn lerp(v0: f32, v1: f32, t: f32) -> f32 {
    t.mul_add(v1 - v0, v0)
}

fn lerp4(v0: [f32; 4], v1: [f32; 4], t: f32) -> [f32; 4] {
    [
        lerp(v0[0], v1[0], t),
        lerp(v0[1], v1[1], t),
        lerp(v0[2], v1[2], t),
        lerp(v0[3], v1[3], t),
    ]
}

fn color_to_vec4(color: Color, space: ColorSpace) -> [f32; 4] {
    match space {
        ColorSpace::Oklab => {
            let RgbaColor { r, g, b, a } = color.to_rgba();
            let oklab = oklab::srgb_to_oklab(RGB { r, g, b });
            [oklab.l, oklab.a, oklab.b, a as f32 / 255.0]
        }
        ColorSpace::Srgb => {
            let RgbaColor { r, g, b, a } = color.to_rgba();
            [r as f32 / 255.0, g as f32 / 255.0, b as f32 / 255.0, a as f32 / 255.0]
        }
    }
}

fn vec4_to_color(vec: [f32; 4], space: ColorSpace) -> Color {
    match space {
        ColorSpace::Oklab => {
            let [l, a, b, alpha] = vec;
            let rgb = oklab::oklab_to_srgb(Oklab { l, a, b });
            Color::Rgba(RgbaColor {
                r: rgb.r,
                g: rgb.g,
                b: rgb.b,
                a: (alpha * 255.0).round() as u8,
            })
        }
        ColorSpace::Srgb => {
            let [r, g, b, a] = vec;
            Color::Rgba(RgbaColor {
                r: (r * 255.0).round() as u8,
                g: (g * 255.0).round() as u8,
                b: (b * 255.0).round() as u8,
                a: (a * 255.0).round() as u8,
            })
        }
    }
}

pub fn mix_colors<'a>(
    mut colors: impl Iterator<Item = &'a WeightedColor>,
    space: ColorSpace,
) -> Color {
    let first = colors.next().expect("no colors to mix");
    let mut mixed = color_to_vec4(first.0, space);
    let mut total_weight = first.1;
    for WeightedColor(color, weight) in colors {
        let vec = color_to_vec4(*color, space);
        total_weight += weight;
        mixed = lerp4(mixed, vec, weight / total_weight);
    }
    vec4_to_color(mixed, space)
}
