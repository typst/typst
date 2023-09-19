//! Image handling.

use std::cell::RefCell;
use std::collections::{BTreeMap, HashMap, HashSet};
use std::fmt::{self, Debug, Formatter};
use std::io;
use std::rc::Rc;
use std::sync::Arc;

use comemo::{Prehashed, Track, Tracked};
use ecow::{eco_format, EcoString, EcoVec};
use image::codecs::gif::GifDecoder;
use image::codecs::jpeg::JpegDecoder;
use image::codecs::png::PngDecoder;
use image::io::Limits;
use image::{guess_format, ImageDecoder, ImageResult};
use typst_macros::{cast, Cast};
use usvg::{NodeExt, TreeParsing, TreeTextToPath};

use crate::diag::{bail, format_xml_like_error, StrResult};
use crate::eval::Bytes;
use crate::font::{Font, FontBook, FontInfo, FontVariant, FontWeight};
use crate::geom::Axes;
use crate::World;

/// A raster or vector image.
///
/// Values of this type are cheap to clone and hash.
#[derive(Clone, Hash, Eq, PartialEq)]
pub struct Image(Arc<Prehashed<Repr>>);

/// The internal representation.
#[derive(Hash)]
struct Repr {
    /// The raw, undecoded image data.
    data: Bytes,
    /// The format of the encoded `buffer`.
    format: ImageFormat,
    /// The size of the image.
    size: Axes<u32>,
    /// A loader for fonts referenced by an image (currently, only applies to
    /// SVG).
    loader: PreparedLoader,
    /// A text describing the image.
    alt: Option<EcoString>,
}

impl Image {
    /// Create an image from a buffer and a format.
    #[comemo::memoize]
    pub fn new(
        data: Bytes,
        format: ImageFormat,
        alt: Option<EcoString>,
    ) -> StrResult<Self> {
        let loader = PreparedLoader::default();
        let decoded = match format {
            ImageFormat::Raster(format) => decode_raster(&data, format)?,
            ImageFormat::Vector(VectorFormat::Svg) => {
                decode_svg(&data, (&loader as &dyn SvgFontLoader).track())?
            }
        };

        Ok(Self(Arc::new(Prehashed::new(Repr {
            data,
            format,
            size: decoded.size(),
            loader,
            alt,
        }))))
    }

    /// Create a font-dependant image from a buffer and a format.
    #[comemo::memoize]
    pub fn with_fonts(
        data: Bytes,
        format: ImageFormat,
        world: Tracked<dyn World + '_>,
        fallback_families: EcoVec<String>,
        alt: Option<EcoString>,
    ) -> StrResult<Self> {
        let loader = WorldLoader::new(world, fallback_families);
        let decoded = match format {
            ImageFormat::Raster(format) => decode_raster(&data, format)?,
            ImageFormat::Vector(VectorFormat::Svg) => {
                decode_svg(&data, (&loader as &dyn SvgFontLoader).track())?
            }
        };

        Ok(Self(Arc::new(Prehashed::new(Repr {
            data,
            format,
            size: decoded.size(),
            loader: loader.into_prepared(),
            alt,
        }))))
    }

    /// The raw image data.
    pub fn data(&self) -> &Bytes {
        &self.0.data
    }

    /// The format of the image.
    pub fn format(&self) -> ImageFormat {
        self.0.format
    }

    /// The size of the image in pixels.
    pub fn size(&self) -> Axes<u32> {
        self.0.size
    }

    /// The width of the image in pixels.
    pub fn width(&self) -> u32 {
        self.size().x
    }

    /// The height of the image in pixels.
    pub fn height(&self) -> u32 {
        self.size().y
    }

    /// A text describing the image.
    pub fn alt(&self) -> Option<&str> {
        self.0.alt.as_deref()
    }

    /// The decoded version of the image.
    pub fn decoded(&self) -> Rc<DecodedImage> {
        match self.format() {
            ImageFormat::Raster(format) => decode_raster(self.data(), format),
            ImageFormat::Vector(VectorFormat::Svg) => {
                decode_svg(self.data(), (&self.0.loader as &dyn SvgFontLoader).track())
            }
        }
        .unwrap()
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

/// A raster or vector image format.
#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
pub enum ImageFormat {
    /// A raster graphics format.
    Raster(RasterFormat),
    /// A vector graphics format.
    Vector(VectorFormat),
}

impl From<RasterFormat> for ImageFormat {
    fn from(format: RasterFormat) -> Self {
        Self::Raster(format)
    }
}

impl From<VectorFormat> for ImageFormat {
    fn from(format: VectorFormat) -> Self {
        Self::Vector(format)
    }
}

cast! {
    ImageFormat,
    self => match self {
        Self::Raster(v) => v.into_value(),
        Self::Vector(v) => v.into_value()
    },
    v: RasterFormat => Self::Raster(v),
    v: VectorFormat => Self::Vector(v),
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

/// A vector graphics format.
#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash, Cast)]
pub enum VectorFormat {
    /// The vector graphics format of the web.
    Svg,
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

/// A decoded image.
pub enum DecodedImage {
    /// A decoded pixel raster with its ICC profile.
    Raster(image::DynamicImage, Option<IccProfile>, RasterFormat),
    /// A decoded SVG tree.
    Svg(usvg::Tree),
}

impl DecodedImage {
    /// The size of the image in pixels.
    pub fn size(&self) -> Axes<u32> {
        Axes::new(self.width(), self.height())
    }

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
fn decode_raster(data: &Bytes, format: RasterFormat) -> StrResult<Rc<DecodedImage>> {
    fn decode_with<'a, T: ImageDecoder<'a>>(
        decoder: ImageResult<T>,
    ) -> ImageResult<(image::DynamicImage, Option<IccProfile>)> {
        let mut decoder = decoder?;
        let icc = decoder.icc_profile().filter(|data| !data.is_empty()).map(IccProfile);
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

    Ok(Rc::new(DecodedImage::Raster(dynamic, icc, format)))
}

/// Decode an SVG image.
#[comemo::memoize]
fn decode_svg(
    data: &Bytes,
    loader: Tracked<dyn SvgFontLoader + '_>,
) -> StrResult<Rc<DecodedImage>> {
    // Disable usvg's default to "Times New Roman". Instead, we default to
    // the empty family and later, when we traverse the SVG, we check for
    // empty and non-existing family names and replace them with the true
    // fallback family. This way, we can memoize SVG decoding with and without
    // fonts if the SVG does not contain text.
    let opts = usvg::Options { font_family: String::new(), ..Default::default() };
    let mut tree = usvg::Tree::from_data(data, &opts).map_err(format_usvg_error)?;
    if tree.has_text_nodes() {
        let fontdb = load_svg_fonts(&tree, loader);
        tree.convert_text(&fontdb);
    }
    Ok(Rc::new(DecodedImage::Svg(tree)))
}

/// A font family and its variants.
#[derive(Clone)]
struct FontData {
    /// The usvg-compatible font family name.
    usvg_family: EcoString,
    /// The font variants included in the family.
    fonts: EcoVec<Font>,
}

/// Discover and load the fonts referenced by an SVG.
fn load_svg_fonts(
    tree: &usvg::Tree,
    loader: Tracked<dyn SvgFontLoader + '_>,
) -> fontdb::Database {
    let mut fontdb = fontdb::Database::new();
    let mut font_cache = HashMap::<EcoString, Option<FontData>>::new();
    let mut loaded = HashSet::<EcoString>::new();

    // Loads a font family by its Typst name and returns its data.
    let mut load = |family: &str| -> Option<FontData> {
        let family = EcoString::from(family.trim()).to_lowercase();
        if let Some(success) = font_cache.get(&family) {
            return success.clone();
        }

        let fonts = loader.load(&family);
        let usvg_family = fonts.iter().find_map(|font| {
            font.find_name(ttf_parser::name_id::TYPOGRAPHIC_FAMILY)
                .or_else(|| font.find_name(ttf_parser::name_id::FAMILY))
                .map(Into::<EcoString>::into)
        });

        let font_data = usvg_family.map(|usvg_family| FontData { usvg_family, fonts });
        font_cache.insert(family, font_data.clone());
        font_data
    };

    // Loads a font family into the fontdb database.
    let mut load_into_db = |font_data: &FontData| {
        if loaded.contains(&font_data.usvg_family) {
            return;
        }

        // We load all variants for the family, since we don't know which will
        // be used.
        for font in &font_data.fonts {
            fontdb.load_font_data(font.data().to_vec());
        }

        loaded.insert(font_data.usvg_family.clone());
    };

    let fallback_families = loader.fallback_families();
    let fallback_fonts = fallback_families
        .iter()
        .filter_map(|family| load(family.as_str()))
        .collect::<EcoVec<_>>();

    // Determine the best font for each text node.
    traverse_svg(&tree.root, &mut |node| {
        let usvg::NodeKind::Text(text) = &mut *node.borrow_mut() else { return };
        for chunk in &mut text.chunks {
            for span in &mut chunk.spans {
                let Some(text) = chunk.text.get(span.start..span.end) else { continue };

                let inline_families = &span.font.families;
                let inline_fonts = inline_families
                    .iter()
                    .filter(|family| !family.is_empty())
                    .filter_map(|family| load(family.as_str()))
                    .collect::<EcoVec<_>>();

                // Find a font that covers all characters in the span while
                // taking the fallback order into account.
                let font =
                    inline_fonts.iter().chain(fallback_fonts.iter()).find(|font_data| {
                        font_data.fonts.iter().any(|font| {
                            text.chars().all(|c| font.info().coverage.contains(c as u32))
                        })
                    });

                if let Some(font) = font {
                    load_into_db(font);
                    span.font.families = vec![font.usvg_family.to_string()];
                } else if !fallback_families.is_empty() {
                    // If no font covers all characters, use last resort fallback
                    // (only if fallback is enabled <=> fallback_families is not empty)
                    let like = inline_fonts
                        .first()
                        .or(fallback_fonts.first())
                        .and_then(|font_data| font_data.fonts.first())
                        .map(|font| font.info().clone());

                    let variant = FontVariant {
                        style: span.font.style.into(),
                        weight: FontWeight::from_number(span.font.weight),
                        stretch: span.font.stretch.into(),
                    };

                    let fallback = loader
                        .find_fallback(text, like, variant)
                        .and_then(|family| load(family.as_str()));

                    if let Some(font) = fallback {
                        load_into_db(&font);
                        span.font.families = vec![font.usvg_family.to_string()];
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
    for descendant in node.descendants() {
        f(&descendant);
        descendant.subroots(|subroot| traverse_svg(&subroot, f))
    }
}

/// Interface for loading fonts for an SVG.
///
/// Can be backed by a `WorldLoader` or a `PreparedLoader`. The first is used
/// when the image is initially decoded. It records all required fonts and
/// produces a `PreparedLoader` from it. This loader can then be used to
/// redecode the image with a cache hit from the initial decoding. This way, we
/// can cheaply access the decoded version of an image.
///
/// The alternative would be to store the decoded image directly in the image,
/// but that would make `Image` not `Send` because `usvg::Tree` is not `Send`.
/// The current design also has the added benefit that large decoded images can
/// be evicted if they are not used anymore.
#[comemo::track]
trait SvgFontLoader {
    /// Load all fonts for the given lowercased font family.
    fn load(&self, family: &str) -> EcoVec<Font>;

    /// Prioritized sequence of fallback font families.
    fn fallback_families(&self) -> &[String];

    /// Find a last resort fallback for a given text and font variant.
    fn find_fallback(
        &self,
        text: &str,
        like: Option<FontInfo>,
        font: FontVariant,
    ) -> Option<EcoString>;
}

/// Loads fonts for an SVG from a world
struct WorldLoader<'a> {
    world: Tracked<'a, dyn World + 'a>,
    seen: RefCell<BTreeMap<EcoString, EcoVec<Font>>>,
    fallback_families: EcoVec<String>,
}

impl<'a> WorldLoader<'a> {
    fn new(
        world: Tracked<'a, dyn World + 'a>,
        fallback_families: EcoVec<String>,
    ) -> Self {
        Self { world, seen: Default::default(), fallback_families }
    }

    fn into_prepared(self) -> PreparedLoader {
        let fonts = self.seen.into_inner().into_values().flatten().collect::<EcoVec<_>>();
        PreparedLoader {
            book: FontBook::from_fonts(fonts.iter()),
            fonts,
            fallback_families: self.fallback_families,
        }
    }
}

impl SvgFontLoader for WorldLoader<'_> {
    fn load(&self, family: &str) -> EcoVec<Font> {
        self.seen
            .borrow_mut()
            .entry(family.into())
            .or_insert_with(|| {
                self.world
                    .book()
                    .select_family(family)
                    .filter_map(|id| self.world.font(id))
                    .collect()
            })
            .clone()
    }

    fn fallback_families(&self) -> &[String] {
        self.fallback_families.as_slice()
    }

    fn find_fallback(
        &self,
        text: &str,
        like: Option<FontInfo>,
        variant: FontVariant,
    ) -> Option<EcoString> {
        self.world
            .book()
            .select_fallback(like.as_ref(), variant, text)
            .and_then(|id| self.world.font(id))
            .map(|font| font.info().family.to_lowercase().as_str().into())
    }
}

/// Loads fonts for an SVG from a prepared list.
#[derive(Default, Hash)]
struct PreparedLoader {
    book: FontBook,
    fonts: EcoVec<Font>,
    fallback_families: EcoVec<String>,
}

impl SvgFontLoader for PreparedLoader {
    fn load(&self, family: &str) -> EcoVec<Font> {
        self.book
            .select_family(family)
            .filter_map(|id| self.fonts.get(id))
            .cloned()
            .collect()
    }

    fn fallback_families(&self) -> &[String] {
        self.fallback_families.as_slice()
    }

    fn find_fallback(
        &self,
        text: &str,
        like: Option<FontInfo>,
        variant: FontVariant,
    ) -> Option<EcoString> {
        self.book
            .select_fallback(like.as_ref(), variant, text)
            .and_then(|id| self.fonts.get(id))
            .map(|font| font.info().family.to_lowercase().as_str().into())
    }
}

/// Format the user-facing raster graphic decoding error message.
fn format_image_error(error: image::ImageError) -> EcoString {
    match error {
        image::ImageError::Limits(_) => "file is too large".into(),
        err => eco_format!("failed to decode image ({err})"),
    }
}

/// Format the user-facing SVG decoding error message.
fn format_usvg_error(error: usvg::Error) -> EcoString {
    match error {
        usvg::Error::NotAnUtf8Str => "file is not valid utf-8".into(),
        usvg::Error::MalformedGZip => "file is not compressed correctly".into(),
        usvg::Error::ElementsLimitReached => "file is too large".into(),
        usvg::Error::InvalidSize => {
            "failed to parse SVG (width, height, or viewbox is invalid)".into()
        }
        usvg::Error::ParsingFailed(error) => format_xml_like_error("SVG", error),
    }
}
