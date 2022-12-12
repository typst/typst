//! Image handling.

use std::io;
use std::sync::Arc;

use crate::diag::{format_xml_like_error, StrResult};
use crate::util::Buffer;

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

impl Image {
    /// Create an image from a buffer and a format.
    ///
    /// Extracts the width and height.
    pub fn new(data: Buffer, format: ImageFormat) -> StrResult<Self> {
        let (width, height) = determine_size(&data, format)?;
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
    #[comemo::memoize]
    pub fn decode(&self) -> StrResult<Arc<DecodedImage>> {
        Ok(Arc::new(match self.format {
            ImageFormat::Vector(VectorFormat::Svg) => {
                let opts = usvg::Options::default();
                let tree = usvg::Tree::from_data(&self.data, &opts.to_ref())
                    .map_err(format_usvg_error)?;
                DecodedImage::Svg(tree)
            }
            ImageFormat::Raster(format) => {
                let cursor = io::Cursor::new(&self.data);
                let reader = image::io::Reader::with_format(cursor, format.into());
                let dynamic = reader.decode().map_err(format_image_error)?;
                DecodedImage::Raster(dynamic, format)
            }
        }))
    }
}

/// A raster or vector image format.
#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
pub enum ImageFormat {
    /// A raster graphics format.
    Raster(RasterFormat),
    /// A vector graphics format.
    Vector(VectorFormat),
}

/// A raster graphics format.
#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
pub enum RasterFormat {
    /// Raster format for illustrations and transparent graphics.
    Png,
    /// Lossy raster format suitable for photos.
    Jpg,
    /// Raster format that is typically used for short animated clips.
    Gif,
}

/// A vector graphics format.
#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
pub enum VectorFormat {
    /// The vector graphics format of the web.
    Svg,
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

impl From<ttf_parser::RasterImageFormat> for RasterFormat {
    fn from(format: ttf_parser::RasterImageFormat) -> Self {
        match format {
            ttf_parser::RasterImageFormat::PNG => RasterFormat::Png,
        }
    }
}

impl From<ttf_parser::RasterImageFormat> for ImageFormat {
    fn from(format: ttf_parser::RasterImageFormat) -> Self {
        Self::Raster(format.into())
    }
}

/// A decoded image.
pub enum DecodedImage {
    /// A decoded pixel raster.
    Raster(image::DynamicImage, RasterFormat),
    /// An decoded SVG tree.
    Svg(usvg::Tree),
}

/// Determine the image size in pixels.
#[comemo::memoize]
fn determine_size(data: &Buffer, format: ImageFormat) -> StrResult<(u32, u32)> {
    match format {
        ImageFormat::Raster(format) => {
            let cursor = io::Cursor::new(&data);
            let reader = image::io::Reader::with_format(cursor, format.into());
            Ok(reader.into_dimensions().map_err(format_image_error)?)
        }
        ImageFormat::Vector(VectorFormat::Svg) => {
            let opts = usvg::Options::default();
            let tree = usvg::Tree::from_data(&data, &opts.to_ref())
                .map_err(format_usvg_error)?;

            let size = tree.svg_node().size;
            let width = size.width().ceil() as u32;
            let height = size.height().ceil() as u32;
            Ok((width, height))
        }
    }
}

/// Format the user-facing raster graphic decoding error message.
fn format_image_error(error: image::ImageError) -> String {
    match error {
        image::ImageError::Limits(_) => "file is too large".into(),
        _ => "failed to decode image".into(),
    }
}

/// Format the user-facing SVG decoding error message.
fn format_usvg_error(error: usvg::Error) -> String {
    match error {
        usvg::Error::NotAnUtf8Str => "file is not valid utf-8".into(),
        usvg::Error::MalformedGZip => "file is not compressed correctly".into(),
        usvg::Error::ElementsLimitReached => "file is too large".into(),
        usvg::Error::InvalidSize => {
            "failed to parse svg: width, height, or viewbox is invalid".into()
        }
        usvg::Error::ParsingFailed(error) => format_xml_like_error("svg", error),
    }
}
