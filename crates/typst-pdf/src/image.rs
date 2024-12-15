use image::{DynamicImage, GenericImageView, Rgba};
use krilla::image::{BitsPerComponent, CustomImage, ImageColorspace};
use std::hash::{Hash, Hasher};
use std::sync::{Arc, OnceLock};
use typst_library::visualize::{RasterFormat, RasterImage};

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
            self.raster.icc()
        } else {
            // In all other cases, the dynamic will be converted into RGB8 or LUMA8, so the ICC
            // profile may become invalid, and thus we don't include it.
            None
        }
    }

    fn color_space(&self) -> ImageColorspace {
        if self.raster.dynamic().color().has_color() {
            ImageColorspace::Rgb
        } else {
            ImageColorspace::Luma
        }
    }
}

#[comemo::memoize]
pub(crate) fn raster(raster: RasterImage) -> Option<krilla::image::Image> {
    match raster.format() {
        RasterFormat::Jpg => {
            if !raster.is_rotated() {
                krilla::image::Image::from_jpeg(Arc::new(raster.data().clone()))
            }   else {
                // Can't embed original JPEG data if it needed to be rotated.
                krilla::image::Image::from_custom(PdfImage::new(raster))
            }
        }
        _ => krilla::image::Image::from_custom(PdfImage::new(raster)),
    }
}
