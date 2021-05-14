use std::fmt::{self, Debug, Formatter};
use std::io::Cursor;

use image::io::Reader as ImageReader;
use image::{DynamicImage, GenericImageView, ImageFormat};

/// A loaded image.
pub struct Image {
    /// The original format the image was encoded in.
    pub format: ImageFormat,
    /// The decoded image.
    pub buf: DynamicImage,
}

impl Image {
    /// Parse an image from raw data in a supported format.
    ///
    /// The image format is determined automatically.
    pub fn parse(data: &[u8]) -> Option<Self> {
        let cursor = Cursor::new(data);
        let reader = ImageReader::new(cursor).with_guessed_format().ok()?;
        let format = reader.format()?;
        let buf = reader.decode().ok()?;
        Some(Self { format, buf })
    }

    /// The width of the image.
    pub fn width(&self) -> u32 {
        self.buf.width()
    }

    /// The height of the image.
    pub fn height(&self) -> u32 {
        self.buf.height()
    }
}

impl Debug for Image {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        f.debug_struct("Image")
            .field("format", &self.format)
            .field("color", &self.buf.color())
            .field("width", &self.width())
            .field("height", &self.height())
            .finish()
    }
}
