//! Image handling.

mod raster;
mod svg;

pub use self::raster::{
    ExchangeFormat, PixelEncoding, PixelFormat, RasterFormat, RasterImage,
};
pub use self::svg::SvgImage;

use std::ffi::OsStr;
use std::fmt::{self, Debug, Formatter};
use std::sync::Arc;

use ecow::EcoString;
use typst_syntax::{Span, Spanned};
use typst_utils::LazyHash;

use crate::diag::{warning, At, LoadedWithin, SourceResult, StrResult};
use crate::engine::Engine;
use crate::foundations::{
    cast, elem, func, scope, Bytes, Cast, Content, Derived, NativeElement, Packed, Smart,
    StyleChain,
};
use crate::layout::{Length, Rel, Sizing};
use crate::loading::{DataSource, Load, LoadSource, Loaded, Readable};
use crate::model::Figurable;
use crate::text::{families, LocalName};

/// A raster or vector graphic.
///
/// You can wrap the image in a [`figure`] to give it a number and caption.
///
/// Like most elements, images are _block-level_ by default and thus do not
/// integrate themselves into adjacent paragraphs. To force an image to become
/// inline, put it into a [`box`].
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
#[elem(scope, LocalName, Figurable)]
pub struct ImageElem {
    /// A [path]($syntax/#paths) to an image file or raw bytes making up an
    /// image in one of the supported [formats]($image.format).
    ///
    /// Bytes can be used to specify raw pixel data in a row-major,
    /// left-to-right, top-to-bottom format.
    ///
    /// ```example
    /// #let original = read("diagram.svg")
    /// #let changed = original.replace(
    ///   "#2B80FF", // blue
    ///   green.to-hex(),
    /// )
    ///
    /// #image(bytes(original))
    /// #image(bytes(changed))
    /// ```
    #[required]
    #[parse(
        let source = args.expect::<Spanned<DataSource>>("source")?;
        let loaded = source.load(engine.world)?;
        Derived::new(source.v, loaded)
    )]
    pub source: Derived<DataSource, Loaded>,

    /// The image's format.
    ///
    /// By default, the format is detected automatically. Typically, you thus
    /// only need to specify this when providing raw bytes as the
    /// [`source`]($image.source) (even then, Typst will try to figure out the
    /// format automatically, but that's not always possible).
    ///
    /// Supported formats are `{"png"}`, `{"jpg"}`, `{"gif"}`, `{"svg"}`,
    /// `{"webp"}` as well as raw pixel data. Embedding PDFs as images is
    /// [not currently supported](https://github.com/typst/typst/issues/145).
    ///
    /// When providing raw pixel data as the `source`, you must specify a
    /// dictionary with the following keys as the `format`:
    /// - `encoding` ([str]): The encoding of the pixel data. One of:
    ///   - `{"rgb8"}` (three 8-bit channels: red, green, blue)
    ///   - `{"rgba8"}` (four 8-bit channels: red, green, blue, alpha)
    ///   - `{"luma8"}` (one 8-bit channel)
    ///   - `{"lumaa8"}` (two 8-bit channels: luma and alpha)
    /// - `width` ([int]): The pixel width of the image.
    /// - `height` ([int]): The pixel height of the image.
    ///
    /// The pixel width multiplied by the height multiplied by the channel count
    /// for the specified encoding must then match the `source` data.
    ///
    /// ```example
    /// #image(
    ///   read(
    ///     "tetrahedron.svg",
    ///     encoding: none,
    ///   ),
    ///   format: "svg",
    ///   width: 2cm,
    /// )
    ///
    /// #image(
    ///   bytes(range(16).map(x => x * 16)),
    ///   format: (
    ///     encoding: "luma8",
    ///     width: 4,
    ///     height: 4,
    ///   ),
    ///   width: 2cm,
    /// )
    /// ```
    pub format: Smart<ImageFormat>,

    /// The width of the image.
    pub width: Smart<Rel<Length>>,

    /// The height of the image.
    pub height: Sizing,

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

    /// A hint to viewers how they should scale the image.
    ///
    /// When set to `{auto}`, the default is left up to the viewer. For PNG
    /// export, Typst will default to smooth scaling, like most PDF and SVG
    /// viewers.
    ///
    /// _Note:_ The exact look may differ across PDF viewers.
    pub scaling: Smart<ImageScaling>,

    /// An ICC profile for the image.
    ///
    /// ICC profiles define how to interpret the colors in an image. When set
    /// to `{auto}`, Typst will try to extract an ICC profile from the image.
    #[parse(match args.named::<Spanned<Smart<DataSource>>>("icc")? {
        Some(Spanned { v: Smart::Custom(source), span }) => Some(Smart::Custom({
            let loaded = Spanned::new(&source, span).load(engine.world)?;
            Derived::new(source, loaded.data)
        })),
        Some(Spanned { v: Smart::Auto, .. }) => Some(Smart::Auto),
        None => None,
    })]
    pub icc: Smart<Derived<DataSource, Bytes>>,
}

#[scope]
#[allow(clippy::too_many_arguments)]
impl ImageElem {
    /// Decode a raster or vector graphic from bytes or a string.
    #[func(title = "Decode Image")]
    #[deprecated = "`image.decode` is deprecated, directly pass bytes to `image` instead"]
    pub fn decode(
        span: Span,
        /// The data to decode as an image. Can be a string for SVGs.
        data: Spanned<Readable>,
        /// The image's format. Detected automatically by default.
        #[named]
        format: Option<Smart<ImageFormat>>,
        /// The width of the image.
        #[named]
        width: Option<Smart<Rel<Length>>>,
        /// The height of the image.
        #[named]
        height: Option<Sizing>,
        /// A text describing the image.
        #[named]
        alt: Option<Option<EcoString>>,
        /// How the image should adjust itself to a given area.
        #[named]
        fit: Option<ImageFit>,
        /// A hint to viewers how they should scale the image.
        #[named]
        scaling: Option<Smart<ImageScaling>>,
    ) -> StrResult<Content> {
        let bytes = data.v.into_bytes();
        let loaded =
            Loaded::new(Spanned::new(LoadSource::Bytes, data.span), bytes.clone());
        let source = Derived::new(DataSource::Bytes(bytes), loaded);
        let mut elem = ImageElem::new(source);
        if let Some(format) = format {
            elem.format.set(format);
        }
        if let Some(width) = width {
            elem.width.set(width);
        }
        if let Some(height) = height {
            elem.height.set(height);
        }
        if let Some(alt) = alt {
            elem.alt.set(alt);
        }
        if let Some(fit) = fit {
            elem.fit.set(fit);
        }
        if let Some(scaling) = scaling {
            elem.scaling.set(scaling);
        }
        Ok(elem.pack().spanned(span))
    }
}

impl Packed<ImageElem> {
    /// Decodes the image.
    pub fn decode(&self, engine: &mut Engine, styles: StyleChain) -> SourceResult<Image> {
        let span = self.span();
        let loaded = &self.source.derived;
        let format = self.determine_format(styles).at(span)?;

        // Warn the user if the image contains a foreign object. Not perfect
        // because the svg could also be encoded, but that's an edge case.
        if format == ImageFormat::Vector(VectorFormat::Svg) {
            let has_foreign_object =
                memchr::memmem::find(&loaded.data, b"<foreignObject").is_some();

            if has_foreign_object {
                engine.sink.warn(warning!(
                span,
                "image contains foreign object";
                hint: "SVG images with foreign objects might render incorrectly in typst";
                hint: "see https://github.com/typst/typst/issues/1421 for more information"
            ));
            }
        }

        // Construct the image itself.
        let kind = match format {
            ImageFormat::Raster(format) => ImageKind::Raster(
                RasterImage::new(
                    loaded.data.clone(),
                    format,
                    self.icc.get_ref(styles).as_ref().map(|icc| icc.derived.clone()),
                )
                .at(span)?,
            ),
            ImageFormat::Vector(VectorFormat::Svg) => ImageKind::Svg(
                SvgImage::with_fonts(
                    loaded.data.clone(),
                    engine.world,
                    &families(styles).map(|f| f.as_str()).collect::<Vec<_>>(),
                )
                .within(loaded)?,
            ),
        };

        Ok(Image::new(kind, self.alt.get_cloned(styles), self.scaling.get(styles)))
    }

    /// Tries to determine the image format based on the format that was
    /// explicitly defined, or else the extension, or else the data.
    fn determine_format(&self, styles: StyleChain) -> StrResult<ImageFormat> {
        if let Smart::Custom(v) = self.format.get(styles) {
            return Ok(v);
        };

        let Derived { source, derived: loaded } = &self.source;
        if let DataSource::Path(path) = source {
            let ext = std::path::Path::new(path.as_str())
                .extension()
                .and_then(OsStr::to_str)
                .unwrap_or_default()
                .to_lowercase();

            match ext.as_str() {
                "png" => return Ok(ExchangeFormat::Png.into()),
                "jpg" | "jpeg" => return Ok(ExchangeFormat::Jpg.into()),
                "gif" => return Ok(ExchangeFormat::Gif.into()),
                "svg" | "svgz" => return Ok(VectorFormat::Svg.into()),
                "webp" => return Ok(ExchangeFormat::Webp.into()),
                _ => {}
            }
        }

        Ok(ImageFormat::detect(&loaded.data).ok_or("unknown image format")?)
    }
}

impl LocalName for Packed<ImageElem> {
    const KEY: &'static str = "figure";
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
    /// The scaling algorithm to use.
    scaling: Smart<ImageScaling>,
}

impl Image {
    /// When scaling an image to it's natural size, we default to this DPI
    /// if the image doesn't contain DPI metadata.
    pub const DEFAULT_DPI: f64 = 72.0;

    /// Should always be the same as the default DPI used by usvg.
    pub const USVG_DEFAULT_DPI: f64 = 96.0;

    /// Create an image from a `RasterImage` or `SvgImage`.
    pub fn new(
        kind: impl Into<ImageKind>,
        alt: Option<EcoString>,
        scaling: Smart<ImageScaling>,
    ) -> Self {
        Self::new_impl(kind.into(), alt, scaling)
    }

    /// Create an image with optional properties set to the default.
    pub fn plain(kind: impl Into<ImageKind>) -> Self {
        Self::new(kind, None, Smart::Auto)
    }

    /// The internal, non-generic implementation. This is memoized to reuse
    /// the `Arc` and `LazyHash`.
    #[comemo::memoize]
    fn new_impl(
        kind: ImageKind,
        alt: Option<EcoString>,
        scaling: Smart<ImageScaling>,
    ) -> Image {
        Self(Arc::new(LazyHash::new(Repr { kind, alt, scaling })))
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
            ImageKind::Svg(_) => Some(Image::USVG_DEFAULT_DPI),
        }
    }

    /// A text describing the image.
    pub fn alt(&self) -> Option<&str> {
        self.0.alt.as_deref()
    }

    /// The image scaling algorithm to use for this image.
    pub fn scaling(&self) -> Smart<ImageScaling> {
        self.0.scaling
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
            .field("scaling", &self.scaling())
            .finish()
    }
}

/// A kind of image.
#[derive(Clone, Hash)]
pub enum ImageKind {
    /// A raster image.
    Raster(RasterImage),
    /// An SVG image.
    Svg(SvgImage),
}

impl From<RasterImage> for ImageKind {
    fn from(image: RasterImage) -> Self {
        Self::Raster(image)
    }
}

impl From<SvgImage> for ImageKind {
    fn from(image: SvgImage) -> Self {
        Self::Svg(image)
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

impl ImageFormat {
    /// Try to detect the format of an image from data.
    pub fn detect(data: &[u8]) -> Option<Self> {
        if let Some(format) = ExchangeFormat::detect(data) {
            return Some(Self::Raster(RasterFormat::Exchange(format)));
        }

        if is_svg(data) {
            return Some(Self::Vector(VectorFormat::Svg));
        }

        None
    }
}

/// Checks whether the data looks like an SVG or a compressed SVG.
fn is_svg(data: &[u8]) -> bool {
    // Check for the gzip magic bytes. This check is perhaps a bit too
    // permissive as other formats than SVGZ could use gzip.
    if data.starts_with(&[0x1f, 0x8b]) {
        return true;
    }

    // If the first 2048 bytes contain the SVG namespace declaration, we assume
    // that it's an SVG. Note that, if the SVG does not contain a namespace
    // declaration, usvg will reject it.
    let head = &data[..data.len().min(2048)];
    memchr::memmem::find(head, b"http://www.w3.org/2000/svg").is_some()
}

/// A vector graphics format.
#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash, Cast)]
pub enum VectorFormat {
    /// The vector graphics format of the web.
    Svg,
}

impl<R> From<R> for ImageFormat
where
    R: Into<RasterFormat>,
{
    fn from(format: R) -> Self {
        Self::Raster(format.into())
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
        Self::Vector(v) => v.into_value(),
    },
    v: RasterFormat => Self::Raster(v),
    v: VectorFormat => Self::Vector(v),
}

/// The image scaling algorithm a viewer should use.
#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash, Cast)]
pub enum ImageScaling {
    /// Scale with a smoothing algorithm such as bilinear interpolation.
    Smooth,
    /// Scale with nearest neighbor or a similar algorithm to preserve the
    /// pixelated look of the image.
    Pixelated,
}
