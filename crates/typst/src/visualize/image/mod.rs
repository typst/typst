//! Image handling.

mod raster;
mod svg;

pub use self::raster::{RasterFormat, RasterImage};
pub use self::svg::SvgImage;

use std::ffi::OsStr;
use std::fmt::{self, Debug, Formatter};
use std::sync::Arc;

use comemo::Tracked;
use ecow::EcoString;

use crate::diag::{bail, At, SourceResult, StrResult};
use crate::engine::Engine;
use crate::foundations::{
    cast, elem, func, scope, Bytes, Cast, Content, NativeElement, Packed, Resolve, Smart,
    StyleChain,
};
use crate::layout::{
    Abs, Axes, FixedAlignment, Frame, FrameItem, LayoutSingle, Length, Point, Regions,
    Rel, Size,
};
use crate::loading::Readable;
use crate::model::Figurable;
use crate::syntax::{Span, Spanned};
use crate::text::{families, Lang, LocalName, Region};
use crate::util::{option_eq, LazyHash};
use crate::visualize::Path;
use crate::World;

/// A raster or vector graphic.
///
/// Supported formats are PNG, JPEG, GIF and SVG.
///
/// _Note:_ Work on SVG export is ongoing and there might be visual inaccuracies
/// in the resulting PDF. Make sure to double-check embedded SVG images. If you
/// have an issue, also feel free to report it on [GitHub][gh-svg].
///
/// # Example
/// ```example
/// #figure(
///   image("molecular.jpg", width: 80%),
///   caption: [
///     A step in the molecular testing
///     pipeline of our lab.
///   ],
/// )
/// ```
///
/// [gh-svg]: https://github.com/typst/typst/issues?q=is%3Aopen+is%3Aissue+label%3Asvg
#[elem(scope, LayoutSingle, LocalName, Figurable)]
pub struct ImageElem {
    /// Path to an image file.
    #[required]
    #[parse(
        let Spanned { v: path, span } =
            args.expect::<Spanned<EcoString>>("path to image file")?;
        let id = span.resolve_path(&path).at(span)?;
        let data = engine.world.file(id).at(span)?;
        path
    )]
    #[borrowed]
    pub path: EcoString,

    /// The raw file data.
    #[internal]
    #[required]
    #[parse(Readable::Bytes(data))]
    pub data: Readable,

    /// The image's format. Detected automatically by default.
    pub format: Smart<ImageFormat>,

    /// The width of the image.
    pub width: Smart<Rel<Length>>,

    /// The height of the image.
    pub height: Smart<Rel<Length>>,

    /// A text describing the image.
    pub alt: Option<EcoString>,

    /// How the image should adjust itself to a given area (the area is defined
    /// by the `width` and `height` fields). Note that `fit` doesn't visually
    /// change anything if the area's aspect ratio is the same as the image's
    /// one.
    ///
    /// ```example
    /// #set page(width: 300pt, height: 50pt, margin: 10pt)
    /// #image("tiger.jpg", width: 100%, fit: "cover")
    /// #image("tiger.jpg", width: 100%, fit: "contain")
    /// #image("tiger.jpg", width: 100%, fit: "stretch")
    /// ```
    #[default(ImageFit::Cover)]
    pub fit: ImageFit,
}

#[scope]
impl ImageElem {
    /// Decode a raster or vector graphic from bytes or a string.
    ///
    /// ```example
    /// #let original = read("diagram.svg")
    /// #let changed = original.replace(
    ///   "#2B80FF", // blue
    ///   green.to-hex(),
    /// )
    ///
    /// #image.decode(original)
    /// #image.decode(changed)
    /// ```
    #[func(title = "Decode Image")]
    pub fn decode(
        /// The call span of this function.
        span: Span,
        /// The data to decode as an image. Can be a string for SVGs.
        data: Readable,
        /// The image's format. Detected automatically by default.
        #[named]
        format: Option<Smart<ImageFormat>>,
        /// The width of the image.
        #[named]
        width: Option<Smart<Rel<Length>>>,
        /// The height of the image.
        #[named]
        height: Option<Smart<Rel<Length>>>,
        /// A text describing the image.
        #[named]
        alt: Option<Option<EcoString>>,
        /// How the image should adjust itself to a given area.
        #[named]
        fit: Option<ImageFit>,
    ) -> StrResult<Content> {
        let mut elem = ImageElem::new(EcoString::new(), data);
        if let Some(format) = format {
            elem.push_format(format);
        }
        if let Some(width) = width {
            elem.push_width(width);
        }
        if let Some(height) = height {
            elem.push_height(height);
        }
        if let Some(alt) = alt {
            elem.push_alt(alt);
        }
        if let Some(fit) = fit {
            elem.push_fit(fit);
        }
        Ok(elem.pack().spanned(span))
    }
}

impl LayoutSingle for Packed<ImageElem> {
    #[typst_macros::time(name = "image", span = self.span())]
    fn layout(
        &self,
        engine: &mut Engine,
        styles: StyleChain,
        regions: Regions,
    ) -> SourceResult<Frame> {
        // Take the format that was explicitly defined, or parse the extension,
        // or try to detect the format.
        let data = self.data();
        let format = match self.format(styles) {
            Smart::Custom(v) => v,
            Smart::Auto => {
                let ext = std::path::Path::new(self.path().as_str())
                    .extension()
                    .and_then(OsStr::to_str)
                    .unwrap_or_default()
                    .to_lowercase();

                match ext.as_str() {
                    "png" => ImageFormat::Raster(RasterFormat::Png),
                    "jpg" | "jpeg" => ImageFormat::Raster(RasterFormat::Jpg),
                    "gif" => ImageFormat::Raster(RasterFormat::Gif),
                    "svg" | "svgz" => ImageFormat::Vector(VectorFormat::Svg),
                    _ => match &data {
                        Readable::Str(_) => ImageFormat::Vector(VectorFormat::Svg),
                        Readable::Bytes(bytes) => match RasterFormat::detect(bytes) {
                            Some(f) => ImageFormat::Raster(f),
                            None => bail!(self.span(), "unknown image format"),
                        },
                    },
                }
            }
        };

        let image = Image::with_fonts(
            data.clone().into(),
            format,
            self.alt(styles),
            engine.world,
            &families(styles).map(|s| s.into()).collect::<Vec<_>>(),
        )
        .at(self.span())?;

        let sizing = Axes::new(self.width(styles), self.height(styles));
        let region = sizing
            .zip_map(regions.base(), |s, r| s.map(|v| v.resolve(styles).relative_to(r)))
            .unwrap_or(regions.base());

        let expand = sizing.as_ref().map(Smart::is_custom) | regions.expand;
        let region_ratio = region.x / region.y;

        // Find out whether the image is wider or taller than the target size.
        let pxw = image.width();
        let pxh = image.height();
        let px_ratio = pxw / pxh;
        let wide = px_ratio > region_ratio;

        // The space into which the image will be placed according to its fit.
        let target = if expand.x && expand.y {
            // If both width and height are forced, take them.
            region
        } else if expand.x {
            // If just width is forced, take it.
            Size::new(region.x, region.y.min(region.x / px_ratio))
        } else if expand.y {
            // If just height is forced, take it.
            Size::new(region.x.min(region.y * px_ratio), region.y)
        } else {
            // If neither is forced, take the natural image size at the image's
            // DPI bounded by the available space.
            let dpi = image.dpi().unwrap_or(Image::DEFAULT_DPI);
            let natural = Axes::new(pxw, pxh).map(|v| Abs::inches(v / dpi));
            Size::new(
                natural.x.min(region.x).min(region.y * px_ratio),
                natural.y.min(region.y).min(region.x / px_ratio),
            )
        };

        // Compute the actual size of the fitted image.
        let fit = self.fit(styles);
        let fitted = match fit {
            ImageFit::Cover | ImageFit::Contain => {
                if wide == (fit == ImageFit::Contain) {
                    Size::new(target.x, target.x / px_ratio)
                } else {
                    Size::new(target.y * px_ratio, target.y)
                }
            }
            ImageFit::Stretch => target,
        };

        // First, place the image in a frame of exactly its size and then resize
        // the frame to the target size, center aligning the image in the
        // process.
        let mut frame = Frame::soft(fitted);
        frame.push(Point::zero(), FrameItem::Image(image, fitted, self.span()));
        frame.resize(target, Axes::splat(FixedAlignment::Center));

        // Create a clipping group if only part of the image should be visible.
        if fit == ImageFit::Cover && !target.fits(fitted) {
            frame.clip(Path::rect(frame.size()));
        }

        Ok(frame)
    }
}

impl LocalName for Packed<ImageElem> {
    fn local_name(lang: Lang, region: Option<Region>) -> &'static str {
        match lang {
            Lang::ALBANIAN => "Figurë",
            Lang::ARABIC => "شكل",
            Lang::BOKMÅL => "Figur",
            Lang::CATALAN => "Figura",
            Lang::CHINESE if option_eq(region, "TW") => "圖",
            Lang::CHINESE => "图",
            Lang::CZECH => "Obrázek",
            Lang::DANISH => "Figur",
            Lang::DUTCH => "Figuur",
            Lang::ESTONIAN => "Joonis",
            Lang::FILIPINO => "Pigura",
            Lang::FINNISH => "Kuva",
            Lang::FRENCH => "Fig.",
            Lang::GERMAN => "Abbildung",
            Lang::GREEK => "Σχήμα",
            Lang::HUNGARIAN => "Ábra",
            Lang::ITALIAN => "Figura",
            Lang::NYNORSK => "Figur",
            Lang::POLISH => "Rysunek",
            Lang::PORTUGUESE => "Figura",
            Lang::ROMANIAN => "Figura",
            Lang::RUSSIAN => "Рис.",
            Lang::SERBIAN => "Слика",
            Lang::SLOVENIAN => "Slika",
            Lang::SPANISH => "Figura",
            Lang::SWEDISH => "Figur",
            Lang::TURKISH => "Şekil",
            Lang::UKRAINIAN => "Рисунок",
            Lang::VIETNAMESE => "Hình",
            Lang::JAPANESE => "図",
            Lang::ENGLISH | _ => "Figure",
        }
    }
}

impl Figurable for Packed<ImageElem> {}

/// How an image should adjust itself to a given area,
#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash, Cast)]
pub enum ImageFit {
    /// The image should completely cover the area (preserves aspect ratio by
    /// cropping the image only horizontally or vertically). This is the
    /// default.
    Cover,
    /// The image should be fully contained in the area (preserves aspect
    /// ratio; doesn't crop the image; one dimension can be narrower than
    /// specified).
    Contain,
    /// The image should be stretched so that it exactly fills the area, even if
    /// this means that the image will be distorted (doesn't preserve aspect
    /// ratio and doesn't crop the image).
    Stretch,
}

/// A loaded raster or vector image.
///
/// Values of this type are cheap to clone and hash.
#[derive(Clone, Hash, Eq, PartialEq)]
pub struct Image(Arc<LazyHash<Repr>>);

/// The internal representation.
#[derive(Hash)]
struct Repr {
    /// The raw, undecoded image data.
    kind: ImageKind,
    /// A text describing the image.
    alt: Option<EcoString>,
}

/// A kind of image.
#[derive(Hash)]
pub enum ImageKind {
    /// A raster image.
    Raster(RasterImage),
    /// An SVG image.
    Svg(SvgImage),
}

impl Image {
    /// When scaling an image to it's natural size, we default to this DPI
    /// if the image doesn't contain DPI metadata.
    pub const DEFAULT_DPI: f64 = 72.0;

    /// Create an image from a buffer and a format.
    #[comemo::memoize]
    #[typst_macros::time(name = "load image")]
    pub fn new(
        data: Bytes,
        format: ImageFormat,
        alt: Option<EcoString>,
    ) -> StrResult<Image> {
        let kind = match format {
            ImageFormat::Raster(format) => {
                ImageKind::Raster(RasterImage::new(data, format)?)
            }
            ImageFormat::Vector(VectorFormat::Svg) => {
                ImageKind::Svg(SvgImage::new(data)?)
            }
        };

        Ok(Self(Arc::new(LazyHash::new(Repr { kind, alt }))))
    }

    /// Create a possibly font-dependant image from a buffer and a format.
    #[comemo::memoize]
    #[typst_macros::time(name = "load image")]
    pub fn with_fonts(
        data: Bytes,
        format: ImageFormat,
        alt: Option<EcoString>,
        world: Tracked<dyn World + '_>,
        families: &[String],
    ) -> StrResult<Image> {
        let kind = match format {
            ImageFormat::Raster(format) => {
                ImageKind::Raster(RasterImage::new(data, format)?)
            }
            ImageFormat::Vector(VectorFormat::Svg) => {
                ImageKind::Svg(SvgImage::with_fonts(data, world, families)?)
            }
        };

        Ok(Self(Arc::new(LazyHash::new(Repr { kind, alt }))))
    }

    /// The raw image data.
    pub fn data(&self) -> &Bytes {
        match &self.0.kind {
            ImageKind::Raster(raster) => raster.data(),
            ImageKind::Svg(svg) => svg.data(),
        }
    }

    /// The format of the image.
    pub fn format(&self) -> ImageFormat {
        match &self.0.kind {
            ImageKind::Raster(raster) => raster.format().into(),
            ImageKind::Svg(_) => VectorFormat::Svg.into(),
        }
    }

    /// The width of the image in pixels.
    pub fn width(&self) -> f64 {
        match &self.0.kind {
            ImageKind::Raster(raster) => raster.width() as f64,
            ImageKind::Svg(svg) => svg.width(),
        }
    }

    /// The height of the image in pixels.
    pub fn height(&self) -> f64 {
        match &self.0.kind {
            ImageKind::Raster(raster) => raster.height() as f64,
            ImageKind::Svg(svg) => svg.height(),
        }
    }

    /// The image's pixel density in pixels per inch, if known.
    pub fn dpi(&self) -> Option<f64> {
        match &self.0.kind {
            ImageKind::Raster(raster) => raster.dpi(),
            ImageKind::Svg(_) => None,
        }
    }

    /// A text describing the image.
    pub fn alt(&self) -> Option<&str> {
        self.0.alt.as_deref()
    }

    /// The decoded image.
    pub fn kind(&self) -> &ImageKind {
        &self.0.kind
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

/// A vector graphics format.
#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash, Cast)]
pub enum VectorFormat {
    /// The vector graphics format of the web.
    Svg,
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
