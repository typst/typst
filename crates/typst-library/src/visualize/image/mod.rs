//! Image handling.

mod pixmap;
mod raster;
mod svg;

pub use self::raster::{RasterFormat, RasterImage};
pub use self::svg::SvgImage;

use std::fmt::{self, Debug, Formatter};
use std::hash::Hash;
use std::sync::Arc;

use comemo::Tracked;
use ecow::EcoString;
use pixmap::{Pixmap, PixmapFormat, PixmapSource};
use typst_syntax::{Span, Spanned};
use typst_utils::LazyHash;

use crate::diag::{bail, At, SourceResult, StrResult};
use crate::engine::Engine;
use crate::foundations::{
    cast, elem, func, scope, AutoValue, Bytes, Cast, Content, Dict, NativeElement,
    Packed, Show, Smart, StyleChain, Value,
};
use crate::layout::{BlockElem, Length, Rel, Sizing};
use crate::loading::Readable;
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
    /// Path to an image file.
    ///
    /// For more details, see the [Paths section]($syntax/#paths).
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

    /// The data required to decode the image.
    #[internal]
    #[required]
    #[parse(data.into())]
    pub source: ImageSource,

    /// The image's format. Detected automatically by default.
    ///
    /// Supported formats are PNG, JPEG, GIF, and SVG. Using a PDF as an image
    /// is [not currently supported](https://github.com/typst/typst/issues/145).
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

    /// Whether text in SVG images should be converted into curves before
    /// embedding. This will result in the text becoming unselectable in the
    /// output.
    #[default(false)]
    pub flatten_text: bool,

    /// A hint to the viewer how it should scale the image.
    ///
    /// **Note:** This option may be ignored and results look different
    /// depending on the format and viewer.
    pub scaling: ImageScaling,
}

#[scope]
#[allow(clippy::too_many_arguments)]
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
        source: ImageSource,
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
        /// Whether text in SVG images should be converted into paths.
        #[named]
        flatten_text: Option<bool>,
        /// How the image should be scaled by the viewer.
        #[named]
        scaling: Option<ImageScaling>,
    ) -> StrResult<Content> {
        let mut elem = ImageElem::new(EcoString::new(), source);
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
    scaling: ImageScaling,
}

impl Image {
    /// When scaling an image to it's natural size, we default to this DPI
    /// if the image doesn't contain DPI metadata.
    pub const DEFAULT_DPI: f64 = 72.0;

    /// Should always be the same as the default DPI used by usvg.
    pub const USVG_DEFAULT_DPI: f64 = 96.0;

    /// Create an image from a source and a format.
    #[comemo::memoize]
    #[typst_macros::time(name = "load image")]
    pub fn new(
        source: ImageSource,
        format: ImageFormat,
        options: &ImageOptions,
    ) -> StrResult<Image> {
        let kind = match format {
            ImageFormat::Raster(format) => {
                let ImageSource::Readable(readable) = source else {
                    bail!("expected readable source for the given format (str or bytes)");
                };
                ImageKind::Raster(RasterImage::new(readable.into(), format)?)
            }
            ImageFormat::Vector(VectorFormat::Svg) => {
                let ImageSource::Readable(readable) = source else {
                    bail!("expected readable source for the given format (str or bytes)");
                };
                ImageKind::Svg(SvgImage::new(readable.into(), options)?)
            }
            ImageFormat::Pixmap(format) => {
                let ImageSource::Pixmap(source) = source else {
                    bail!("source must be a pixmap");
                };
                ImageKind::Pixmap(Pixmap::new(source, format)?)
            }
        };

        Ok(Self(Arc::new(LazyHash::new(Repr {
            kind,
            alt: options.alt.clone(),
            scaling: options.scaling,
        }))))
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
    pub fn scaling(&self) -> ImageScaling {
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

/// Information required to decode an image.
#[derive(Debug, Clone, PartialEq, Hash)]
pub enum ImageSource {
    Readable(Readable),
    Pixmap(Arc<PixmapSource>),
}

impl From<Bytes> for ImageSource {
    fn from(bytes: Bytes) -> Self {
        ImageSource::Readable(Readable::Bytes(bytes))
    }
}

cast! {
    ImageSource,
    data: Readable => ImageSource::Readable(data),
    mut dict: Dict => {
        let source = ImageSource::Pixmap(Arc::new(PixmapSource {
            data: dict.take("data")?.cast()?,
            pixel_width: dict.take("pixel-width")?.cast()?,
            pixel_height: dict.take("pixel-height")?.cast()?,
            icc_profile: dict.take("icc-profile").ok().map(|value| value.cast()).transpose()?,
        }));
        dict.finish(&["data", "pixel-width", "pixel-height", "icc-profile"])?;
        source
    },
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
#[derive(Default, Debug, Copy, Clone, Eq, PartialEq, Hash)]
pub enum ImageScaling {
    /// Use the default scaling algorithm.
    #[default]
    Auto,
    /// Scale photos with a smoothing algorithm such as bilinear interpolation.
    Smooth,
    /// Scale with nearest neighbor or similar to preserve the pixelated look
    /// of the image.
    Pixelated,
}

cast! {
    ImageScaling,
    self => match self {
        ImageScaling::Auto => Value::Auto,
        ImageScaling::Pixelated => "pixelated".into_value(),
        ImageScaling::Smooth => "smooth".into_value(),
    },
    _: AutoValue => ImageScaling::Auto,
    "pixelated" => ImageScaling::Pixelated,
    "smooth" => ImageScaling::Smooth,
}

/// A kind of image.
#[derive(Hash)]
pub enum ImageKind {
    /// A raster image.
    Raster(RasterImage),
    /// An SVG image.
    Svg(SvgImage),
    /// An image constructed from a pixmap.
    Pixmap(Pixmap),
}

pub struct ImageOptions<'a> {
    pub alt: Option<EcoString>,
    pub scaling: ImageScaling,
    pub world: Option<Tracked<'a, dyn World + 'a>>,
    pub families: &'a [&'a str],
    pub flatten_text: bool,
}

impl Default for ImageOptions<'_> {
    fn default() -> Self {
        ImageOptions {
            alt: None,
            scaling: ImageScaling::Auto,
            world: None,
            families: &[],
            flatten_text: false,
        }
    }
}

impl Hash for ImageOptions<'_> {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.alt.hash(state);
        self.scaling.hash(state);
        self.families.hash(state);
        self.flatten_text.hash(state);
    }
}
