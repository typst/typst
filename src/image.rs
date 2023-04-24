//! Image handling.

use std::collections::BTreeSet;
use std::fmt::{self, Debug, Formatter};
use std::hash::{Hash, Hasher};
use std::io;
use std::sync::Arc;

use comemo::Tracked;
use ecow::EcoString;

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
        world: Tracked<dyn World>,
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
    /// A decoded pixel raster.
    Raster(image::DynamicImage, RasterFormat),
    /// An decoded SVG tree.
    Svg(usvg::Tree),
}

impl DecodedImage {
    /// The width of the image in pixels.
    pub fn width(&self) -> u32 {
        match self {
            Self::Raster(dynamic, _) => dynamic.width(),
            Self::Svg(tree) => tree.svg_node().size.width().ceil() as u32,
        }
    }

    /// The height of the image in pixels.
    pub fn height(&self) -> u32 {
        match self {
            Self::Raster(dynamic, _) => dynamic.height(),
            Self::Svg(tree) => tree.svg_node().size.height().ceil() as u32,
        }
    }
}

/// Decode a raster image.
#[comemo::memoize]
fn decode_raster(data: &Buffer, format: RasterFormat) -> StrResult<Arc<DecodedImage>> {
    let cursor = io::Cursor::new(&data);
    let reader = image::io::Reader::with_format(cursor, format.into());
    let dynamic = reader.decode().map_err(format_image_error)?;
    Ok(Arc::new(DecodedImage::Raster(dynamic, format)))
}

/// Decode an SVG image.
#[comemo::memoize]
fn decode_svg(data: &Buffer) -> StrResult<Arc<DecodedImage>> {
    let opts = usvg::Options::default();
    let tree = usvg::Tree::from_data(data, &opts.to_ref()).map_err(format_usvg_error)?;
    Ok(Arc::new(DecodedImage::Svg(tree)))
}

/// Decode an SVG image with access to fonts.
#[comemo::memoize]
fn decode_svg_with_fonts(
    data: &Buffer,
    world: Tracked<dyn World>,
    fallback_family: Option<&str>,
) -> StrResult<Arc<DecodedImage>> {
    // Parse XML.
    let xml = std::str::from_utf8(data)
        .map_err(|_| format_usvg_error(usvg::Error::NotAnUtf8Str))?;
    let document = roxmltree::Document::parse(xml)
        .map_err(|err| format_xml_like_error("svg", err))?;

    // Parse SVG.
    let mut opts = usvg::Options {
        fontdb: load_svg_fonts(&document, world, fallback_family),
        ..Default::default()
    };

    // Recover the non-lowercased version of the family because
    // usvg is case sensitive.
    let book = world.book();
    if let Some(family) = fallback_family
        .and_then(|lowercase| book.select_family(lowercase).next())
        .and_then(|index| book.info(index))
        .map(|info| info.family.clone())
    {
        opts.font_family = family;
    }

    let tree =
        usvg::Tree::from_xmltree(&document, &opts.to_ref()).map_err(format_usvg_error)?;

    Ok(Arc::new(DecodedImage::Svg(tree)))
}

/// Discover and load the fonts referenced by an SVG.
fn load_svg_fonts(
    document: &roxmltree::Document,
    world: Tracked<dyn World>,
    fallback_family: Option<&str>,
) -> fontdb::Database {
    // Find out which font families are referenced by the SVG. We simply do a
    // search for `font-family` attributes. This won't help with CSS, but usvg
    // 22.0 doesn't seem to support it anyway. Once we bump to the latest usvg,
    // this can be replaced by a scan for text elements in the SVG:
    // https://github.com/RazrFalcon/resvg/issues/555
    let mut referenced = BTreeSet::<EcoString>::new();
    traverse_xml(&document.root(), &mut |node| {
        if let Some(list) = node.attribute("font-family") {
            for family in list.split(',') {
                referenced.insert(EcoString::from(family.trim()).to_lowercase());
            }
        }
    });

    // Prepare font database.
    let mut fontdb = fontdb::Database::new();
    for family in referenced.iter().map(|family| family.as_str()).chain(fallback_family) {
        // We load all variants for the family, since we don't know which will
        // be used.
        for id in world.book().select_family(family) {
            if let Some(font) = world.font(id) {
                let source = Arc::new(font.data().clone());
                fontdb.load_font_source(fontdb::Source::Binary(source));
            }
        }
    }

    fontdb
}

/// Search for all font families referenced by an SVG.
fn traverse_xml<F>(node: &roxmltree::Node, f: &mut F)
where
    F: FnMut(&roxmltree::Node),
{
    f(node);
    for child in node.children() {
        traverse_xml(&child, f);
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
