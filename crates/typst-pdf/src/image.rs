use std::hash::{Hash, Hasher};
use std::sync::{Arc, OnceLock};

use ecow::eco_format;
use image::{DynamicImage, EncodableLayout, GenericImageView, Rgba};
use krilla::image::{BitsPerComponent, CustomImage, ImageColorspace};
use krilla::pdf::PdfDocument;
use krilla::surface::Surface;
use krilla_svg::{SurfaceExt, SvgSettings};
use typst_library::diag::{At, SourceResult};
use typst_library::foundations::Smart;
use typst_library::layout::{Abs, Angle, Ratio, Size, Transform};
use typst_library::visualize::{
    ExchangeFormat, Image, ImageKind, ImageScaling, PdfImage, RasterFormat, RasterImage,
};
use typst_syntax::Span;
use typst_utils::defer;

use crate::convert::{FrameContext, GlobalContext};
use crate::tags;
use crate::util::{SizeExt, TransformExt};

#[typst_macros::time(name = "handle image")]
pub(crate) fn handle_image(
    gc: &mut GlobalContext,
    fc: &mut FrameContext,
    image: &Image,
    size: Size,
    surface: &mut Surface,
    span: Span,
) -> SourceResult<()> {
    surface.push_transform(&fc.state().transform().to_krilla());
    surface.set_location(span.into_raw());
    let mut surface = defer(surface, |s| {
        s.pop();
        s.reset_location();
    });

    let interpolate = image.scaling() == Smart::Custom(ImageScaling::Smooth);

    gc.image_spans.insert(span);

    let mut handle = tags::image(gc, fc, &mut surface, image, size);
    let surface = handle.surface();

    match image.kind() {
        ImageKind::Raster(raster) => {
            let (exif_transform, new_size) = exif_transform(raster, size);
            surface.push_transform(&exif_transform.to_krilla());
            let mut surface = defer(surface, |s| s.pop());

            let image = convert_raster(raster.clone(), interpolate)
                .map_err(|e| eco_format!("failed to process image: {e}"))
                .at(span)?;

            if !gc.image_to_spans.contains_key(&image) {
                gc.image_to_spans.insert(image.clone(), span);
            }

            if let Some(size) = new_size.to_krilla() {
                surface.draw_image(image, size);
            }
        }
        ImageKind::Svg(svg) => {
            if let Some(size) = size.to_krilla() {
                surface.draw_svg(
                    svg.tree(),
                    size,
                    SvgSettings { embed_text: true, ..Default::default() },
                );
            }
        }
        ImageKind::Pdf(pdf) => {
            if let Some(size) = size.to_krilla() {
                surface.draw_pdf_page(&convert_pdf(pdf), size, pdf.page_index());
            }
        }
    }

    Ok(())
}

struct Repr {
    /// The original, underlying raster image.
    raster: RasterImage,
    /// The alpha channel of the raster image, if existing.
    alpha_channel: OnceLock<Option<Vec<u8>>>,
    /// A (potentially) converted version of the dynamic image stored `raster` that is
    /// guaranteed to either be in luma8 or rgb8, and thus can be used for the
    /// `color_channel` method of `CustomImage`.
    actual_dynamic: OnceLock<Arc<DynamicImage>>,
}

/// A wrapper around `RasterImage` so that we can implement `CustomImage`.
#[derive(Clone)]
struct PdfRasterImage(Arc<Repr>);

impl PdfRasterImage {
    pub fn new(raster: RasterImage) -> Self {
        Self(Arc::new(Repr {
            raster,
            alpha_channel: OnceLock::new(),
            actual_dynamic: OnceLock::new(),
        }))
    }
}

impl Hash for PdfRasterImage {
    fn hash<H: Hasher>(&self, state: &mut H) {
        // `alpha_channel` and `actual_dynamic` are generated from the underlying `RasterImage`,
        // so this is enough. Since `raster` is prehashed, this is also very cheap.
        self.0.raster.hash(state);
    }
}

impl CustomImage for PdfRasterImage {
    fn color_channel(&self) -> &[u8] {
        self.0
            .actual_dynamic
            .get_or_init(|| {
                let dynamic = self.0.raster.dynamic();
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
        self.0
            .alpha_channel
            .get_or_init(|| {
                self.0.raster.dynamic().color().has_alpha().then(|| {
                    self.0
                        .raster
                        .dynamic()
                        .pixels()
                        .map(|(_, _, Rgba([_, _, _, a]))| a)
                        .collect()
                })
            })
            .as_ref()
            .map(|v| &**v)
    }

    fn bits_per_component(&self) -> BitsPerComponent {
        BitsPerComponent::Eight
    }

    fn size(&self) -> (u32, u32) {
        (self.0.raster.width(), self.0.raster.height())
    }

    fn icc_profile(&self) -> Option<&[u8]> {
        if matches!(
            self.0.raster.dynamic().as_ref(),
            DynamicImage::ImageLuma8(_)
                | DynamicImage::ImageLumaA8(_)
                | DynamicImage::ImageRgb8(_)
                | DynamicImage::ImageRgba8(_)
        ) {
            self.0.raster.icc().map(|b| b.as_bytes())
        } else {
            // In all other cases, the dynamic will be converted into RGB8 or LUMA8, so the ICC
            // profile may become invalid, and thus we don't include it.
            None
        }
    }

    fn color_space(&self) -> ImageColorspace {
        // Remember that we convert all images to either RGB or luma.
        if self.0.raster.dynamic().color().has_color() {
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
) -> Result<krilla::image::Image, String> {
    if let RasterFormat::Exchange(ExchangeFormat::Jpg) = raster.format() {
        let image_data: Arc<dyn AsRef<[u8]> + Send + Sync> =
            Arc::new(raster.data().clone());
        let icc_profile = raster.icc().map(|i| {
            let i: Arc<dyn AsRef<[u8]> + Send + Sync> = Arc::new(i.clone());
            i
        });

        krilla::image::Image::from_jpeg_with_icc(
            image_data.into(),
            icc_profile.map(|i| i.into()),
            interpolate,
        )
    } else {
        krilla::image::Image::from_custom(PdfRasterImage::new(raster), interpolate)
    }
}

#[comemo::memoize]
fn convert_pdf(pdf: &PdfImage) -> PdfDocument {
    PdfDocument::new(pdf.document().pdf().clone())
}

fn exif_transform(image: &RasterImage, size: Size) -> (Transform, Size) {
    // For JPEGs, we want to apply the EXIF orientation as a transformation
    // because we don't recode them. For other formats, the transform is already
    // baked into the dynamic image data.
    if image.format() != RasterFormat::Exchange(ExchangeFormat::Jpg) {
        return (Transform::identity(), size);
    }

    let base = |hp: bool, vp: bool, mut base_ts: Transform, size: Size| {
        if hp {
            // Flip horizontally in-place.
            base_ts = base_ts.pre_concat(
                Transform::scale(-Ratio::one(), Ratio::one())
                    .pre_concat(Transform::translate(-size.x, Abs::zero())),
            )
        }

        if vp {
            // Flip vertically in-place.
            base_ts = base_ts.pre_concat(
                Transform::scale(Ratio::one(), -Ratio::one())
                    .pre_concat(Transform::translate(Abs::zero(), -size.y)),
            )
        }

        base_ts
    };

    let no_flipping =
        |hp: bool, vp: bool| (base(hp, vp, Transform::identity(), size), size);

    let with_flipping = |hp: bool, vp: bool| {
        let base_ts = Transform::rotate_at(Angle::deg(90.0), Abs::zero(), Abs::zero())
            .pre_concat(Transform::scale(Ratio::one(), -Ratio::one()));
        let inv_size = Size::new(size.y, size.x);
        (base(hp, vp, base_ts, inv_size), inv_size)
    };

    match image.exif_rotation() {
        Some(2) => no_flipping(true, false),
        Some(3) => no_flipping(true, true),
        Some(4) => no_flipping(false, true),
        Some(5) => with_flipping(false, false),
        Some(6) => with_flipping(false, true),
        Some(7) => with_flipping(true, true),
        Some(8) => with_flipping(true, false),
        _ => no_flipping(false, false),
    }
}
