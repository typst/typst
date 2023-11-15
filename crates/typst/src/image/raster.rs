use std::hash::{Hash, Hasher};
use std::io;
use std::sync::Arc;

use ecow::{eco_format, EcoString};
use image::codecs::gif::GifDecoder;
use image::codecs::jpeg::JpegDecoder;
use image::codecs::png::PngDecoder;
use image::io::Limits;
use image::{guess_format, ImageDecoder, ImageResult};
use typst_macros::Cast;

use crate::diag::{bail, StrResult};
use crate::eval::Bytes;

/// A decoded raster image.
#[derive(Clone, Hash)]
pub struct RasterImage(Arc<Repr>);

/// The internal representation.
struct Repr {
    data: Bytes,
    format: RasterFormat,
    dynamic: image::DynamicImage,
    icc: Option<Vec<u8>>,
}

impl RasterImage {
    /// Decode a raster image.
    #[comemo::memoize]
    pub fn new(data: Bytes, format: RasterFormat) -> StrResult<Self> {
        fn decode_with<'a, T: ImageDecoder<'a>>(
            decoder: ImageResult<T>,
        ) -> ImageResult<(image::DynamicImage, Option<Vec<u8>>)> {
            let mut decoder = decoder?;
            let icc = decoder.icc_profile().filter(|icc| !icc.is_empty());
            decoder.set_limits(Limits::default())?;
            let dynamic = image::DynamicImage::from_decoder(decoder)?;
            Ok((dynamic, icc))
        }

        let cursor = io::Cursor::new(&data);
        let (dynamic, icc) = match format {
            RasterFormat::Jpg => decode_with(JpegDecoder::new(cursor)),
            RasterFormat::Png => decode_with(PngDecoder::new(cursor)),
            RasterFormat::Gif => decode_with(GifDecoder::new(cursor)),
        }
        .map_err(format_image_error)?;

        Ok(Self(Arc::new(Repr { data, format, dynamic, icc })))
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

/// Format the user-facing raster graphic decoding error message.
fn format_image_error(error: image::ImageError) -> EcoString {
    match error {
        image::ImageError::Limits(_) => "file is too large".into(),
        err => eco_format!("failed to decode image ({err})"),
    }
}
