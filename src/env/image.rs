use std::fmt::{self, Debug, Formatter};
use std::io::Cursor;

use image::io::Reader as ImageReader;
use image::{DynamicImage, GenericImageView, ImageFormat};

use super::Buffer;

/// A loaded image resource.
pub struct ImageResource {
    /// The original format the image was encoded in.
    pub format: ImageFormat,
    /// The decoded image.
    pub buf: DynamicImage,
}

impl ImageResource {
    /// Parse an image resource from raw data in a supported format.
    ///
    /// The image format is determined automatically.
    pub fn parse(data: Buffer) -> Option<Self> {
        let cursor = Cursor::new(data.as_ref());
        let reader = ImageReader::new(cursor).with_guessed_format().ok()?;
        let format = reader.format()?;
        let buf = reader.decode().ok()?;
        Some(Self { format, buf })
    }
}

impl Debug for ImageResource {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        let (width, height) = self.buf.dimensions();
        f.debug_struct("ImageResource")
            .field("format", &self.format)
            .field("color", &self.buf.color())
            .field("width", &width)
            .field("height", &height)
            .finish()
    }
}
