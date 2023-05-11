//! Image handling.

use std::collections::BTreeMap;
use std::fmt::{self, Debug, Formatter};
use std::hash::{Hash, Hasher};
use std::io;
use std::sync::Arc;

use comemo::Tracked;
use ecow::EcoString;
use image::codecs::gif::GifDecoder;
use image::codecs::jpeg::JpegDecoder;
use image::codecs::png::PngDecoder;
use image::io::Limits;
use image::{ImageDecoder, ImageResult};
use usvg::{TreeParsing, TreeTextToPath};

use crate::diag::{format_xml_like_error, StrResult};
use crate::util::Buffer;
use crate::World;

/// A raster or vector image.
///
/// Values of this type are cheap to clone and hash.
#[derive(Clone)]
pub struct Image {
    /// The raw, undecoded image data.
    data: Buffer,
    /// The format of the encoded `buffer`.
    format: ImageFormat,
    /// The decoded image.
    decoded: Arc<DecodedImage>,
    /// A text describing the image.
    alt: Option<EcoString>,
}

impl Image {
    /// Create an image from a buffer and a format.
    pub fn new(
        data: Buffer,
        format: ImageFormat,
        alt: Option<EcoString>,
    ) -> StrResult<Self> {
        let decoded = match format {
            ImageFormat::Raster(format) => decode_raster(&data, format)?,
            ImageFormat::Vector(VectorFormat::Svg) => decode_svg(&data)?,
        };

        Ok(Self { data, format, decoded, alt })
    }

    /// Create a font-dependant image from a buffer and a format.
    pub fn with_fonts(
        data: Buffer,
        format: ImageFormat,
        world: Tracked<dyn World + '_>,
        fallback_family: Option<&str>,
        alt: Option<EcoString>,
    ) -> StrResult<Self> {
        let decoded = match format {
            ImageFormat::Raster(format) => decode_raster(&data, format)?,
            ImageFormat::Vector(VectorFormat::Svg) => {
                decode_svg_with_fonts(&data, world, fallback_family)?
            }
        };

        Ok(Self { data, format, decoded, alt })
    }

    /// The raw image data.
    pub fn data(&self) -> &Buffer {
        &self.data
    }

    /// The format of the image.
    pub fn format(&self) -> ImageFormat {
        self.format
    }

    /// The decoded version of the image.
    pub fn decoded(&self) -> &DecodedImage {
        &self.decoded
    }

    /// The width of the image in pixels.
    pub fn width(&self) -> u32 {
        self.decoded().width()
    }

    /// The height of the image in pixels.
    pub fn height(&self) -> u32 {
        self.decoded().height()
    }

    /// A text describing the image.
    pub fn alt(&self) -> Option<&str> {
        self.alt.as_deref()
    }
}

impl Debug for Image {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        f.debug_struct("Image")
            .field("format", &self.format())
            .field("width", &self.width())
            .field("height", &self.height())
            .field("alt", &self.alt())
            .finish()
    }
}

impl Eq for Image {}

impl PartialEq for Image {
    fn eq(&self, other: &Self) -> bool {
        self.data() == other.data() && self.format() == other.format()
    }
}

impl Hash for Image {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.data().hash(state);
        self.format().hash(state);
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
    /// A decoded pixel raster with its ICC profile.
    Raster(image::DynamicImage, Option<IccProfile>, RasterFormat),
    /// An decoded SVG tree.
    Svg(usvg::Tree),
}

impl DecodedImage {
    /// The width of the image in pixels.
    pub fn width(&self) -> u32 {
        match self {
            Self::Raster(dynamic, _, _) => dynamic.width(),
            Self::Svg(tree) => tree.size.width().ceil() as u32,
        }
    }

    /// The height of the image in pixels.
    pub fn height(&self) -> u32 {
        match self {
            Self::Raster(dynamic, _, _) => dynamic.height(),
            Self::Svg(tree) => tree.size.height().ceil() as u32,
        }
    }
}

/// Raw data for of an ICC profile.
pub struct IccProfile(pub Vec<u8>);

/// Decode a raster image.
#[comemo::memoize]
fn decode_raster(data: &Buffer, format: RasterFormat) -> StrResult<Arc<DecodedImage>> {
    fn decode_with<'a, T: ImageDecoder<'a>>(
        decoder: ImageResult<T>,
    ) -> ImageResult<(image::DynamicImage, Option<IccProfile>)> {
        let mut decoder = decoder?;
        let icc = decoder.icc_profile().map(IccProfile);
        decoder.set_limits(Limits::default())?;
        let dynamic = image::DynamicImage::from_decoder(decoder)?;
        Ok((dynamic, icc))
    }

    let cursor = io::Cursor::new(data);
    let (dynamic, icc) = match format {
        RasterFormat::Jpg => decode_with(JpegDecoder::new(cursor)),
        RasterFormat::Png => decode_with(PngDecoder::new(cursor)),
        RasterFormat::Gif => decode_with(GifDecoder::new(cursor)),
    }
    .map_err(format_image_error)?;

    Ok(Arc::new(DecodedImage::Raster(dynamic, icc, format)))
}

/// Decode an SVG image.
#[comemo::memoize]
fn decode_svg(data: &Buffer) -> StrResult<Arc<DecodedImage>> {
    let opts = usvg::Options::default();
    let tree = usvg::Tree::from_data(data, &opts).map_err(format_usvg_error)?;
    Ok(Arc::new(DecodedImage::Svg(tree)))
}

/// Decode an SVG image with access to fonts.
#[comemo::memoize]
fn decode_svg_with_fonts(
    data: &Buffer,
    world: Tracked<dyn World + '_>,
    fallback_family: Option<&str>,
) -> StrResult<Arc<DecodedImage>> {
    let mut opts = usvg::Options::default();

    // Recover the non-lowercased version of the family because
    // usvg is case sensitive.
    let book = world.book();
    let fallback_family = fallback_family
        .and_then(|lowercase| book.select_family(lowercase).next())
        .and_then(|index| book.info(index))
        .map(|info| info.family.clone());

    if let Some(family) = &fallback_family {
        opts.font_family = family.clone();
    }

    let mut tree = usvg::Tree::from_data(data, &opts).map_err(format_usvg_error)?;
    if tree.has_text_nodes() {
        let fontdb = load_svg_fonts(&tree, world, fallback_family.as_deref());
        tree.convert_text(&fontdb);
    }

    Ok(Arc::new(DecodedImage::Svg(tree)))
}

/// Discover and load the fonts referenced by an SVG.
fn load_svg_fonts(
    tree: &usvg::Tree,
    world: Tracked<dyn World + '_>,
    fallback_family: Option<&str>,
) -> fontdb::Database {
    let mut referenced = BTreeMap::<EcoString, bool>::new();
    let mut fontdb = fontdb::Database::new();
    let mut load = |family: &str| {
        let lower = EcoString::from(family.trim()).to_lowercase();
        if let Some(&success) = referenced.get(&lower) {
            return success;
        }

        // We load all variants for the family, since we don't know which will
        // be used.
        let mut success = false;
        for id in world.book().select_family(&lower) {
            if let Some(font) = world.font(id) {
                let source = Arc::new(font.data().clone());
                fontdb.load_font_source(fontdb::Source::Binary(source));
                success = true;
            }
        }

        referenced.insert(lower, success);
        success
    };

    // Load fallback family.
    if let Some(family) = fallback_family {
        load(family);
    }

    // Find out which font families are referenced by the SVG.
    traverse_svg(&tree.root, &mut |node| {
        let usvg::NodeKind::Text(text) = &mut *node.borrow_mut() else { return };
        for chunk in &mut text.chunks {
            for span in &mut chunk.spans {
                for family in &mut span.font.families {
                    if !load(family) {
                        let Some(fallback) = fallback_family else { continue };
                        *family = fallback.into();
                    }
                }
            }
        }
    });

    fontdb
}

/// Search for all font families referenced by an SVG.
fn traverse_svg<F>(node: &usvg::Node, f: &mut F)
where
    F: FnMut(&usvg::Node),
{
    f(node);
    for child in node.children() {
        traverse_svg(&child, f);
    }
}

/// Format the user-facing raster graphic decoding error message.
fn format_image_error(error: image::ImageError) -> EcoString {
    match error {
        image::ImageError::Limits(_) => "file is too large".into(),
        _ => "failed to decode image".into(),
    }
}

/// Format the user-facing SVG decoding error message.
fn format_usvg_error(error: usvg::Error) -> EcoString {
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
