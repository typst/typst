use std::collections::HashMap;
use std::io::Cursor;
use std::sync::Arc;

use image::{DynamicImage, GenericImageView, Rgba};
use pdf_writer::{Chunk, Filter, Finish, Ref};

use super::{deflate, PdfContext};
use crate::geom::ColorSpace;
use crate::image::{ImageKind, RasterFormat, RasterImage, SvgImage};

/// Embed all used images into the PDF.
#[tracing::instrument(skip_all)]
pub fn write_images(ctx: &mut PdfContext) {
    for image in ctx.image_map.items() {
        // Add the primary image.
        match image.kind() {
            ImageKind::Raster(raster) => {
                // TODO: Error if image could not be encoded.
                let (data, filter, has_color) = encode_raster_image(raster);
                let width = image.width();
                let height = image.height();

                let image_ref = ctx.alloc.bump();
                ctx.image_refs.push(image_ref);

                let mut image = ctx.pdf.image_xobject(image_ref, &data);
                image.filter(filter);
                image.width(width as i32);
                image.height(height as i32);
                image.bits_per_component(8);

                let mut icc_ref = None;
                let space = image.color_space();
                if raster.icc().is_some() {
                    let id = ctx.alloc.bump();
                    space.icc_based(id);
                    icc_ref = Some(id);
                } else if has_color {
                    ctx.colors.write(ColorSpace::Srgb, space, &mut ctx.alloc);
                } else {
                    ctx.colors.write(ColorSpace::D65Gray, space, &mut ctx.alloc);
                }

                // Add a second gray-scale image containing the alpha values if
                // this image has an alpha channel.
                if raster.dynamic().color().has_alpha() {
                    let (alpha_data, alpha_filter) = encode_alpha(raster);
                    let mask_ref = ctx.alloc.bump();
                    image.s_mask(mask_ref);
                    image.finish();

                    let mut mask = ctx.pdf.image_xobject(mask_ref, &alpha_data);
                    mask.filter(alpha_filter);
                    mask.width(width as i32);
                    mask.height(height as i32);
                    mask.color_space().device_gray();
                    mask.bits_per_component(8);
                } else {
                    image.finish();
                }

                if let (Some(icc), Some(icc_ref)) = (raster.icc(), icc_ref) {
                    let compressed = deflate(icc);
                    let mut stream = ctx.pdf.icc_profile(icc_ref, &compressed);
                    stream.filter(Filter::FlateDecode);
                    if has_color {
                        stream.n(3);
                        stream.alternate().srgb();
                    } else {
                        stream.n(1);
                        stream.alternate().d65_gray();
                    }
                }
            }

            ImageKind::Svg(svg) => {
                let chunk = encode_svg(svg);
                let mut map = HashMap::new();
                chunk.renumber_into(&mut ctx.pdf, |old| {
                    *map.entry(old).or_insert_with(|| ctx.alloc.bump())
                });
                ctx.image_refs.push(map[&Ref::new(1)]);
            }
        }
    }
}

/// Encode an image with a suitable filter and return the data, filter and
/// whether the image has color.
///
/// Skips the alpha channel as that's encoded separately.
#[comemo::memoize]
#[tracing::instrument(skip_all)]
fn encode_raster_image(image: &RasterImage) -> (Arc<Vec<u8>>, Filter, bool) {
    let dynamic = image.dynamic();
    match (image.format(), dynamic) {
        // 8-bit gray JPEG.
        (RasterFormat::Jpg, DynamicImage::ImageLuma8(_)) => {
            let mut data = Cursor::new(vec![]);
            dynamic.write_to(&mut data, image::ImageFormat::Jpeg).unwrap();
            (data.into_inner().into(), Filter::DctDecode, false)
        }

        // 8-bit RGB JPEG (CMYK JPEGs get converted to RGB earlier).
        (RasterFormat::Jpg, DynamicImage::ImageRgb8(_)) => {
            let mut data = Cursor::new(vec![]);
            dynamic.write_to(&mut data, image::ImageFormat::Jpeg).unwrap();
            (data.into_inner().into(), Filter::DctDecode, true)
        }

        // TODO: Encode flate streams with PNG-predictor?

        // 8-bit gray PNG.
        (RasterFormat::Png, DynamicImage::ImageLuma8(luma)) => {
            let data = deflate(luma.as_raw());
            (data.into(), Filter::FlateDecode, false)
        }

        // Anything else (including Rgb(a) PNGs).
        (_, buf) => {
            let (width, height) = buf.dimensions();
            let mut pixels = Vec::with_capacity(3 * width as usize * height as usize);
            for (_, _, Rgba([r, g, b, _])) in buf.pixels() {
                pixels.push(r);
                pixels.push(g);
                pixels.push(b);
            }

            let data = deflate(&pixels);
            (data.into(), Filter::FlateDecode, true)
        }
    }
}

/// Encode an image's alpha channel if present.
#[comemo::memoize]
#[tracing::instrument(skip_all)]
fn encode_alpha(raster: &RasterImage) -> (Arc<Vec<u8>>, Filter) {
    let pixels: Vec<_> = raster
        .dynamic()
        .pixels()
        .map(|(_, _, Rgba([_, _, _, a]))| a)
        .collect();
    (Arc::new(deflate(&pixels)), Filter::FlateDecode)
}

/// Encode an SVG into a chunk of PDF objects.
///
/// The main XObject will have ID 1.
#[comemo::memoize]
#[tracing::instrument(skip_all)]
fn encode_svg(svg: &SvgImage) -> Arc<Chunk> {
    let mut chunk = Chunk::new();

    // Safety: We do not keep any references to tree nodes beyond the
    // scope of `with`.
    unsafe {
        svg.with(|tree| {
            svg2pdf::convert_tree_into(
                tree,
                svg2pdf::Options::default(),
                &mut chunk,
                Ref::new(1),
            );
        });
    }

    Arc::new(chunk)
}
