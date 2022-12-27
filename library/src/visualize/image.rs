use std::ffi::OsStr;

use typst::image::{Image, ImageFormat, RasterFormat, VectorFormat};

use crate::prelude::*;

/// # Image
/// A raster or vector graphic.
///
/// Supported formats are PNG, JPEG, GIF and SVG.
///
/// ## Example
/// ```
/// #align(center)[
///   #image("molecular.jpg", width: 80%)
///
///   *A step in the molecular testing
///   pipeline of our lab*
/// ]
/// ```
///
/// ## Parameters
/// - path: EcoString (positional, required)
///   Path to an image file.
///
/// - width: Rel<Length> (named)
///   The width of the image.
///
/// - height: Rel<Length> (named)
///   The height of the image.
///
/// ## Category
/// visualize
#[func]
#[capable(Layout, Inline)]
#[derive(Debug, Hash)]
pub struct ImageNode(pub Image);

#[node]
impl ImageNode {
    /// How the image should adjust itself to a given area.
    pub const FIT: ImageFit = ImageFit::Cover;

    fn construct(vm: &Vm, args: &mut Args) -> SourceResult<Content> {
        let Spanned { v: path, span } =
            args.expect::<Spanned<EcoString>>("path to image file")?;

        let full = vm.locate(&path).at(span)?;
        let buffer = vm.world().file(&full).at(span)?;
        let ext = full.extension().and_then(OsStr::to_str).unwrap_or_default();
        let format = match ext.to_lowercase().as_str() {
            "png" => ImageFormat::Raster(RasterFormat::Png),
            "jpg" | "jpeg" => ImageFormat::Raster(RasterFormat::Jpg),
            "gif" => ImageFormat::Raster(RasterFormat::Gif),
            "svg" | "svgz" => ImageFormat::Vector(VectorFormat::Svg),
            _ => bail!(span, "unknown image format"),
        };

        let image = Image::new(buffer, format).at(span)?;
        let width = args.named("width")?;
        let height = args.named("height")?;

        Ok(ImageNode(image).pack().boxed(Axes::new(width, height)))
    }
}

impl Layout for ImageNode {
    fn layout(
        &self,
        _: &mut Vt,
        styles: StyleChain,
        regions: Regions,
    ) -> SourceResult<Fragment> {
        let pxw = self.0.width() as f64;
        let pxh = self.0.height() as f64;
        let px_ratio = pxw / pxh;

        // Find out whether the image is wider or taller than the target size.
        let Regions { first, expand, .. } = regions;
        let region_ratio = first.x / first.y;
        let wide = px_ratio > region_ratio;

        // The space into which the image will be placed according to its fit.
        let target = if expand.x && expand.y {
            first
        } else if expand.x || (!expand.y && wide && first.x.is_finite()) {
            Size::new(first.x, first.y.min(first.x.safe_div(px_ratio)))
        } else if first.y.is_finite() {
            Size::new(first.x.min(first.y * px_ratio), first.y)
        } else {
            Size::new(Abs::pt(pxw), Abs::pt(pxh))
        };

        // Compute the actual size of the fitted image.
        let fit = styles.get(Self::FIT);
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
        frame.push(Point::zero(), Element::Image(self.0.clone(), fitted));
        frame.resize(target, Align::CENTER_HORIZON);

        // Create a clipping group if only part of the image should be visible.
        if fit == ImageFit::Cover && !target.fits(fitted) {
            frame.clip();
        }

        // Apply metadata.
        frame.meta(styles);

        Ok(Fragment::frame(frame))
    }
}

impl Inline for ImageNode {}

/// How an image should adjust itself to a given area.
#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
pub enum ImageFit {
    /// The image should completely cover the area.
    Cover,
    /// The image should be fully contained in the area.
    Contain,
    /// The image should be stretched so that it exactly fills the area.
    Stretch,
}

castable! {
    ImageFit,
    /// The image should completely cover the area. This is the default.
    "cover" => Self::Cover,
    /// The image should be fully contained in the area.
    "contain" => Self::Contain,
    /// The image should be stretched so that it exactly fills the area, even if
    /// this means that the image will be distorted.
    "stretch" => Self::Stretch,
}
