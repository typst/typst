//! Image handling.

mod raster;
mod svg;

pub use self::raster::{RasterFormat, RasterImage};
pub use self::svg::SvgImage;

use std::fmt::{self, Debug, Formatter};
use std::sync::Arc;

use comemo::Tracked;
use ecow::EcoString;
use typst_syntax::{Span, Spanned};
use typst_utils::LazyHash;

use crate::diag::{At, SourceResult, StrResult};
use crate::engine::Engine;
use crate::foundations::{
    cast, elem, func, scope, Bytes, Cast, Content, NativeElement, Packed, Show, Smart,
    StyleChain,
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
    /// Path to an image file
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

    /// The raw file data.
    #[internal]
    #[required]
    #[parse(Readable::Bytes(data))]
    pub data: Readable,

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

    /// Whether text in SVG images should be converted into paths before
    /// embedding. This will result in the text becoming unselectable in
    /// the output.
    #[default(false)]
    pub flatten_text: bool,
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
        height: Option<Sizing>,
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

    /// Should always be the same as the default DPI used by usvg.
    pub const USVG_DEFAULT_DPI: f64 = 96.0;

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

    /// Create a possibly font-dependent image from a buffer and a format.
    #[comemo::memoize]
    #[typst_macros::time(name = "load image")]
    pub fn with_fonts(
        data: Bytes,
        format: ImageFormat,
        alt: Option<EcoString>,
        world: Tracked<dyn World + '_>,
        families: &[&str],
        outlined: bool,
    ) -> StrResult<Image> {
        let kind = match format {
            ImageFormat::Raster(format) => {
                ImageKind::Raster(RasterImage::new(data, format)?)
            }
            ImageFormat::Vector(VectorFormat::Svg) => {
                ImageKind::Svg(SvgImage::with_fonts(data, world, outlined, families)?)
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
            ImageKind::Svg(_) => Some(Image::USVG_DEFAULT_DPI),
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
