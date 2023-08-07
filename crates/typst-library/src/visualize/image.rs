use std::ffi::OsStr;
use std::path::Path;

use typst::geom::Smart;
use typst::image::{Image, ImageFormat, RasterFormat, VectorFormat};

use crate::compute::Readable;
use crate::meta::{Figurable, LocalName};
use crate::prelude::*;
use crate::text::families;

/// A raster or vector graphic.
///
/// Supported formats are PNG, JPEG, GIF and SVG.
///
/// _Note:_ Work on SVG export is ongoing and there might be visual inaccuracies
/// in the resulting PDF. Make sure to double-check embedded SVG images. If you
/// have an issue, also feel free to report it on [GitHub][gh-svg].
///
/// ## Example { #example }
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
///
/// Display: Image
/// Category: visualize
#[element(Layout, LocalName, Figurable)]
#[scope(
    scope.define("decode", image_decode_func());
    scope
)]
pub struct ImageElem {
    /// Path to an image file.
    #[required]
    #[parse(
        let Spanned { v: path, span } =
            args.expect::<Spanned<EcoString>>("path to image file")?;
        let id = vm.location().join(&path).at(span)?;
        let data = vm.world().file(id).at(span)?;
        path
    )]
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

    /// How the image should adjust itself to a given area.
    #[default(ImageFit::Cover)]
    pub fit: ImageFit,
}

/// Decode a raster of vector graphic from bytes or a string.
///
/// ## Example { #example }
/// ```example
/// #let original = read("diagram.svg")
/// #let changed = original.replace(
///   "#2B80FF", // blue
///   green.hex(),
/// )
///
/// #image.decode(original)
/// #image.decode(changed)
/// ```
///
/// Display: Decode Image
/// Category: visualize
#[func]
pub fn image_decode(
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
    Ok(elem.pack())
}

impl Layout for ImageElem {
    #[tracing::instrument(name = "ImageElem::layout", skip_all)]
    fn layout(
        &self,
        vt: &mut Vt,
        styles: StyleChain,
        regions: Regions,
    ) -> SourceResult<Fragment> {
        // Take the format that was explicitly defined, or parse the extention,
        // or try to detect the format.
        let data = self.data();
        let format = match self.format(styles) {
            Smart::Custom(v) => v,
            Smart::Auto => {
                let ext = Path::new(self.path().as_str())
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
            data.into(),
            format,
            vt.world,
            families(styles).next().as_ref().map(|f| f.as_str()),
            self.alt(styles),
        )
        .at(self.span())?;

        let sizing = Axes::new(self.width(styles), self.height(styles));
        let region = sizing
            .zip(regions.base())
            .map(|(s, r)| s.map(|v| v.resolve(styles).relative_to(r)))
            .unwrap_or(regions.base());

        let expand = sizing.as_ref().map(Smart::is_custom) | regions.expand;
        let region_ratio = region.x / region.y;

        // Find out whether the image is wider or taller than the target size.
        let pxw = image.width() as f64;
        let pxh = image.height() as f64;
        let px_ratio = pxw / pxh;
        let wide = px_ratio > region_ratio;

        // The space into which the image will be placed according to its fit.
        let target = if expand.x && expand.y {
            region
        } else if expand.x || (!expand.y && wide && region.x.is_finite()) {
            Size::new(region.x, region.y.min(region.x.safe_div(px_ratio)))
        } else if region.y.is_finite() {
            Size::new(region.x.min(region.y * px_ratio), region.y)
        } else {
            Size::new(Abs::pt(pxw), Abs::pt(pxh))
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
        let mut frame = Frame::new(fitted);
        frame.push(Point::zero(), FrameItem::Image(image, fitted, self.span()));
        frame.resize(target, Align::CENTER_HORIZON);

        // Create a clipping group if only part of the image should be visible.
        if fit == ImageFit::Cover && !target.fits(fitted) {
            frame.clip();
        }

        // Apply metadata.
        frame.meta(styles, false);

        Ok(Fragment::frame(frame))
    }
}

impl LocalName for ImageElem {
    fn local_name(&self, lang: Lang, _: Option<Region>) -> &'static str {
        match lang {
            Lang::ALBANIAN => "Figurë",
            Lang::ARABIC => "شكل",
            Lang::BOKMÅL => "Figur",
            Lang::CHINESE => "图",
            Lang::CZECH => "Obrázek",
            Lang::DANISH => "Figur",
            Lang::DUTCH => "Figuur",
            Lang::FILIPINO => "Pigura",
            Lang::FRENCH => "Figure",
            Lang::GERMAN => "Abbildung",
            Lang::ITALIAN => "Figura",
            Lang::NYNORSK => "Figur",
            Lang::POLISH => "Rysunek",
            Lang::PORTUGUESE => "Figura",
            Lang::RUSSIAN => "Рисунок",
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

impl Figurable for ImageElem {}

/// How an image should adjust itself to a given area.
#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash, Cast)]
pub enum ImageFit {
    /// The image should completely cover the area. This is the default.
    Cover,
    /// The image should be fully contained in the area.
    Contain,
    /// The image should be stretched so that it exactly fills the area, even if
    /// this means that the image will be distorted.
    Stretch,
}
