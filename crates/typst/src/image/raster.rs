use std::hash::{Hash, Hasher};
use std::io::{self, Read};
use std::sync::Arc;

use ecow::{eco_format, EcoString};
use image::codecs::gif::GifDecoder;
use image::codecs::jpeg::JpegDecoder;
use image::codecs::png::PngDecoder;
use image::io::Limits;
use image::{guess_format, DynamicImage, ImageBuffer, ImageDecoder, ImageResult};
use jxl_oxide::JxlImage;
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
    dynamic: DynamicImage,
    icc: Option<Vec<u8>>,
}

impl RasterImage {
    /// Decode a raster image.
    #[comemo::memoize]
    pub fn new(data: Bytes, format: RasterFormat) -> StrResult<Self> {
        let cursor = io::Cursor::new(&data);
        let (dynamic, icc) = match format {
            RasterFormat::Jpg => {
                decode_with(JpegDecoder::new(cursor)).map_err(format_image_error)?
            }
            RasterFormat::Png => {
                decode_with(PngDecoder::new(cursor)).map_err(format_image_error)?
            }
            RasterFormat::Gif => {
                decode_with(GifDecoder::new(cursor)).map_err(format_image_error)?
            }
            RasterFormat::Jxl => decode_jxl(cursor)
                .map_err(|err| eco_format!("failed to decode jpeg xl image ({err})"))?,
        };

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
    pub fn dynamic(&self) -> &DynamicImage {
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

/// Decode an image using a decoder from the `image` crate.
///
/// Returns a decoded image and optionally an ICC profile.
fn decode_with<'a, T: ImageDecoder<'a>>(
    decoder: ImageResult<T>,
) -> ImageResult<(DynamicImage, Option<Vec<u8>>)> {
    let mut decoder = decoder?;
    let icc = decoder.icc_profile().filter(|icc| !icc.is_empty());
    decoder.set_limits(Limits::default())?;
    let dynamic = DynamicImage::from_decoder(decoder)?;
    Ok((dynamic, icc))
}

/// Decode a JPEG XL image using `jxl-oxide`.
///
/// Returns a decoded image and optionally an ICC profile.
fn decode_jxl<R: Read>(reader: R) -> StrResult<(DynamicImage, Option<Vec<u8>>)> {
    use jxl_oxide::{PixelFormat, RenderResult};

    let mut image = JxlImage::from_reader(reader)
        .map_err(|err| eco_format!("failed to decode JPEG XL image ({err})"))?;
    let width = image.width();
    let height = image.height();
    let bit_depth = image.image_header().metadata.bit_depth.bits_per_sample();
    let render = match image
        .render_next_frame()
        .map_err(|err| eco_format!("failed to decode JPEG XL image ({err})"))?
    {
        RenderResult::Done(render) => render,
        RenderResult::NeedMoreData => {
            return Err(eco_format!("JPEG XL frame is incomplete."))
        }
        RenderResult::NoMoreFrames => {
            return Err(eco_format!("Not enough frames are present in JPEG XL file."))
        }
    };
    let dynamic = match image.pixel_format() {
        PixelFormat::Gray => {
            if bit_depth <= 8 {
                let buf = render.color_channels()[0]
                    .buf()
                    .iter()
                    .map(|&f| (f * (u8::MAX as f32)) as u8)
                    .collect();
                let image_buffer = ImageBuffer::from_vec(width, height, buf).unwrap();
                DynamicImage::ImageLuma8(image_buffer)
            } else {
                let buf = render.color_channels()[0]
                    .buf()
                    .iter()
                    .map(|&f| (f * (u16::MAX as f32)) as u16)
                    .collect();
                let image_buffer = ImageBuffer::from_vec(width, height, buf).unwrap();
                DynamicImage::ImageLuma16(image_buffer)
            }
        }
        PixelFormat::Graya => {
            if bit_depth <= 8 {
                let buf = render
                    .image()
                    .buf()
                    .iter()
                    .map(|&f| (f * (u8::MAX as f32)) as u8)
                    .collect();
                let image_buffer = ImageBuffer::from_vec(width, height, buf).unwrap();
                DynamicImage::ImageLumaA8(image_buffer)
            } else {
                let buf = render
                    .image()
                    .buf()
                    .iter()
                    .map(|&f| (f * (u16::MAX as f32)) as u16)
                    .collect();
                let image_buffer = ImageBuffer::from_vec(width, height, buf).unwrap();
                DynamicImage::ImageLumaA16(image_buffer)
            }
        }
        PixelFormat::Rgb => {
            if bit_depth <= 8 {
                let buf = render
                    .image()
                    .buf()
                    .iter()
                    .map(|&f| (f * (u8::MAX as f32)) as u8)
                    .collect();
                let image_buffer = ImageBuffer::from_vec(width, height, buf).unwrap();
                DynamicImage::ImageRgb8(image_buffer)
            } else if bit_depth <= 16 {
                let buf = render
                    .image()
                    .buf()
                    .iter()
                    .map(|&f| (f * (u16::MAX as f32)) as u16)
                    .collect();
                let image_buffer = ImageBuffer::from_vec(width, height, buf).unwrap();
                DynamicImage::ImageRgb16(image_buffer)
            } else {
                let buf = render.image().buf().to_vec();
                let image_buffer = ImageBuffer::from_vec(width, height, buf).unwrap();
                DynamicImage::ImageRgb32F(image_buffer)
            }
        }
        PixelFormat::Rgba => {
            if bit_depth <= 8 {
                let buf = render
                    .image()
                    .buf()
                    .iter()
                    .map(|&f| (f / (u8::MAX as f32)) as u8)
                    .collect();
                let image_buffer = ImageBuffer::from_vec(width, height, buf).unwrap();
                DynamicImage::ImageRgba8(image_buffer)
            } else if bit_depth <= 16 {
                let buf = render
                    .image()
                    .buf()
                    .iter()
                    .map(|&f| (f / (u16::MAX as f32)) as u16)
                    .collect();
                let image_buffer = ImageBuffer::from_vec(width, height, buf).unwrap();
                DynamicImage::ImageRgba16(image_buffer)
            } else {
                let buf = render.image().buf().to_vec();
                let image_buffer = ImageBuffer::from_vec(width, height, buf).unwrap();
                DynamicImage::ImageRgba32F(image_buffer)
            }
        }
        PixelFormat::Cmyk => todo!(),
        PixelFormat::Cmyka => todo!(),
    };
    let icc = image.embedded_icc().map(|icc| icc.to_vec());
    Ok((dynamic, icc))
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
    /// Universal and modern raster format with great compression.
    Jxl,
}

impl RasterFormat {
    /// Try to detect the format of data in a buffer.
    pub fn detect(data: &[u8]) -> Option<Self> {
        guess_format(data).ok().and_then(|format| format.try_into().ok())
    }
}

impl TryFrom<RasterFormat> for image::ImageFormat {
    type Error = EcoString;

    fn try_from(format: RasterFormat) -> StrResult<Self> {
        Ok(match format {
            RasterFormat::Png => image::ImageFormat::Png,
            RasterFormat::Jpg => image::ImageFormat::Jpeg,
            RasterFormat::Gif => image::ImageFormat::Gif,
            RasterFormat::Jxl => bail!("Format not supported by the image crate."),
        })
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
