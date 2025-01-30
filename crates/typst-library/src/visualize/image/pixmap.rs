use std::sync::Arc;

use image::{DynamicImage, ImageBuffer, Pixel};

use crate::diag::{bail, StrResult};
use crate::foundations::{cast, dict, Bytes, Cast, Dict};

/// A raster image based on a flat pixmap.
#[derive(Clone, Hash)]
pub struct PixmapImage(Arc<Repr>);

/// The internal representation.
#[derive(Hash)]
struct Repr {
    source: PixmapSource,
    format: PixmapFormat,
}

impl PixmapImage {
    /// Builds a new [`PixmapImage`] from a flat, uncompressed byte sequence.
    #[comemo::memoize]
    #[typst_macros::time(name = "load pixmap")]
    pub fn new(source: PixmapSource, format: PixmapFormat) -> StrResult<PixmapImage> {
        if source.pixel_width == 0 || source.pixel_height == 0 {
            bail!("zero-sized images are not allowed");
        }

        let pixel_size = match format {
            PixmapFormat::Rgb8 => 3,
            PixmapFormat::Rgba8 => 4,
            PixmapFormat::Luma8 => 1,
            PixmapFormat::Lumaa8 => 2,
        };

        let Some(expected_size) = source
            .pixel_width
            .checked_mul(source.pixel_height)
            .and_then(|size| size.checked_mul(pixel_size))
        else {
            bail!("provided pixel dimensions are too large");
        };

        if expected_size as usize != source.data.len() {
            bail!("provided pixel dimensions and pixmap data do not match");
        }

        Ok(Self(Arc::new(Repr { source, format })))
    }

    /// The image's format.
    pub fn format(&self) -> PixmapFormat {
        self.0.format
    }

    /// The image's pixel width.
    pub fn width(&self) -> u32 {
        self.0.source.pixel_width
    }

    /// The image's pixel height.
    pub fn height(&self) -> u32 {
        self.0.source.pixel_height
    }

    /// The raw data encoded in the given format.
    pub fn data(&self) -> &Bytes {
        &self.0.source.data
    }

    /// Transform the image data into a [`DynamicImage`].
    #[comemo::memoize]
    pub fn to_image(&self) -> Arc<DynamicImage> {
        // TODO: Optimize by returning a `View` if possible?
        fn decode<P: Pixel<Subpixel = u8>>(
            source: &PixmapSource,
        ) -> ImageBuffer<P, Vec<u8>> {
            ImageBuffer::from_raw(
                source.pixel_width,
                source.pixel_height,
                source.data.to_vec(),
            )
            .unwrap()
        }
        Arc::new(match self.0.format {
            PixmapFormat::Rgb8 => decode::<image::Rgb<u8>>(&self.0.source).into(),
            PixmapFormat::Rgba8 => decode::<image::Rgba<u8>>(&self.0.source).into(),
            PixmapFormat::Luma8 => decode::<image::Luma<u8>>(&self.0.source).into(),
            PixmapFormat::Lumaa8 => decode::<image::LumaA<u8>>(&self.0.source).into(),
        })
    }

    /// Access the ICC profile, if any.
    pub fn icc_profile(&self) -> Option<&[u8]> {
        self.0.source.icc_profile.as_deref()
    }
}

/// Determines how the given image is interpreted and encoded.
#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash, Cast)]
pub enum PixmapFormat {
    /// Red, green, and blue channels, one byte per channel.
    /// No alpha channel.
    Rgb8,
    /// Red, green, blue, and alpha channels, one byte per channel.
    Rgba8,
    /// A single byte channel representing brightness.
    Luma8,
    /// Brightness and alpha, one byte per channel.
    Lumaa8,
}

/// Raw pixmap data and relevant metadata.
#[derive(Debug, Clone, PartialEq, Hash)]
pub struct PixmapSource {
    pub data: Bytes,
    pub pixel_width: u32,
    pub pixel_height: u32,
    pub icc_profile: Option<Bytes>,
}

cast! {
    PixmapSource,
    self => dict! {
        "data" => self.data.clone(),
        "pixel-width" => self.pixel_width,
        "pixel-height" => self.pixel_height,
        "icc-profile" => self.icc_profile.clone()
    }.into_value(),
    mut dict: Dict => {
        let source = PixmapSource {
            data: dict.take("data")?.cast()?,
            pixel_width: dict.take("pixel-width")?.cast()?,
            pixel_height: dict.take("pixel-height")?.cast()?,
            icc_profile: dict.take("icc-profile").ok().map(|v| v.cast()).transpose()?,
        };
        dict.finish(&["data", "pixel-width", "pixel-height", "icc-profile"])?;
        source
    }
}
