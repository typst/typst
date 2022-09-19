//! Image handling.

use std::io;

use crate::loading::Buffer;

/// A raster or vector image.
///
/// Values of this type are cheap to clone and hash.
#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub struct Image {
    /// The raw, undecoded image data.
    data: Buffer,
    /// The format of the encoded `buffer`.
    format: ImageFormat,
    /// The width in pixels.
    width: u32,
    /// The height in pixels.
    height: u32,
}

/// A decoded image.
pub enum DecodedImage {
    /// A pixel raster format, like PNG or JPEG.
    Raster(image::DynamicImage),
    /// An SVG vector graphic.
    Svg(usvg::Tree),
}

/// A raster or vector image format.
#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
pub enum ImageFormat {
    /// Raster format for illustrations and transparent graphics.
    Png,
    /// Lossy raster format suitable for photos.
    Jpg,
    /// Raster format that is typically used for short animated clips.
    Gif,
    /// The vector graphics format of the web.
    Svg,
}

impl Image {
    /// Create an image from a raw buffer and a file extension.
    ///
    /// The file extension is used to determine the format.
    pub fn new(data: Buffer, ext: &str) -> io::Result<Self> {
        let format = match ext {
            "svg" | "svgz" => ImageFormat::Svg,
            "png" => ImageFormat::Png,
            "jpg" | "jpeg" => ImageFormat::Jpg,
            "gif" => ImageFormat::Gif,
            _ => {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    "unknown image format",
                ));
            }
        };

        let (width, height) = match format {
            ImageFormat::Svg => {
                let opts = usvg::Options::default();
                let tree =
                    usvg::Tree::from_data(&data, &opts.to_ref()).map_err(invalid)?;

                let size = tree.svg_node().size;
                let width = size.width().ceil() as u32;
                let height = size.height().ceil() as u32;
                (width, height)
            }
            _ => {
                let cursor = io::Cursor::new(&data);
                let format = convert_format(format);
                let reader = image::io::Reader::with_format(cursor, format);
                reader.into_dimensions().map_err(invalid)?
            }
        };

        Ok(Self { data, format, width, height })
    }

    /// The raw image data.
    pub fn data(&self) -> &Buffer {
        &self.data
    }

    /// The format of the image.
    pub fn format(&self) -> ImageFormat {
        self.format
    }

    /// The width of the image in pixels.
    pub fn width(&self) -> u32 {
        self.width
    }

    /// The height of the image in pixels.
    pub fn height(&self) -> u32 {
        self.height
    }

    /// Decode the image.
    pub fn decode(&self) -> io::Result<DecodedImage> {
        Ok(match self.format {
            ImageFormat::Svg => {
                let opts = usvg::Options::default();
                let tree =
                    usvg::Tree::from_data(&self.data, &opts.to_ref()).map_err(invalid)?;
                DecodedImage::Svg(tree)
            }
            _ => {
                let cursor = io::Cursor::new(&self.data);
                let format = convert_format(self.format);
                let reader = image::io::Reader::with_format(cursor, format);
                let dynamic = reader.decode().map_err(invalid)?;
                DecodedImage::Raster(dynamic)
            }
        })
    }
}

/// Convert a raster image format to the image crate's format.
fn convert_format(format: ImageFormat) -> image::ImageFormat {
    match format {
        ImageFormat::Png => image::ImageFormat::Png,
        ImageFormat::Jpg => image::ImageFormat::Jpeg,
        ImageFormat::Gif => image::ImageFormat::Gif,
        ImageFormat::Svg => panic!("must be a raster format"),
    }
}

/// Turn any error into an I/O error.
fn invalid<E>(error: E) -> io::Error
where
    E: std::error::Error + Send + Sync + 'static,
{
    io::Error::new(io::ErrorKind::InvalidData, error)
}
