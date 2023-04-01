use std::ffi::OsStr;
use std::path::Path;

use typst::image::{Image, ImageFormat, RasterFormat, VectorFormat};

use crate::prelude::*;

/// A raster or vector graphic.
///
/// Supported formats are PNG, JPEG, GIF and SVG.
///
/// ## Example
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
/// Display: Image
/// Category: visualize
#[element(Layout)]
pub struct ImageElem {
    /// Path to an image file.
    #[required]
    #[parse(
        let Spanned { v: path, span } =
            args.expect::<Spanned<EcoString>>("path to image file")?;
        let path: EcoString = vm.locate(&path).at(span)?.to_string_lossy().into();
        let _ = load(vm.world(), &path).at(span)?;
        path
    )]
    pub path: EcoString,

    /// The width of the image.
    pub width: Smart<Rel<Length>>,

    /// The height of the image.
    pub height: Smart<Rel<Length>>,

    /// How the image should adjust itself to a given area.
    #[default(ImageFit::Cover)]
    pub fit: ImageFit,
}

impl Layout for ImageElem {
    fn layout(
        &self,
        vt: &mut Vt,
        styles: StyleChain,
        regions: Regions,
    ) -> SourceResult<Fragment> {
        let image = load(vt.world, &self.path()).unwrap();
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

/// Load an image from a path.
#[comemo::memoize]
fn load(world: Tracked<dyn World>, full: &str) -> StrResult<Image> {
    let full = Path::new(full);
    let buffer = world.file(full)?;
    let ext = full.extension().and_then(OsStr::to_str).unwrap_or_default();
    let format = match ext.to_lowercase().as_str() {
        "png" => ImageFormat::Raster(RasterFormat::Png),
        "jpg" | "jpeg" => ImageFormat::Raster(RasterFormat::Jpg),
        "gif" => ImageFormat::Raster(RasterFormat::Gif),
        "svg" | "svgz" => ImageFormat::Vector(VectorFormat::Svg),
        _ => return Err("unknown image format".into()),
    };
    Image::new(buffer, format)
}
