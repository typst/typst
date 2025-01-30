use std::collections::HashMap;
use std::io::Cursor;

use ecow::eco_format;
use image::{DynamicImage, GenericImageView, Rgba};
use pdf_writer::{Chunk, Filter, Finish, Ref};
use typst_library::diag::{At, SourceResult, StrResult};
use typst_library::foundations::Smart;
use typst_library::visualize::{
    ColorSpace, ExchangeFormat, Image, ImageKind, ImageScaling, RasterFormat, SvgImage,
};
use typst_utils::Deferred;

use crate::{color, deflate, PdfChunk, WithGlobalRefs};

/// Embed all used images into the PDF.
#[typst_macros::time(name = "write images")]
pub fn write_images(
    context: &WithGlobalRefs,
) -> SourceResult<(PdfChunk, HashMap<Image, Ref>)> {
    let mut chunk = PdfChunk::new();
    let mut out = HashMap::new();
    context.resources.traverse(&mut |resources| {
        for (i, image) in resources.images.items().enumerate() {
            if out.contains_key(image) {
                continue;
            }

            let (handle, span) = resources.deferred_images.get(&i).unwrap();
            let encoded = handle.wait().as_ref().map_err(Clone::clone).at(*span)?;

            match encoded {
                EncodedImage::Raster {
                    data,
                    filter,
                    color_space,
                    bits_per_component,
                    width,
                    height,
                    compressed_icc,
                    alpha,
                    interpolate,
                } => {
                    let image_ref = chunk.alloc();
                    out.insert(image.clone(), image_ref);

                    let mut image = chunk.chunk.image_xobject(image_ref, data);
                    image.filter(*filter);
                    image.width(*width as i32);
                    image.height(*height as i32);
                    image.bits_per_component(i32::from(*bits_per_component));
                    image.interpolate(*interpolate);

                    let mut icc_ref = None;
                    let space = image.color_space();
                    if compressed_icc.is_some() {
                        let id = chunk.alloc.bump();
                        space.icc_based(id);
                        icc_ref = Some(id);
                    } else {
                        color::write(
                            *color_space,
                            space,
                            &context.globals.color_functions,
                        );
                    }

                    // Add a second gray-scale image containing the alpha values if
                    // this image has an alpha channel.
                    if let Some((alpha_data, alpha_filter)) = alpha {
                        let mask_ref = chunk.alloc.bump();
                        image.s_mask(mask_ref);
                        image.finish();

                        let mut mask = chunk.image_xobject(mask_ref, alpha_data);
                        mask.filter(*alpha_filter);
                        mask.width(*width as i32);
                        mask.height(*height as i32);
                        mask.color_space().device_gray();
                        mask.bits_per_component(i32::from(*bits_per_component));
                        mask.interpolate(*interpolate);
                    } else {
                        image.finish();
                    }

                    if let (Some(compressed_icc), Some(icc_ref)) =
                        (compressed_icc, icc_ref)
                    {
                        let mut stream = chunk.icc_profile(icc_ref, compressed_icc);
                        stream.filter(Filter::FlateDecode);
                        match color_space {
                            ColorSpace::Srgb => {
                                stream.n(3);
                                stream.alternate().srgb();
                            }
                            ColorSpace::D65Gray => {
                                stream.n(1);
                                stream.alternate().d65_gray();
                            }
                            _ => unimplemented!(),
                        }
                    }
                }
                EncodedImage::Svg(svg_chunk, id) => {
                    let mut map = HashMap::new();
                    svg_chunk.renumber_into(&mut chunk.chunk, |old| {
                        *map.entry(old).or_insert_with(|| chunk.alloc.bump())
                    });
                    out.insert(image.clone(), map[id]);
                }
            }
        }

        Ok(())
    })?;

    Ok((chunk, out))
}

/// Creates a new PDF image from the given image.
///
/// Also starts the deferred encoding of the image.
#[comemo::memoize]
pub fn deferred_image(
    image: Image,
    pdfa: bool,
) -> (Deferred<StrResult<EncodedImage>>, Option<ColorSpace>) {
    let color_space = match image.kind() {
        ImageKind::Raster(raster) if raster.icc().is_none() => {
            Some(to_color_space(raster.dynamic().color()))
        }
        _ => None,
    };

    // PDF/A does not appear to allow interpolation.
    // See https://github.com/typst/typst/issues/2942.
    let interpolate = !pdfa && image.scaling() == Smart::Custom(ImageScaling::Smooth);

    let deferred = Deferred::new(move || match image.kind() {
        ImageKind::Raster(raster) => {
            let format = if raster.format() == RasterFormat::Exchange(ExchangeFormat::Jpg)
            {
                EncodeFormat::DctDecode
            } else {
                EncodeFormat::Flate
            };
            Ok(encode_raster_image(raster.dynamic(), raster.icc(), format, interpolate))
        }
        ImageKind::Svg(svg) => {
            let (chunk, id) = encode_svg(svg, pdfa)
                .map_err(|err| eco_format!("failed to convert SVG to PDF: {err}"))?;
            Ok(EncodedImage::Svg(chunk, id))
        }
    });

    (deferred, color_space)
}

/// Encode an image with a suitable filter.
#[typst_macros::time(name = "encode raster image")]
fn encode_raster_image(
    image: &DynamicImage,
    icc_profile: Option<&[u8]>,
    format: EncodeFormat,
    interpolate: bool,
) -> EncodedImage {
    let color_space = to_color_space(image.color());

    let (filter, data, bits_per_component) = match format {
        EncodeFormat::DctDecode => {
            let mut data = Cursor::new(vec![]);
            image.write_to(&mut data, image::ImageFormat::Jpeg).unwrap();
            (Filter::DctDecode, data.into_inner(), 8)
        }
        EncodeFormat::Flate => {
            // TODO: Encode flate streams with PNG-predictor?
            let (data, bits_per_component) = match (image, color_space) {
                (DynamicImage::ImageRgb8(rgb), _) => (deflate(rgb.as_raw()), 8),
                // Grayscale image
                (DynamicImage::ImageLuma8(luma), _) => (deflate(luma.as_raw()), 8),
                (_, ColorSpace::D65Gray) => (deflate(image.to_luma8().as_raw()), 8),
                // Anything else
                _ => (deflate(image.to_rgb8().as_raw()), 8),
            };
            (Filter::FlateDecode, data, bits_per_component)
        }
    };

    let compressed_icc = icc_profile.map(deflate);
    let alpha = image.color().has_alpha().then(|| encode_alpha(image));

    EncodedImage::Raster {
        data,
        filter,
        color_space,
        bits_per_component,
        width: image.width(),
        height: image.height(),
        compressed_icc,
        alpha,
        interpolate,
    }
}

/// Encode an image's alpha channel if present.
#[typst_macros::time(name = "encode alpha")]
fn encode_alpha(image: &DynamicImage) -> (Vec<u8>, Filter) {
    let pixels: Vec<_> = image.pixels().map(|(_, _, Rgba([_, _, _, a]))| a).collect();
    (deflate(&pixels), Filter::FlateDecode)
}

/// Encode an SVG into a chunk of PDF objects.
#[typst_macros::time(name = "encode svg")]
fn encode_svg(
    svg: &SvgImage,
    pdfa: bool,
) -> Result<(Chunk, Ref), svg2pdf::ConversionError> {
    svg2pdf::to_chunk(
        svg.tree(),
        svg2pdf::ConversionOptions {
            pdfa,
            embed_text: !svg.flatten_text(),
            ..Default::default()
        },
    )
}

/// A pre-encoded image.
pub enum EncodedImage {
    /// A pre-encoded rasterized image.
    Raster {
        /// The raw, pre-deflated image data.
        data: Vec<u8>,
        /// The filter to use for the image.
        filter: Filter,
        /// Which color space this image is encoded in.
        color_space: ColorSpace,
        /// How many bits of each color component are stored.
        bits_per_component: u8,
        /// The image's width.
        width: u32,
        /// The image's height.
        height: u32,
        /// The image's ICC profile, deflated, if any.
        compressed_icc: Option<Vec<u8>>,
        /// The alpha channel of the image, pre-deflated, if any.
        alpha: Option<(Vec<u8>, Filter)>,
        /// Whether image interpolation should be enabled.
        interpolate: bool,
    },
    /// A vector graphic.
    ///
    /// The chunk is the SVG converted to PDF objects.
    Svg(Chunk, Ref),
}

/// How the raster image should be encoded.
enum EncodeFormat {
    DctDecode,
    Flate,
}

/// Matches an [`image::ColorType`] to [`ColorSpace`].
fn to_color_space(color: image::ColorType) -> ColorSpace {
    use image::ColorType::*;
    match color {
        L8 | La8 | L16 | La16 => ColorSpace::D65Gray,
        Rgb8 | Rgba8 | Rgb16 | Rgba16 | Rgb32F | Rgba32F => ColorSpace::Srgb,
        _ => unimplemented!(),
    }
}
