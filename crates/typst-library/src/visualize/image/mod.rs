//! Image handling.

mod pixmap;
mod raster;
mod svg;

pub use self::pixmap::{PixmapFormat, PixmapImage, PixmapSource};
pub use self::raster::{RasterFormat, RasterImage};
pub use self::svg::SvgImage;

use std::fmt::{self, Debug, Formatter};
use std::sync::Arc;

use comemo::Tracked;
use ecow::EcoString;
use typst_syntax::{Span, Spanned};
use typst_utils::LazyHash;

use crate::diag::{SourceResult, StrResult};
use crate::engine::Engine;
use crate::foundations::{
    cast, elem, func, scope, Bytes, Cast, Content, Derived, NativeElement, Packed, Show,
    Smart, StyleChain,
};
use crate::layout::{BlockElem, Length, Rel, Sizing};
use crate::loading::{DataSource, Load, Readable};
use crate::model::Figurable;
use crate::text::LocalName;
use crate::World;

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
#[elem(scope, Show, LocalName, Figurable)]
pub struct ImageElem {
    /// The source to load the image from. Either of:
    ///
    /// - A path to an image file. For more details about paths, see the [Paths
    ///   section]($syntax/#paths).
    /// - Raw bytes making up an encoded image.
    /// - A dictionary with the following keys:
    ///   - `data` ([bytes]): Raw pixel data in the specified [`format`]($image.format).
    ///   - `pixel-width` ([int]): The width in pixels.
    ///   - `pixel-height` ([int]): The height in pixels.
    ///   - `icc-profile` ([bytes], optional): An ICC profile for the image.
    ///
    ///   The width multiplied by the height multiplied by the channel count for
    ///   the specified format must match the data length.
    #[required]
    #[parse(
        let source = args.expect::<Spanned<ImageSource>>("source")?;
        let data = source.load(engine.world)?;
        Derived::new(source.v, data)
    )]
    pub source: Derived<ImageSource, Bytes>,

    /// The image's format. Detected automatically by default.
    ///
    /// Supported image formats are PNG, JPEG, GIF, and SVG. Using a PDF as an image
    /// is [not currently supported](https://github.com/typst/typst/issues/145).
    ///
    /// Aside from these encoded image formats, Typst also lets you provide raw
    /// image data as the source. In this case, providing a format is mandatory.
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

    /// Whether text in SVG images should be converted into curves before
    /// embedding. This will result in the text becoming unselectable in the
    /// output.
    #[default(false)]
    pub flatten_text: bool,
}

#[scope]
#[allow(clippy::too_many_arguments)]
impl ImageElem {
    /// Decode a raster or vector graphic from bytes or a string.
    ///
    /// This function is deprecated. The [`image`] function now accepts bytes
    /// directly.
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
        /// Whether text in SVG images should be converted into curves before
        /// embedding.
        #[named]
        flatten_text: Option<bool>,
    ) -> StrResult<Content> {
        let bytes = data.into_bytes();
        let source =
            Derived::new(ImageSource::Data(DataSource::Bytes(bytes.clone())), bytes);
        let mut elem = ImageElem::new(source);
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
        if let Some(fit) = fit {
            elem.push_fit(fit);
        }
        if let Some(flatten_text) = flatten_text {
            elem.push_flatten_text(flatten_text);
        }
        if let Some(scaling) = scaling {
            elem.push_scaling(scaling);
        }
        Ok(elem.pack().spanned(span))
    }
}

impl Show for Packed<ImageElem> {
    fn show(&self, engine: &mut Engine, styles: StyleChain) -> SourceResult<Content> {
        Ok(BlockElem::single_layouter(self.clone(), engine.routines.layout_image)
            .with_width(self.width(styles))
            .with_height(self.height(styles))
            .pack()
            .spanned(self.span()))
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

    /// Create an image from a kind.
    pub fn new(
        kind: impl Into<ImageKind>,
        alt: Option<EcoString>,
        scaling: Smart<ImageScaling>,
    ) -> Self {
        Self::new_impl(kind.into(), alt, scaling)
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
            ImageKind::Pixmap(pixmap) => pixmap.format().into(),
        }
    }

    /// The width of the image in pixels.
    pub fn width(&self) -> f64 {
        match &self.0.kind {
            ImageKind::Raster(raster) => raster.width() as f64,
            ImageKind::Svg(svg) => svg.width(),
            ImageKind::Pixmap(pixmap) => pixmap.width() as f64,
        }
    }

    /// The height of the image in pixels.
    pub fn height(&self) -> f64 {
        match &self.0.kind {
            ImageKind::Raster(raster) => raster.height() as f64,
            ImageKind::Svg(svg) => svg.height(),
            ImageKind::Pixmap(pixmap) => pixmap.height() as f64,
        }
    }

    /// The image's pixel density in pixels per inch, if known.
    pub fn dpi(&self) -> Option<f64> {
        match &self.0.kind {
            ImageKind::Raster(raster) => raster.dpi(),
            ImageKind::Svg(_) => Some(Image::USVG_DEFAULT_DPI),
            ImageKind::Pixmap(_) => None,
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

/// Information specifying the source of an image's byte data.
#[derive(Debug, Clone, PartialEq, Hash)]
pub enum ImageSource {
    Data(DataSource),
    Pixmap(PixmapSource),
}

impl From<Bytes> for ImageSource {
    fn from(bytes: Bytes) -> Self {
        ImageSource::Data(DataSource::Bytes(bytes))
    }
}

impl Load for Spanned<ImageSource> {
    type Output = Bytes;

    fn load(&self, world: Tracked<dyn World + '_>) -> SourceResult<Self::Output> {
        match &self.v {
            ImageSource::Data(data) => Spanned::new(data, self.span).load(world),
            ImageSource::Pixmap(pixmap) => Ok(pixmap.data.clone()),
        }
    }
}

cast! {
    ImageSource,
    self => match self {
       Self::Data(data) => data.into_value(),
       Self::Pixmap(pixmap) => pixmap.into_value(),
    },
    data: DataSource => Self::Data(data),
    pixmap: PixmapSource => Self::Pixmap(pixmap),
}

/// A kind of image.
#[derive(Clone, Hash)]
pub enum ImageKind {
    /// A raster image.
    Raster(RasterImage),
    /// An SVG image.
    Svg(SvgImage),
    /// An image constructed from a pixmap.
    Pixmap(PixmapImage),
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

impl From<PixmapImage> for ImageKind {
    fn from(image: PixmapImage) -> Self {
        Self::Pixmap(image)
    }
}

/// A raster or vector image format.
#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
pub enum ImageFormat {
    /// A raster graphics format.
    Raster(RasterFormat),
    /// A vector graphics format.
    Vector(VectorFormat),
    /// A format made up of flat pixels without metadata or compression.
    Pixmap(PixmapFormat),
}

impl ImageFormat {
    /// Try to detect the format of an image from data.
    pub fn detect(data: &[u8]) -> Option<Self> {
        if let Some(format) = RasterFormat::detect(data) {
            return Some(Self::Raster(format));
        }

        // SVG or compressed SVG.
        if data.starts_with(b"<svg") || data.starts_with(&[0x1f, 0x8b]) {
            return Some(Self::Vector(VectorFormat::Svg));
        }

        None
    }
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

impl From<PixmapFormat> for ImageFormat {
    fn from(format: PixmapFormat) -> Self {
        Self::Pixmap(format)
    }
}

cast! {
    ImageFormat,
    self => match self {
        Self::Raster(v) => v.into_value(),
        Self::Vector(v) => v.into_value(),
        Self::Pixmap(v) => v.into_value(),
    },
    v: RasterFormat => Self::Raster(v),
    v: VectorFormat => Self::Vector(v),
    v: PixmapFormat => Self::Pixmap(v),
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
