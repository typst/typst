use std::cmp::Ordering;
use std::hash::{Hash, Hasher};
use std::io;
use std::sync::Arc;

use ecow::{eco_format, EcoString};
use image::codecs::gif::GifDecoder;
use image::codecs::jpeg::JpegDecoder;
use image::codecs::png::PngDecoder;
use image::io::Limits;
use image::{guess_format, DynamicImage, ImageDecoder, ImageResult};

use crate::diag::{bail, StrResult};
use crate::foundations::{Bytes, Cast};

/// A decoded raster image.
#[derive(Clone, Hash)]
pub struct RasterImage(Arc<Repr>);

/// The internal representation.
struct Repr {
    data: Bytes,
    format: RasterFormat,
    dynamic: image::DynamicImage,
    icc: Option<Vec<u8>>,
    dpi: Option<f64>,
}

impl RasterImage {
    /// Decode a raster image.
    #[comemo::memoize]
    pub fn new(data: Bytes, format: RasterFormat) -> StrResult<RasterImage> {
        fn decode_with<T: ImageDecoder>(
            decoder: ImageResult<T>,
        ) -> ImageResult<(image::DynamicImage, Option<Vec<u8>>)> {
            let mut decoder = decoder?;
            let icc = decoder.icc_profile().ok().flatten().filter(|icc| !icc.is_empty());
            decoder.set_limits(Limits::default())?;
            let dynamic = image::DynamicImage::from_decoder(decoder)?;
            Ok((dynamic, icc))
        }

        let cursor = io::Cursor::new(&data);
        let (mut dynamic, icc) = match format {
            RasterFormat::Jpg => decode_with(JpegDecoder::new(cursor)),
            RasterFormat::Png => decode_with(PngDecoder::new(cursor)),
            RasterFormat::Gif => decode_with(GifDecoder::new(cursor)),
        }
        .map_err(format_image_error)?;

        let exif = exif::Reader::new()
            .read_from_container(&mut std::io::Cursor::new(&data))
            .ok();

        // Apply rotation from EXIF metadata.
        if let Some(rotation) = exif.as_ref().and_then(exif_rotation) {
            apply_rotation(&mut dynamic, rotation);
        }

        // Extract pixel density.
        let dpi = determine_dpi(&data, exif.as_ref());

        Ok(Self(Arc::new(Repr { data, format, dynamic, icc, dpi })))
    }

    /// The raw image data.
    pub fn data(&self) -> &Bytes {
        &self.0.data
    }

    /// The image's format.
    pub fn format(&self) -> RasterFormat {
        self.0.format
    }

    /// The image's pixel width.
    pub fn width(&self) -> u32 {
        self.dynamic().width()
    }

    /// The image's pixel height.
    pub fn height(&self) -> u32 {
        self.dynamic().height()
    }

    /// The image's pixel density in pixels per inch, if known.
    pub fn dpi(&self) -> Option<f64> {
        self.0.dpi
    }

    /// Access the underlying dynamic image.
    pub fn dynamic(&self) -> &image::DynamicImage {
        &self.0.dynamic
    }

    /// Access the ICC profile, if any.
    pub fn icc(&self) -> Option<&[u8]> {
        self.0.icc.as_deref()
    }
}

impl Hash for Repr {
    fn hash<H: Hasher>(&self, state: &mut H) {
        // The image is fully defined by data and format.
        self.data.hash(state);
        self.format.hash(state);
    }
}

/// A raster graphics format.
#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash, Cast)]
pub enum RasterFormat {
    /// Raster format for illustrations and transparent graphics.
    Png,
    /// Lossy raster format suitable for photos.
    Jpg,
    /// Raster format that is typically used for short animated clips.
    Gif,
}

impl RasterFormat {
    /// Try to detect the format of data in a buffer.
    pub fn detect(data: &[u8]) -> Option<Self> {
        guess_format(data).ok().and_then(|format| format.try_into().ok())
    }
}

impl From<RasterFormat> for image::ImageFormat {
    fn from(format: RasterFormat) -> Self {
        match format {
            RasterFormat::Png => image::ImageFormat::Png,
            RasterFormat::Jpg => image::ImageFormat::Jpeg,
            RasterFormat::Gif => image::ImageFormat::Gif,
        }
    }
}

impl TryFrom<image::ImageFormat> for RasterFormat {
    type Error = EcoString;

    fn try_from(format: image::ImageFormat) -> StrResult<Self> {
        Ok(match format {
            image::ImageFormat::Png => RasterFormat::Png,
            image::ImageFormat::Jpeg => RasterFormat::Jpg,
            image::ImageFormat::Gif => RasterFormat::Gif,
            _ => bail!("Format not yet supported."),
        })
    }
}

/// Try to get the rotation from the EXIF metadata.
fn exif_rotation(exif: &exif::Exif) -> Option<u32> {
    exif.get_field(exif::Tag::Orientation, exif::In::PRIMARY)?
        .value
        .get_uint(0)
}

/// Apply an EXIF rotation to a dynamic image.
fn apply_rotation(image: &mut DynamicImage, rotation: u32) {
    use image::imageops as ops;
    match rotation {
        2 => ops::flip_horizontal_in_place(image),
        3 => ops::rotate180_in_place(image),
        4 => ops::flip_vertical_in_place(image),
        5 => {
            ops::flip_horizontal_in_place(image);
            *image = image.rotate270();
        }
        6 => *image = image.rotate90(),
        7 => {
            ops::flip_horizontal_in_place(image);
            *image = image.rotate90();
        }
        8 => *image = image.rotate270(),
        _ => {}
    }
}

/// Try to determine the DPI (dots per inch) of the image.
fn determine_dpi(data: &[u8], exif: Option<&exif::Exif>) -> Option<f64> {
    // Try to extract the DPI from the EXIF metadata. If that doesn't yield
    // anything, fall back to specialized procedures for extracting JPEG or PNG
    // DPI metadata. GIF does not have any.
    exif.and_then(exif_dpi)
        .or_else(|| jpeg_dpi(data))
        .or_else(|| png_dpi(data))
}

/// Try to get the DPI from the EXIF metadata.
fn exif_dpi(exif: &exif::Exif) -> Option<f64> {
    let axis = |tag| {
        let dpi = exif.get_field(tag, exif::In::PRIMARY)?;
        let exif::Value::Rational(rational) = &dpi.value else { return None };
        Some(rational.first()?.to_f64())
    };

    [axis(exif::Tag::XResolution), axis(exif::Tag::YResolution)]
        .into_iter()
        .flatten()
        .max_by(|a, b| a.partial_cmp(b).unwrap_or(Ordering::Equal))
}

/// Tries to extract the DPI from raw JPEG data (by inspecting the JFIF APP0
/// section).
fn jpeg_dpi(data: &[u8]) -> Option<f64> {
    let validate_at = |index: usize, expect: &[u8]| -> Option<()> {
        data.get(index..)?.starts_with(expect).then_some(())
    };
    let u16_at = |index: usize| -> Option<u16> {
        data.get(index..index + 2)?.try_into().ok().map(u16::from_be_bytes)
    };

    validate_at(0, b"\xFF\xD8\xFF\xE0\0")?;
    validate_at(6, b"JFIF\0")?;
    validate_at(11, b"\x01")?;

    let len = u16_at(4)?;
    if len < 16 {
        return None;
    }

    let units = *data.get(13)?;
    let x = u16_at(14)?;
    let y = u16_at(16)?;
    let dpu = x.max(y) as f64;

    Some(match units {
        1 => dpu,        // already inches
        2 => dpu * 2.54, // cm -> inches
        _ => return None,
    })
}

/// Tries to extract the DPI from raw PNG data.
fn png_dpi(mut data: &[u8]) -> Option<f64> {
    let mut decoder = png::StreamingDecoder::new();
    let dims = loop {
        let (consumed, event) = decoder.update(data, &mut Vec::new()).ok()?;
        match event {
            png::Decoded::PixelDimensions(dims) => break dims,
            // Bail as soon as there is anything data-like.
            png::Decoded::ChunkBegin(_, png::chunk::IDAT)
            | png::Decoded::ImageData
            | png::Decoded::ImageEnd => return None,
            _ => {}
        }
        data = data.get(consumed..)?;
        if consumed == 0 {
            return None;
        }
    };

    let dpu = dims.xppu.max(dims.yppu) as f64;
    match dims.unit {
        png::Unit::Meter => Some(dpu * 0.0254), // meter -> inches
        png::Unit::Unspecified => None,
    }
}

/// Format the user-facing raster graphic decoding error message.
fn format_image_error(error: image::ImageError) -> EcoString {
    match error {
        image::ImageError::Limits(_) => "file is too large".into(),
        err => eco_format!("failed to decode image ({err})"),
    }
}

#[cfg(test)]
mod tests {
    use super::{RasterFormat, RasterImage};
    use crate::foundations::Bytes;

    #[test]
    fn test_image_dpi() {
        #[track_caller]
        fn test(path: &str, format: RasterFormat, dpi: f64) {
            let data = typst_dev_assets::get(path).unwrap();
            let bytes = Bytes::from_static(data);
            let image = RasterImage::new(bytes, format).unwrap();
            assert_eq!(image.dpi().map(f64::round), Some(dpi));
        }

        test("images/f2t.jpg", RasterFormat::Jpg, 220.0);
        test("images/tiger.jpg", RasterFormat::Jpg, 72.0);
        test("images/graph.png", RasterFormat::Png, 144.0);
    }
}
