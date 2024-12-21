use std::sync::LazyLock;

use arrayvec::ArrayVec;
use pdf_writer::{writers, Chunk, Dict, Filter, Name, Ref};
use typst_library::diag::{bail, SourceResult};
use typst_library::visualize::{Color, ColorSpace, Paint};
use typst_syntax::Span;

use crate::{content, deflate, PdfChunk, PdfOptions, Renumber, WithResources};

// The names of the color spaces.
pub const SRGB: Name<'static> = Name(b"srgb");
pub const D65_GRAY: Name<'static> = Name(b"d65gray");
pub const LINEAR_SRGB: Name<'static> = Name(b"linearrgb");

// The ICC profiles.
static SRGB_ICC_DEFLATED: LazyLock<Vec<u8>> =
    LazyLock::new(|| deflate(typst_assets::icc::S_RGB_V4));
static GRAY_ICC_DEFLATED: LazyLock<Vec<u8>> =
    LazyLock::new(|| deflate(typst_assets::icc::S_GREY_V4));

/// The color spaces present in the PDF document
#[derive(Default)]
pub struct ColorSpaces {
    use_srgb: bool,
    use_d65_gray: bool,
    use_linear_rgb: bool,
}

impl ColorSpaces {
    /// Mark a color space as used.
    pub fn mark_as_used(&mut self, color_space: ColorSpace) {
        match color_space {
            ColorSpace::Oklch
            | ColorSpace::Oklab
            | ColorSpace::Hsl
            | ColorSpace::Hsv
            | ColorSpace::Srgb => {
                self.use_srgb = true;
            }
            ColorSpace::D65Gray => {
                self.use_d65_gray = true;
            }
            ColorSpace::LinearRgb => {
                self.use_linear_rgb = true;
            }
            ColorSpace::Cmyk => {}
        }
    }

    /// Write the color spaces to the PDF file.
    pub fn write_color_spaces(&self, mut spaces: Dict, refs: &ColorFunctionRefs) {
        if self.use_srgb {
            write(ColorSpace::Srgb, spaces.insert(SRGB).start(), refs);
        }

        if self.use_d65_gray {
            write(ColorSpace::D65Gray, spaces.insert(D65_GRAY).start(), refs);
        }

        if self.use_linear_rgb {
            write(ColorSpace::LinearRgb, spaces.insert(LINEAR_SRGB).start(), refs);
        }
    }

    /// Write the necessary color spaces functions and ICC profiles to the
    /// PDF file.
    pub fn write_functions(&self, chunk: &mut Chunk, refs: &ColorFunctionRefs) {
        // Write the sRGB color space.
        if let Some(id) = refs.srgb {
            chunk
                .icc_profile(id, &SRGB_ICC_DEFLATED)
                .n(3)
                .range([0.0, 1.0, 0.0, 1.0, 0.0, 1.0])
                .filter(Filter::FlateDecode);
        }

        // Write the gray color space.
        if let Some(id) = refs.d65_gray {
            chunk
                .icc_profile(id, &GRAY_ICC_DEFLATED)
                .n(1)
                .range([0.0, 1.0])
                .filter(Filter::FlateDecode);
        }
    }

    /// Merge two color space usage information together: a given color space is
    /// considered to be used if it is used on either side.
    pub fn merge(&mut self, other: &Self) {
        self.use_d65_gray |= other.use_d65_gray;
        self.use_linear_rgb |= other.use_linear_rgb;
        self.use_srgb |= other.use_srgb;
    }
}

/// Write the color space.
pub fn write(
    color_space: ColorSpace,
    writer: writers::ColorSpace,
    refs: &ColorFunctionRefs,
) {
    match color_space {
        ColorSpace::Srgb
        | ColorSpace::Oklab
        | ColorSpace::Hsl
        | ColorSpace::Hsv
        | ColorSpace::Oklch => writer.icc_based(refs.srgb.unwrap()),
        ColorSpace::D65Gray => writer.icc_based(refs.d65_gray.unwrap()),
        ColorSpace::LinearRgb => {
            writer.cal_rgb(
                [0.9505, 1.0, 1.0888],
                None,
                Some([1.0, 1.0, 1.0]),
                Some([
                    0.4124, 0.2126, 0.0193, 0.3576, 0.715, 0.1192, 0.1805, 0.0722, 0.9505,
                ]),
            );
        }
        ColorSpace::Cmyk => writer.device_cmyk(),
    }
}

/// Global references for color conversion functions.
///
/// These functions are only written once (at most, they are not written if not
/// needed) in the final document, and be shared by all color space
/// dictionaries.
pub struct ColorFunctionRefs {
    pub srgb: Option<Ref>,
    d65_gray: Option<Ref>,
}

impl Renumber for ColorFunctionRefs {
    fn renumber(&mut self, offset: i32) {
        if let Some(r) = &mut self.srgb {
            r.renumber(offset);
        }
        if let Some(r) = &mut self.d65_gray {
            r.renumber(offset);
        }
    }
}

/// Allocate all necessary [`ColorFunctionRefs`].
pub fn alloc_color_functions_refs(
    context: &WithResources,
) -> SourceResult<(PdfChunk, ColorFunctionRefs)> {
    let mut chunk = PdfChunk::new();
    let mut used_color_spaces = ColorSpaces::default();

    if context.options.standards.pdfa {
        used_color_spaces.mark_as_used(ColorSpace::Srgb);
    }

    context.resources.traverse(&mut |r| {
        used_color_spaces.merge(&r.colors);
        Ok(())
    })?;

    let refs = ColorFunctionRefs {
        srgb: if used_color_spaces.use_srgb { Some(chunk.alloc()) } else { None },
        d65_gray: if used_color_spaces.use_d65_gray { Some(chunk.alloc()) } else { None },
    };

    Ok((chunk, refs))
}

/// Encodes the color into four f32s, which can be used in a PDF file.
/// Ensures that the values are in the range [0.0, 1.0].
///
/// # Why?
/// - Oklab: The a and b components are in the range [-0.5, 0.5] and the PDF
///   specifies (and some readers enforce) that all color values be in the range
///   [0.0, 1.0]. This means that the PostScript function and the encoded color
///   must be offset by 0.5.
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
            ColorSpace::Oklab | ColorSpace::Oklch | ColorSpace::Hsl | ColorSpace::Hsv => {
                color.to_space(ColorSpace::Srgb).to_vec4()
            }
            _ => color.to_space(*self).to_vec4(),
        }
    }
}

/// Encodes a paint into either a fill or stroke color.
pub(super) trait PaintEncode {
    /// Set the paint as the fill color.
    fn set_as_fill(
        &self,
        ctx: &mut content::Builder,
        on_text: bool,
        transforms: content::Transforms,
    ) -> SourceResult<()>;

    /// Set the paint as the stroke color.
    fn set_as_stroke(
        &self,
        ctx: &mut content::Builder,
        on_text: bool,
        transforms: content::Transforms,
    ) -> SourceResult<()>;
}

impl PaintEncode for Paint {
    fn set_as_fill(
        &self,
        ctx: &mut content::Builder,
        on_text: bool,
        transforms: content::Transforms,
    ) -> SourceResult<()> {
        match self {
            Self::Solid(c) => c.set_as_fill(ctx, on_text, transforms),
            Self::Gradient(gradient) => gradient.set_as_fill(ctx, on_text, transforms),
            Self::Tiling(tiling) => tiling.set_as_fill(ctx, on_text, transforms),
        }
    }

    fn set_as_stroke(
        &self,
        ctx: &mut content::Builder,
        on_text: bool,
        transforms: content::Transforms,
    ) -> SourceResult<()> {
        match self {
            Self::Solid(c) => c.set_as_stroke(ctx, on_text, transforms),
            Self::Gradient(gradient) => gradient.set_as_stroke(ctx, on_text, transforms),
            Self::Tiling(tiling) => tiling.set_as_stroke(ctx, on_text, transforms),
        }
    }
}

impl PaintEncode for Color {
    fn set_as_fill(
        &self,
        ctx: &mut content::Builder,
        _: bool,
        _: content::Transforms,
    ) -> SourceResult<()> {
        match self {
            Color::Luma(_) => {
                ctx.resources.colors.mark_as_used(ColorSpace::D65Gray);
                ctx.set_fill_color_space(D65_GRAY);

                let [l, _, _, _] = ColorSpace::D65Gray.encode(*self);
                ctx.content.set_fill_color([l]);
            }
            Color::LinearRgb(_) => {
                ctx.resources.colors.mark_as_used(ColorSpace::LinearRgb);
                ctx.set_fill_color_space(LINEAR_SRGB);

                let [r, g, b, _] = ColorSpace::LinearRgb.encode(*self);
                ctx.content.set_fill_color([r, g, b]);
            }
            // Oklab & friends are encoded as RGB.
            Color::Rgb(_)
            | Color::Oklab(_)
            | Color::Oklch(_)
            | Color::Hsl(_)
            | Color::Hsv(_) => {
                ctx.resources.colors.mark_as_used(ColorSpace::Srgb);
                ctx.set_fill_color_space(SRGB);

                let [r, g, b, _] = ColorSpace::Srgb.encode(*self);
                ctx.content.set_fill_color([r, g, b]);
            }
            Color::Cmyk(_) => {
                check_cmyk_allowed(ctx.options)?;
                ctx.reset_fill_color_space();

                let [c, m, y, k] = ColorSpace::Cmyk.encode(*self);
                ctx.content.set_fill_cmyk(c, m, y, k);
            }
        }
        Ok(())
    }

    fn set_as_stroke(
        &self,
        ctx: &mut content::Builder,
        _: bool,
        _: content::Transforms,
    ) -> SourceResult<()> {
        match self {
            Color::Luma(_) => {
                ctx.resources.colors.mark_as_used(ColorSpace::D65Gray);
                ctx.set_stroke_color_space(D65_GRAY);

                let [l, _, _, _] = ColorSpace::D65Gray.encode(*self);
                ctx.content.set_stroke_color([l]);
            }
            Color::LinearRgb(_) => {
                ctx.resources.colors.mark_as_used(ColorSpace::LinearRgb);
                ctx.set_stroke_color_space(LINEAR_SRGB);

                let [r, g, b, _] = ColorSpace::LinearRgb.encode(*self);
                ctx.content.set_stroke_color([r, g, b]);
            }
            // Oklab & friends are encoded as RGB.
            Color::Rgb(_)
            | Color::Oklab(_)
            | Color::Oklch(_)
            | Color::Hsl(_)
            | Color::Hsv(_) => {
                ctx.resources.colors.mark_as_used(ColorSpace::Srgb);
                ctx.set_stroke_color_space(SRGB);

                let [r, g, b, _] = ColorSpace::Srgb.encode(*self);
                ctx.content.set_stroke_color([r, g, b]);
            }
            Color::Cmyk(_) => {
                check_cmyk_allowed(ctx.options)?;
                ctx.reset_stroke_color_space();

                let [c, m, y, k] = ColorSpace::Cmyk.encode(*self);
                ctx.content.set_stroke_cmyk(c, m, y, k);
            }
        }
        Ok(())
    }
}

/// Extra color space functions.
pub(super) trait ColorSpaceExt {
    /// Returns the range of the color space.
    fn range(self) -> &'static [f32];

    /// Converts a color to the color space.
    fn convert<U: QuantizedColor>(self, color: Color) -> ArrayVec<U, 4>;
}

impl ColorSpaceExt for ColorSpace {
    fn range(self) -> &'static [f32] {
        match self {
            ColorSpace::D65Gray => &[0.0, 1.0],
            ColorSpace::Oklab => &[0.0, 1.0, 0.0, 1.0, 0.0, 1.0],
            ColorSpace::Oklch => &[0.0, 1.0, 0.0, 1.0, 0.0, 1.0],
            ColorSpace::LinearRgb => &[0.0, 1.0, 0.0, 1.0, 0.0, 1.0],
            ColorSpace::Srgb => &[0.0, 1.0, 0.0, 1.0, 0.0, 1.0],
            ColorSpace::Cmyk => &[0.0, 1.0, 0.0, 1.0, 0.0, 1.0, 0.0, 1.0],
            ColorSpace::Hsl => &[0.0, 1.0, 0.0, 1.0, 0.0, 1.0],
            ColorSpace::Hsv => &[0.0, 1.0, 0.0, 1.0, 0.0, 1.0],
        }
    }

    fn convert<U: QuantizedColor>(self, color: Color) -> ArrayVec<U, 4> {
        let components = self.encode(color);

        self.range()
            .chunks(2)
            .zip(components)
            .map(|(range, component)| U::quantize(component, [range[0], range[1]]))
            .collect()
    }
}

/// Quantizes a color component to a specific type.
pub(super) trait QuantizedColor {
    fn quantize(color: f32, range: [f32; 2]) -> Self;
}

impl QuantizedColor for u16 {
    fn quantize(color: f32, [min, max]: [f32; 2]) -> Self {
        let value = (color - min) / (max - min);
        (value * Self::MAX as f32).round().clamp(0.0, Self::MAX as f32) as Self
    }
}

impl QuantizedColor for f32 {
    fn quantize(color: f32, [min, max]: [f32; 2]) -> Self {
        color.clamp(min, max)
    }
}

/// Fails with an error if PDF/A processing is enabled.
pub(super) fn check_cmyk_allowed(options: &PdfOptions) -> SourceResult<()> {
    if options.standards.pdfa {
        bail!(
            Span::detached(),
            "cmyk colors are not currently supported by PDF/A export"
        );
    }
    Ok(())
}
