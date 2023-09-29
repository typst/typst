use std::sync::Arc;

use pdf_writer::types::DeviceNSubtype;
use pdf_writer::{writers, Dict, Filter, Name, PdfWriter, Ref};

use super::page::PageContext;
use super::RefExt;
use crate::export::pdf::deflate;
use crate::geom::{Color, ColorSpace, Paint};

// The names of the color spaces.
pub const SRGB: Name<'static> = Name(b"srgb");
pub const D65_GRAY: Name<'static> = Name(b"d65gray");
pub const OKLAB: Name<'static> = Name(b"oklab");
pub const HSV: Name<'static> = Name(b"hsv");
pub const HSL: Name<'static> = Name(b"hsl");
pub const LINEAR_SRGB: Name<'static> = Name(b"linearrgb");

// The names of the color components.
const OKLAB_L: Name<'static> = Name(b"L");
const OKLAB_A: Name<'static> = Name(b"A");
const OKLAB_B: Name<'static> = Name(b"B");
const HSV_H: Name<'static> = Name(b"H");
const HSV_S: Name<'static> = Name(b"S");
const HSV_V: Name<'static> = Name(b"V");
const HSL_H: Name<'static> = Name(b"H");
const HSL_S: Name<'static> = Name(b"S");
const HSL_L: Name<'static> = Name(b"L");

// The ICC profiles.
const SRGB_ICC: &[u8] = include_bytes!("./icc/sRGB-v4.icc");
const GRAY_ICC: &[u8] = include_bytes!("./icc/sGrey-v4.icc");

// The PostScript functions for color spaces.
const OKLAB_SOURCE: &str = include_str!("./postscript/oklab.ps");
const HSL_SOURCE: &str = include_str!("./postscript/hsl.ps");
const HSV_SOURCE: &str = include_str!("./postscript/hsv.ps");

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
    ///
    /// # Warning
    /// The A and B components of the color must be offset by +0.4 before being
    /// encoded into the PDF file.
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
    ///
    /// # Warning
    /// The Hue component of the color must be in degrees and must be divided
    /// by 360.0 before being encoded into the PDF file.
    pub fn hsv(&mut self, alloc: &mut Ref) -> Ref {
        *self.hsv.get_or_insert_with(|| alloc.bump())
    }

    /// Get a reference to the hsl color space.
    ///
    /// # Warning
    /// The Hue component of the color must be in degrees and must be divided
    /// by 360.0 before being encoded into the PDF file.
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
                self.write(ColorSpace::LinearRgb, oklab.alternate_color_space(), alloc);
                oklab.tint_ref(self.oklab(alloc));
                oklab.attrs().subtype(DeviceNSubtype::DeviceN);
            }
            ColorSpace::Srgb => writer.icc_based(self.srgb(alloc)),
            ColorSpace::D65Gray => writer.icc_based(self.d65_gray(alloc)),
            ColorSpace::LinearRgb => {
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
            }
            ColorSpace::Hsv => {
                let mut hsv = writer.device_n([HSV_H, HSV_S, HSV_V]);
                self.write(ColorSpace::Srgb, hsv.alternate_color_space(), alloc);
                hsv.tint_ref(self.hsv(alloc));
                hsv.attrs().subtype(DeviceNSubtype::DeviceN);
            }
            ColorSpace::Cmyk => writer.device_cmyk(),
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
            self.write(ColorSpace::LinearRgb, spaces.insert(LINEAR_SRGB).start(), alloc);
        }
    }

    /// Write the necessary color spaces functions and ICC profiles to the
    /// PDF file.
    pub fn write_functions(&self, writer: &mut PdfWriter) {
        // Write the Oklab function & color space
        if let Some(oklab) = self.oklab {
            let code = oklab_function();
            writer
                .post_script_function(oklab, &code)
                .domain([0.0, 1.0, 0.0, 1.0, 0.0, 1.0])
                .range([0.0, 1.0, 0.0, 1.0, 0.0, 1.0])
                .filter(Filter::FlateDecode);
        }

        // Write the HSV function & color space
        if let Some(hsv) = self.hsv {
            let code = hsv_function();
            writer
                .post_script_function(hsv, &code)
                .domain([0.0, 1.0, 0.0, 1.0, 0.0, 1.0])
                .range([0.0, 1.0, 0.0, 1.0, 0.0, 1.0])
                .filter(Filter::FlateDecode);
        }

        // Write the HSL function & color space
        if let Some(hsl) = self.hsl {
            let code = hsl_function();
            writer
                .post_script_function(hsl, &code)
                .domain([0.0, 1.0, 0.0, 1.0, 0.0, 1.0])
                .range([0.0, 1.0, 0.0, 1.0, 0.0, 1.0])
                .filter(Filter::FlateDecode);
        }

        // Write the sRGB color space
        if let Some(srgb) = self.srgb {
            let profile = srgb_icc();
            writer
                .icc_profile(srgb, &profile)
                .n(3)
                .range([0.0, 1.0, 0.0, 1.0, 0.0, 1.0]);
        }

        // Write the gray color space
        if let Some(gray) = self.d65_gray {
            let profile = gray_icc();
            writer.icc_profile(gray, &profile).n(1).range([0.0, 1.0]);
        }
    }
}

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
    let mut s = unscanny::Scanner::new(source);
    while let Some(c) = s.eat() {
        match c {
            '%' => {
                s.eat_until('\n');
            }
            c if c.is_whitespace() => {
                s.eat_whitespace();
                if buf.ends_with(|c: char| !c.is_whitespace()) {
                    buf.push(' ');
                }
            }
            _ => buf.push(c),
        }
    }
    buf
}

/// Encodes the color into four f32s, which can be used in a PDF file.
/// Ensures that the values are in the range [0.0, 1.0].
///
/// # Why?
/// - Oklab: The a and b components are in the range [-0.4, 0.4] and the PDF
///   specifies (and some readers enforce) that all color values be in the range
///   [0.0, 1.0]. This means that the PostScript function and the encoded color
///   must be offset by 0.4.
/// - HSV/HSL: The hue component is in the range [0.0, 360.0] and the PDF format
///   specifies that it must be in the range [0.0, 1.0]. This means that the
///   PostScript function and the encoded color must be divided by 360.0.
pub trait ColorEncode {
    /// Performs the color to PDF f32 array conversion.
    fn encode(&self, color: Color) -> [f32; 4];
}

impl ColorEncode for ColorSpace {
    fn encode(&self, color: Color) -> [f32; 4] {
        match self {
            ColorSpace::Oklab => {
                let [l, a, b, alpha] = color.to_oklab().to_vec4();
                [l, (a + 0.4).clamp(0.0, 1.0), (b + 0.4).clamp(0.0, 1.0), alpha]
            }
            ColorSpace::Hsl => {
                let [h, s, l, _] = color.to_hsl().to_vec4();
                [h / 360.0, s, l, 0.0]
            }
            ColorSpace::Hsv => {
                let [h, s, v, _] = color.to_hsv().to_vec4();
                [h / 360.0, s, v, 0.0]
            }
            _ => color.to_vec4(),
        }
    }
}

/// Encodes a paint into either a fill or stroke color.
pub trait PaintEncode {
    /// Set the paint as the fill color.
    fn set_as_fill(&self, page_context: &mut PageContext);

    /// Set the paint as the stroke color.
    fn set_as_stroke(&self, page_context: &mut PageContext);
}

impl PaintEncode for Paint {
    fn set_as_fill(&self, ctx: &mut PageContext) {
        let Paint::Solid(color) = self;
        match color {
            Color::Luma(_) => {
                ctx.parent.colors.d65_gray(&mut ctx.parent.alloc);
                ctx.set_fill_color_space(D65_GRAY);

                let [l, _, _, _] = ColorSpace::D65Gray.encode(*color);
                ctx.content.set_fill_color([l]);
            }
            Color::Oklab(_) => {
                ctx.parent.colors.oklab(&mut ctx.parent.alloc);
                ctx.set_fill_color_space(OKLAB);

                let [l, a, b, _] = ColorSpace::Oklab.encode(*color);
                ctx.content.set_fill_color([l, a, b]);
            }
            Color::LinearRgb(_) => {
                ctx.parent.colors.linear_rgb();
                ctx.set_fill_color_space(LINEAR_SRGB);

                let [r, g, b, _] = ColorSpace::LinearRgb.encode(*color);
                ctx.content.set_fill_color([r, g, b]);
            }
            Color::Rgba(_) => {
                ctx.parent.colors.srgb(&mut ctx.parent.alloc);
                ctx.set_fill_color_space(SRGB);

                let [r, g, b, _] = ColorSpace::Srgb.encode(*color);
                ctx.content.set_fill_color([r, g, b]);
            }
            Color::Cmyk(_) => {
                ctx.reset_fill_color_space();

                let [c, m, y, k] = ColorSpace::Cmyk.encode(*color);
                ctx.content.set_fill_cmyk(c, m, y, k);
            }
            Color::Hsl(_) => {
                ctx.parent.colors.hsl(&mut ctx.parent.alloc);
                ctx.set_fill_color_space(HSL);

                let [h, s, l, _] = ColorSpace::Hsl.encode(*color);
                ctx.content.set_fill_color([h, s, l]);
            }
            Color::Hsv(_) => {
                ctx.parent.colors.hsv(&mut ctx.parent.alloc);
                ctx.set_fill_color_space(HSV);

                let [h, s, v, _] = ColorSpace::Hsv.encode(*color);
                ctx.content.set_fill_color([h, s, v]);
            }
        }
    }

    fn set_as_stroke(&self, ctx: &mut PageContext) {
        let Paint::Solid(color) = self;
        match color {
            Color::Luma(_) => {
                ctx.parent.colors.d65_gray(&mut ctx.parent.alloc);
                ctx.set_stroke_color_space(D65_GRAY);

                let [l, _, _, _] = ColorSpace::D65Gray.encode(*color);
                ctx.content.set_stroke_color([l]);
            }
            Color::Oklab(_) => {
                ctx.parent.colors.oklab(&mut ctx.parent.alloc);
                ctx.set_stroke_color_space(OKLAB);

                let [l, a, b, _] = ColorSpace::Oklab.encode(*color);
                ctx.content.set_stroke_color([l, a, b]);
            }
            Color::LinearRgb(_) => {
                ctx.parent.colors.linear_rgb();
                ctx.set_stroke_color_space(LINEAR_SRGB);

                let [r, g, b, _] = ColorSpace::LinearRgb.encode(*color);
                ctx.content.set_stroke_color([r, g, b]);
            }
            Color::Rgba(_) => {
                ctx.parent.colors.srgb(&mut ctx.parent.alloc);
                ctx.set_stroke_color_space(SRGB);

                let [r, g, b, _] = ColorSpace::Srgb.encode(*color);
                ctx.content.set_stroke_color([r, g, b]);
            }
            Color::Cmyk(_) => {
                ctx.reset_stroke_color_space();

                let [c, m, y, k] = ColorSpace::Cmyk.encode(*color);
                ctx.content.set_stroke_cmyk(c, m, y, k);
            }
            Color::Hsl(_) => {
                ctx.parent.colors.hsl(&mut ctx.parent.alloc);
                ctx.set_stroke_color_space(HSL);

                let [h, s, l, _] = ColorSpace::Hsl.encode(*color);
                ctx.content.set_stroke_color([h, s, l]);
            }
            Color::Hsv(_) => {
                ctx.parent.colors.hsv(&mut ctx.parent.alloc);
                ctx.set_stroke_color_space(HSV);

                let [h, s, v, _] = ColorSpace::Hsv.encode(*color);
                ctx.content.set_stroke_color([h, s, v]);
            }
        }
    }
}
