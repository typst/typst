use std::io::Cursor;

use image::{DynamicImage, GenericImageView, ImageResult, Rgba};
use pdf_writer::{Filter, Finish};

use super::{deflate, PdfContext, RefExt};
use crate::image::{DecodedImage, RasterFormat};

/// Embed all used images into the PDF.
pub fn write_images(ctx: &mut PdfContext) {
    for image in ctx.image_map.items() {
        let image_ref = ctx.alloc.bump();
        ctx.image_refs.push(image_ref);

        let width = image.width();
        let height = image.height();

        // Add the primary image.
        // TODO: Error if image could not be encoded.
        match image.decode().unwrap().as_ref() {
            DecodedImage::Raster(dynamic, format) => {
                // TODO: Error if image could not be encoded.
                let (data, filter, has_color) = encode_image(*format, dynamic).unwrap();
                let mut image = ctx.writer.image_xobject(image_ref, &data);
                image.filter(filter);
                image.width(width as i32);
                image.height(height as i32);
                image.bits_per_component(8);

                let space = image.color_space();
                if has_color {
                    space.device_rgb();
                } else {
                    space.device_gray();
                }

                // Add a second gray-scale image containing the alpha values if
                // this image has an alpha channel.
                if dynamic.color().has_alpha() {
                    let (alpha_data, alpha_filter) = encode_alpha(dynamic);
                    let mask_ref = ctx.alloc.bump();
                    image.s_mask(mask_ref);
                    image.finish();

                    let mut mask = ctx.writer.image_xobject(mask_ref, &alpha_data);
                    mask.filter(alpha_filter);
                    mask.width(width as i32);
                    mask.height(height as i32);
                    mask.color_space().device_gray();
                    mask.bits_per_component(8);
                }
            }
            DecodedImage::Svg(svg) => {
                let next_ref = svg2pdf::convert_tree_into(
                    svg,
                    svg2pdf::Options::default(),
                    &mut ctx.writer,
                    image_ref,
                );
                ctx.alloc = next_ref;
            }
        }
    }
}

/// Encode an image with a suitable filter and return the data, filter and
/// whether the image has color.
///
/// Skips the alpha channel as that's encoded separately.
fn encode_image(
    format: RasterFormat,
    dynamic: &DynamicImage,
) -> ImageResult<(Vec<u8>, Filter, bool)> {
    Ok(match (format, dynamic) {
        // 8-bit gray JPEG.
        (RasterFormat::Jpg, DynamicImage::ImageLuma8(_)) => {
            let mut data = Cursor::new(vec![]);
            dynamic.write_to(&mut data, image::ImageFormat::Jpeg)?;
            (data.into_inner(), Filter::DctDecode, false)
        }

        // 8-bit RGB JPEG (CMYK JPEGs get converted to RGB earlier).
        (RasterFormat::Jpg, DynamicImage::ImageRgb8(_)) => {
            let mut data = Cursor::new(vec![]);
            dynamic.write_to(&mut data, image::ImageFormat::Jpeg)?;
            (data.into_inner(), Filter::DctDecode, true)
        }

        // TODO: Encode flate streams with PNG-predictor?

        // 8-bit gray PNG.
        (RasterFormat::Png, DynamicImage::ImageLuma8(luma)) => {
            let data = deflate(luma.as_raw());
            (data, Filter::FlateDecode, false)
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
            (data, Filter::FlateDecode, true)
        }
    })
}

/// Encode an image's alpha channel if present.
fn encode_alpha(dynamic: &DynamicImage) -> (Vec<u8>, Filter) {
    let pixels: Vec<_> = dynamic.pixels().map(|(_, _, Rgba([_, _, _, a]))| a).collect();
    (deflate(&pixels), Filter::FlateDecode)
}
