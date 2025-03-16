use std::hash::{Hash, Hasher};
use std::sync::{Arc, OnceLock};

use image::{DynamicImage, EncodableLayout, GenericImageView, Rgba};
use krilla::graphics::image::{BitsPerComponent, CustomImage, ImageColorspace};
use krilla::surface::Surface;
use krilla_svg::{SurfaceExt, SvgSettings};
use typst_library::diag::{bail, SourceResult};
use typst_library::foundations::Smart;
use typst_library::layout::Size;
use typst_library::visualize::{
    ExchangeFormat, Image, ImageKind, ImageScaling, RasterFormat, RasterImage,
};
use typst_syntax::Span;

use crate::convert::{FrameContext, GlobalContext};
use crate::util::{SizeExt, TransformExt};

pub(crate) fn handle_image(
    gc: &mut GlobalContext,
    fc: &mut FrameContext,
    image: &Image,
    size: Size,
    surface: &mut Surface,
    span: Span,
) -> SourceResult<()> {
    surface.push_transform(&fc.state().transform().to_krilla());
    surface.set_location(span.into_raw().get());

    let interpolate = image.scaling() == Smart::Custom(ImageScaling::Smooth);

    match image.kind() {
        ImageKind::Raster(raster) => {
            let image = match convert_raster(raster.clone(), interpolate) {
                None => bail!(span, "failed to process image"),
                Some(i) => i,
            };

            if !gc.image_to_spans.contains_key(&image) {
                gc.image_to_spans.insert(image.clone(), span);
                gc.image_spans.insert(span);
            }

            surface.draw_image(image, size.to_krilla());
        }
        ImageKind::Svg(svg) => {
            surface.draw_svg(
                svg.tree(),
                size.to_krilla(),
                SvgSettings { embed_text: true, ..Default::default() },
            );
        }
    }

    surface.pop();
    surface.reset_location();

    Ok(())
}

/// A wrapper around RasterImage so that we can implement `CustomImage`.
#[derive(Clone)]
struct PdfImage {
    /// The original, underlying raster image.
    raster: RasterImage,
    /// The alpha channel of the raster image, if existing.
    alpha_channel: OnceLock<Option<Arc<Vec<u8>>>>,
    /// A (potentially) converted version of the dynamic image stored `raster` that is
    /// guaranteed to either be in luma8 or rgb8, and thus can be used for the
    /// `color_channel` method of `CustomImage`.
    actual_dynamic: OnceLock<Arc<DynamicImage>>,
}

impl PdfImage {
    pub fn new(raster: RasterImage) -> Self {
        Self {
            raster,
            alpha_channel: OnceLock::new(),
            actual_dynamic: OnceLock::new(),
        }
    }
}

impl Hash for PdfImage {
    fn hash<H: Hasher>(&self, state: &mut H) {
        // `alpha_channel` and `actual_dynamic` are generated from the underlying `RasterImage`,
        // so this is enough. Since `raster` is prehashed, this is also very cheap.
        self.raster.hash(state);
    }
}

impl CustomImage for PdfImage {
    fn color_channel(&self) -> &[u8] {
        self.actual_dynamic
            .get_or_init(|| {
                let dynamic = self.raster.dynamic();
                let channel_count = dynamic.color().channel_count();

                match (dynamic.as_ref(), channel_count) {
                    // Pure luma8 or rgb8 image, can use it directly.
                    (DynamicImage::ImageLuma8(_), _) => dynamic.clone(),
                    (DynamicImage::ImageRgb8(_), _) => dynamic.clone(),
                    // Grey-scale image, convert to luma8.
                    (_, 1 | 2) => Arc::new(DynamicImage::ImageLuma8(dynamic.to_luma8())),
                    // Anything else, convert to rgb8.
                    _ => Arc::new(DynamicImage::ImageRgb8(dynamic.to_rgb8())),
                }
            })
            .as_bytes()
    }

    fn alpha_channel(&self) -> Option<&[u8]> {
        self.alpha_channel
            .get_or_init(|| {
                self.raster.dynamic().color().has_alpha().then(|| {
                    Arc::new(
                        self.raster
                            .dynamic()
                            .pixels()
                            .map(|(_, _, Rgba([_, _, _, a]))| a)
                            .collect(),
                    )
                })
            })
            .as_ref()
            .map(|v| &***v)
    }

    fn bits_per_component(&self) -> BitsPerComponent {
        BitsPerComponent::Eight
    }

    fn size(&self) -> (u32, u32) {
        (self.raster.width(), self.raster.height())
    }

    fn icc_profile(&self) -> Option<&[u8]> {
        if matches!(
            self.raster.dynamic().as_ref(),
            DynamicImage::ImageLuma8(_)
                | DynamicImage::ImageLumaA8(_)
                | DynamicImage::ImageRgb8(_)
                | DynamicImage::ImageRgba8(_)
        ) {
            self.raster.icc().map(|b| b.as_bytes())
        } else {
            // In all other cases, the dynamic will be converted into RGB8 or LUMA8, so the ICC
            // profile may become invalid, and thus we don't include it.
            None
        }
    }

    fn color_space(&self) -> ImageColorspace {
        // Remember that we convert all images to either RGB or luma.
        if self.raster.dynamic().color().has_color() {
            ImageColorspace::Rgb
        } else {
            ImageColorspace::Luma
        }
    }
}

#[comemo::memoize]
fn convert_raster(
    raster: RasterImage,
    interpolate: bool,
) -> Option<krilla::graphics::image::Image> {
    match raster.format() {
        RasterFormat::Exchange(e) => match e {
            ExchangeFormat::Jpg => {
                if !raster.is_rotated() {
                    let image_data: Arc<dyn AsRef<[u8]> + Send + Sync> =
                        Arc::new(raster.data().clone());
                    krilla::graphics::image::Image::from_jpeg(image_data.into(), interpolate)
                } else {
                    // Can't embed original JPEG data if it had to be rotated.
                    krilla::graphics::image::Image::from_custom(PdfImage::new(raster), interpolate)
                }
            }
            _ => krilla::graphics::image::Image::from_custom(PdfImage::new(raster), interpolate),
        },
        RasterFormat::Pixel(_) => {
            krilla::graphics::image::Image::from_custom(PdfImage::new(raster), interpolate)
        }
    }
}
