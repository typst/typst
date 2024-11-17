use std::collections::HashMap;
use std::io::Cursor;

use ecow::eco_format;
use image::{DynamicImage, GenericImageView, Rgba};
use pdf_writer::{Chunk, Filter, Finish, Ref};
use typst_library::diag::{At, SourceResult, StrResult};
use typst_library::visualize::{
    ColorSpace, Image, ImageKind, RasterFormat, RasterImage, SvgImage,
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
                    has_color,
                    width,
                    height,
                    icc,
                    alpha,
                } => {
                    let image_ref = chunk.alloc();
                    out.insert(image.clone(), image_ref);

                    let mut image = chunk.chunk.image_xobject(image_ref, data);
                    image.filter(*filter);
                    image.width(*width as i32);
                    image.height(*height as i32);
                    image.bits_per_component(8);

                    let mut icc_ref = None;
                    let space = image.color_space();
                    if icc.is_some() {
                        let id = chunk.alloc.bump();
                        space.icc_based(id);
                        icc_ref = Some(id);
                    } else if *has_color {
                        color::write(
                            ColorSpace::Srgb,
                            space,
                            &context.globals.color_functions,
                        );
                    } else {
                        color::write(
                            ColorSpace::D65Gray,
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
                        mask.bits_per_component(8);
                    } else {
                        image.finish();
                    }

                    if let (Some(icc), Some(icc_ref)) = (icc, icc_ref) {
                        let mut stream = chunk.icc_profile(icc_ref, icc);
                        stream.filter(Filter::FlateDecode);
                        if *has_color {
                            stream.n(3);
                            stream.alternate().srgb();
                        } else {
                            stream.n(1);
                            stream.alternate().d65_gray();
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
            if raster.dynamic().color().channel_count() > 2 {
                Some(ColorSpace::Srgb)
            } else {
                Some(ColorSpace::D65Gray)
            }
        }
        _ => None,
    };

    let deferred = Deferred::new(move || match image.kind() {
        ImageKind::Raster(raster) => {
            let raster = raster.clone();
            let (width, height) = (raster.width(), raster.height());
            let (data, filter, has_color) = encode_raster_image(&raster);
            let icc = raster.icc().map(deflate);

            let alpha =
                raster.dynamic().color().has_alpha().then(|| encode_alpha(&raster));

            Ok(EncodedImage::Raster {
                data,
                filter,
                has_color,
                width,
                height,
                icc,
                alpha,
            })
        }
        ImageKind::Svg(svg) => {
            let (chunk, id) = encode_svg(svg, pdfa)
                .map_err(|err| eco_format!("failed to convert SVG to PDF: {err}"))?;
            Ok(EncodedImage::Svg(chunk, id))
        }
    });

    (deferred, color_space)
}

/// Encode an image with a suitable filter and return the data, filter and
/// whether the image has color.
///
/// Skips the alpha channel as that's encoded separately.
#[typst_macros::time(name = "encode raster image")]
fn encode_raster_image(image: &RasterImage) -> (Vec<u8>, Filter, bool) {
    let dynamic = image.dynamic();
    let channel_count = dynamic.color().channel_count();
    let has_color = channel_count > 2;

    if image.format() == RasterFormat::Jpg {
        let mut data = Cursor::new(vec![]);
        dynamic.write_to(&mut data, image::ImageFormat::Jpeg).unwrap();
        (data.into_inner(), Filter::DctDecode, has_color)
    } else {
        // TODO: Encode flate streams with PNG-predictor?
        let data = match (dynamic, channel_count) {
            (DynamicImage::ImageLuma8(luma), _) => deflate(luma.as_raw()),
            (DynamicImage::ImageRgb8(rgb), _) => deflate(rgb.as_raw()),
            // Grayscale image
            (_, 1 | 2) => deflate(dynamic.to_luma8().as_raw()),
            // Anything else
            _ => deflate(dynamic.to_rgb8().as_raw()),
        };
        (data, Filter::FlateDecode, has_color)
    }
}

/// Encode an image's alpha channel if present.
#[typst_macros::time(name = "encode alpha")]
fn encode_alpha(raster: &RasterImage) -> (Vec<u8>, Filter) {
    let pixels: Vec<_> = raster
        .dynamic()
        .pixels()
        .map(|(_, _, Rgba([_, _, _, a]))| a)
        .collect();
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
        /// Whether the image has color.
        has_color: bool,
        /// The image's width.
        width: u32,
        /// The image's height.
        height: u32,
        /// The image's ICC profile, pre-deflated, if any.
        icc: Option<Vec<u8>>,
        /// The alpha channel of the image, pre-deflated, if any.
        alpha: Option<(Vec<u8>, Filter)>,
    },
    /// A vector graphic.
    ///
    /// The chunk is the SVG converted to PDF objects.
    Svg(Chunk, Ref),
}
