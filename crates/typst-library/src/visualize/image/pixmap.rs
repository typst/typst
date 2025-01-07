use std::sync::Arc;

use image::{DynamicImage, ImageBuffer, Pixel};

use crate::diag::{bail, StrResult};
use crate::foundations::{Bytes, Cast};

#[derive(Debug, PartialEq, Hash)]
pub struct PixmapSource {
    pub data: Bytes,
    pub pixel_width: u32,
    pub pixel_height: u32,
    pub icc_profile: Option<Bytes>,
}

/// A raster image based on a flat pixmap.
#[derive(Clone, Hash)]
pub struct Pixmap(Arc<Repr>);

/// The internal representation.
#[derive(Hash)]
struct Repr {
    source: Arc<PixmapSource>,
    format: PixmapFormat,
}

impl Pixmap {
    /// Build a new [`Pixmap`] from a flat, uncompressed byte sequence.
    #[comemo::memoize]
    pub fn new(source: Arc<PixmapSource>, format: PixmapFormat) -> StrResult<Pixmap> {
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
    pub fn data(&self) -> &[u8] {
        self.0.source.data.as_slice()
    }

    /// Transform the image data into an [`DynamicImage`].
    #[comemo::memoize]
    pub fn to_image(&self) -> Arc<DynamicImage> {
        // TODO optimize by returning a `View` if possible?
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
    /// The red, green, and blue channels are each eight bit integers.
    /// There is no alpha channel.
    Rgb8,
    /// The red, green, blue, and alpha channels are each eight bit integers.
    Rgba8,
    /// A single eight bit channel representing brightness.
    Luma8,
    /// One byte of brightness, another for alpha.
    Lumaa8,
}
