use std::sync::Arc;

use pdf_writer::{
    types::DeviceNSubtype, writers, Dict, Filter, Finish, Name, PdfWriter, Ref,
};

use crate::{
    export::pdf::deflate,
    geom::{Color, ColorExt, ColorSpace},
};

use super::RefExt;

/// The color spaces present in the PDF document
#[derive(Default)]
pub struct ColorSpaces {
    oklab: Option<Ref>,
    srgb: Option<Ref>,
    d65_gray: Option<Ref>,
    hsv: Option<Ref>,
    hsl: Option<Ref>,
    use_linear_rgb: bool,
}

impl ColorSpaces {
    /// Get a reference to the oklab color space.
    pub fn oklab(&mut self, alloc: &mut Ref) -> Ref {
        *self.oklab.get_or_insert_with(|| alloc.bump())
    }

    /// Get a reference to the srgb color space.
    pub fn srgb(&mut self, alloc: &mut Ref) -> Ref {
        *self.srgb.get_or_insert_with(|| alloc.bump())
    }

    /// Get a reference to the gray color space.
    pub fn d65_gray(&mut self, alloc: &mut Ref) -> Ref {
        *self.d65_gray.get_or_insert_with(|| alloc.bump())
    }

    /// Get a reference to the hsv color space.
    pub fn hsv(&mut self, alloc: &mut Ref) -> Ref {
        *self.hsv.get_or_insert_with(|| alloc.bump())
    }

    /// Get a reference to the hsl color space.
    pub fn hsl(&mut self, alloc: &mut Ref) -> Ref {
        *self.hsl.get_or_insert_with(|| alloc.bump())
    }

    /// Mark linear RGB as used.
    pub fn linear_rgb(&mut self) {
        self.use_linear_rgb = true;
    }

    /// Write the color space on usage.
    pub fn write(
        &mut self,
        color_space: ColorSpace,
        writer: writers::ColorSpace,
        alloc: &mut Ref,
    ) {
        match color_space {
            ColorSpace::Oklab => {
                let mut oklab = writer.device_n([OKLAB_L, OKLAB_A, OKLAB_B]);
                self.write(ColorSpace::LinearRGB, oklab.alternate_color_space(), alloc);
                oklab.tint_ref(self.oklab(alloc));
                oklab.attrs().subtype(DeviceNSubtype::DeviceN);
                oklab.finish();
            }
            ColorSpace::Srgb => writer.icc_based(self.srgb(alloc)),
            ColorSpace::D65Gray => writer.icc_based(self.d65_gray(alloc)),
            ColorSpace::LinearRGB => {
                writer.cal_rgb(
                    [0.9505, 1.0, 1.0888],
                    None,
                    Some([1.0, 1.0, 1.0]),
                    Some([
                        0.4124, 0.2126, 0.0193, 0.3576, 0.715, 0.1192, 0.1805, 0.0722,
                        0.9505,
                    ]),
                );
            }
            ColorSpace::Hsl => {
                let mut hsl = writer.device_n([HSL_H, HSL_S, HSL_L]);
                self.write(ColorSpace::Srgb, hsl.alternate_color_space(), alloc);
                hsl.tint_ref(self.hsl(alloc));
                hsl.attrs().subtype(DeviceNSubtype::DeviceN);
                hsl.finish();
            }
            ColorSpace::Hsv => {
                let mut hsv = writer.device_n([HSV_H, HSV_S, HSV_V]);
                self.write(ColorSpace::Srgb, hsv.alternate_color_space(), alloc);
                hsv.tint_ref(self.hsv(alloc));
                hsv.attrs().subtype(DeviceNSubtype::DeviceN);
                hsv.finish();
            }
            ColorSpace::Cmyk => writer.device_cmyk(),
        }
    }

    /// Write the necessary color spaces to the PDF file.
    pub fn write_functions(&self, writer: &mut PdfWriter) {
        // Write the Oklab function & color space
        if let Some(oklab) = self.oklab {
            let code = oklab_function();
            let mut color_function = writer.post_script_function(oklab, &code);
            color_function.filter(Filter::FlateDecode);
            color_function.domain([0.0, 1.0, 0.0, 1.0, 0.0, 1.0]);
            color_function.range([0.0, 1.0, 0.0, 1.0, 0.0, 1.0]);
            color_function.finish();
        }

        // Write the HSV function & color space
        if let Some(hsv) = self.hsv {
            let code = hsv_function();
            let mut color_function = writer.post_script_function(hsv, &code);
            color_function.filter(Filter::FlateDecode);
            color_function.domain([0.0, 1.0, 0.0, 1.0, 0.0, 1.0]);
            color_function.range([0.0, 1.0, 0.0, 1.0, 0.0, 1.0]);
            color_function.finish();
        }

        // Write the HSL function & color space
        if let Some(hsl) = self.hsl {
            let code = hsl_function();
            let mut color_function = writer.post_script_function(hsl, &code);
            color_function.filter(Filter::FlateDecode);
            color_function.domain([0.0, 1.0, 0.0, 1.0, 0.0, 1.0]);
            color_function.range([0.0, 1.0, 0.0, 1.0, 0.0, 1.0]);
            color_function.finish();
        }

        // Write the sRGB color space
        if let Some(srgb) = self.srgb {
            let profile = srgb_icc();
            let mut icc_profile = writer.icc_profile(srgb, &profile);
            icc_profile.alternate().srgb();
            icc_profile.n(3);
            icc_profile.range([0.0, 1.0, 0.0, 1.0, 0.0, 1.0]);
            icc_profile.finish();
        }

        // Write the gray color space
        if let Some(gray) = self.d65_gray {
            let profile = gray_icc();
            let mut icc_profile = writer.icc_profile(gray, &profile);
            icc_profile.alternate().d65_gray();
            icc_profile.n(1);
            icc_profile.range([0.0, 1.0]);
            icc_profile.finish();
        }
    }

    // Write the color spaces to the PDF file.
    pub fn write_color_spaces(&mut self, mut spaces: Dict, alloc: &mut Ref) {
        if self.oklab.is_some() {
            self.write(ColorSpace::Oklab, spaces.insert(OKLAB).start(), alloc);
        }

        if self.srgb.is_some() {
            self.write(ColorSpace::Srgb, spaces.insert(SRGB).start(), alloc);
        }

        if self.d65_gray.is_some() {
            self.write(ColorSpace::D65Gray, spaces.insert(D65_GRAY).start(), alloc);
        }

        if self.hsv.is_some() {
            self.write(ColorSpace::Hsv, spaces.insert(HSV).start(), alloc);
        }

        if self.hsl.is_some() {
            self.write(ColorSpace::Hsl, spaces.insert(HSL).start(), alloc);
        }

        if self.use_linear_rgb {
            self.write(ColorSpace::LinearRGB, spaces.insert(LINEAR_SRGB).start(), alloc);
        }

        spaces.finish();
    }
}

/// The ICC profile for sRGB colors
const SRGB_ICC: &[u8] = include_bytes!("../../../../../assets/icc/sRGB-v4.icc");

/// The ICC profile for gray colors
const GRAY_ICC: &[u8] = include_bytes!("../../../../../assets/icc/sGrey-v4.icc");

/// The PostScript function for Oklab colors
const OKLAB_SOURCE: &str = include_str!("../../../../../assets/post-script/oklab.ps");

/// The PostScript function for HSL colors
const HSL_SOURCE: &str = include_str!("../../../../../assets/post-script/hsl.ps");

/// The PostScript function for HSV colors
const HSV_SOURCE: &str = include_str!("../../../../../assets/post-script/hsv.ps");

/// The name of the sRGB color space
pub const SRGB: Name<'static> = Name(b"srgb");

/// The name of the gray color space
pub const D65_GRAY: Name<'static> = Name(b"d65gray");

/// The name of the OkLab color space
pub const OKLAB: Name<'static> = Name(b"oklab");

/// The name of the HSV color space
pub const HSV: Name<'static> = Name(b"hsv");

/// The name of the HSL color space
pub const HSL: Name<'static> = Name(b"hsl");

/// The name of the linear RGB color space
pub const LINEAR_SRGB: Name<'static> = Name(b"linearrgb");

/// The name of the "lightness" component of the OkLab color space
const OKLAB_L: Name<'static> = Name(b"OkLabL");

/// The name of the "a" component of the OkLab color space
const OKLAB_A: Name<'static> = Name(b"OkLabA");

/// The name of the "b" component of the OkLab color space
const OKLAB_B: Name<'static> = Name(b"OkLabB");

/// The name of the "hue" component of the HSV color space
const HSV_H: Name<'static> = Name(b"HsvH");

/// The name of the "saturation" component of the HSV color space
const HSV_S: Name<'static> = Name(b"HsvS");

/// The name of the "value" component of the HSV color space
const HSV_V: Name<'static> = Name(b"HsvV");

/// The name of the "hue" component of the HSL color space
const HSL_H: Name<'static> = Name(b"HsvH");

/// The name of the "saturation" component of the HSL color space
const HSL_S: Name<'static> = Name(b"HsvS");

/// The name of the "lightness" component of the HSL color space
const HSL_L: Name<'static> = Name(b"HsvL");

/// Deflated sRGB ICC profile
#[comemo::memoize]
fn srgb_icc() -> Arc<Vec<u8>> {
    Arc::new(deflate(SRGB_ICC))
}

/// Deflated gray ICC profile
#[comemo::memoize]
fn gray_icc() -> Arc<Vec<u8>> {
    Arc::new(deflate(GRAY_ICC))
}

/// Deflated Oklab PostScript function
#[comemo::memoize]
fn oklab_function() -> Arc<Vec<u8>> {
    let code = minify(OKLAB_SOURCE);
    Arc::new(deflate(code.as_bytes()))
}

/// Deflated HSV PostScript function
#[comemo::memoize]
fn hsv_function() -> Arc<Vec<u8>> {
    let code = minify(HSV_SOURCE);
    Arc::new(deflate(code.as_bytes()))
}

/// Deflated HSL PostScript function
#[comemo::memoize]
fn hsl_function() -> Arc<Vec<u8>> {
    let code = minify(HSL_SOURCE);
    Arc::new(deflate(code.as_bytes()))
}

/// This function removes comments, line spaces and carriage returns from a
/// PostScript program. This is necessary to optimize the size of the PDF file.
fn minify(source: &str) -> String {
    let mut buf = String::with_capacity(source.len());
    let mut in_comment = false;
    let mut in_line_space = false;
    for c in source.chars() {
        match c {
            '%' => in_comment = true,
            '\n' => {
                if !in_line_space {
                    buf.push(' ');
                }

                in_comment = false;
                in_line_space = true;
            }
            '\r' => {}
            _ if in_comment => {}
            _ if in_line_space => {
                if !c.is_whitespace() {
                    in_line_space = false;
                    buf.push(c);
                }
            }
            _ => {
                buf.push(c);
            }
        }
    }
    buf
}

pub trait ColorPdfEncode {
    /// Encodes the color into four f32s, which can be used in a PDF file.
    /// Ensures that the values are in the range [0.0, 1.0].
    fn encode(&self, color: Color) -> [f32; 4];
}

impl ColorPdfEncode for ColorSpace {
    fn encode(&self, color: Color) -> [f32; 4] {
        match self {
            ColorSpace::Oklab => {
                let [l, a, b, _] = color.to_oklab().to_vec4();
                [
                    l as f32,
                    (a as f32 + 0.4).clamp(0.0, 1.0),
                    (b as f32 + 0.4).clamp(0.0, 1.0),
                    0.0,
                ]
            }
            ColorSpace::Srgb => {
                let [r, g, b, _] = color.to_rgba().to_vec4();

                [r as f32, g as f32, b as f32, 0.0]
            }
            ColorSpace::D65Gray => {
                let [l, _, _, _] = color.to_luma().to_vec4();

                [l as f32, 0.0, 0.0, 0.0]
            }
            ColorSpace::LinearRGB => {
                let [r, g, b, _] = color.to_linear_rgb().to_vec4();

                [r as f32, g as f32, b as f32, 0.0]
            }
            ColorSpace::Hsl => {
                let [h, s, l, _] = color.to_hsl().to_vec4();

                [h.to_degrees() as f32 / 360.0, s as f32, l as f32, 0.0]
            }
            ColorSpace::Hsv => {
                let [h, s, v, _] = color.to_hsv().to_vec4();

                [h.to_degrees() as f32 / 360.0, s as f32, v as f32, 0.0]
            }
            ColorSpace::Cmyk => {
                let [c, m, y, k] = color.to_cmyk().to_vec4();

                [c as f32, m as f32, y as f32, k as f32]
            }
        }
    }
}
