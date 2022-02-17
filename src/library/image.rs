//! Raster and vector graphics.

use super::prelude::*;
use super::TextNode;
use crate::diag::Error;
use crate::image::ImageId;

/// Show a raster or vector graphic.
#[derive(Debug, Hash)]
pub struct ImageNode(pub ImageId);

#[class]
impl ImageNode {
    /// How the image should adjust itself to a given area.
    pub const FIT: ImageFit = ImageFit::Cover;

    fn construct(vm: &mut Vm, args: &mut Args) -> TypResult<Template> {
        let path = args.expect::<Spanned<EcoString>>("path to image file")?;
        let full = vm.resolve(&path.v);
        let id = vm.images.load(&full).map_err(|err| {
            Error::boxed(path.span, match err.kind() {
                std::io::ErrorKind::NotFound => "file not found".into(),
                _ => format!("failed to load image ({})", err),
            })
        })?;

        let width = args.named("width")?;
        let height = args.named("height")?;

        Ok(Template::inline(
            ImageNode(id).pack().sized(Spec::new(width, height)),
        ))
    }
}

impl Layout for ImageNode {
    fn layout(
        &self,
        vm: &mut Vm,
        regions: &Regions,
        styles: StyleChain,
    ) -> Vec<Constrained<Arc<Frame>>> {
        let img = vm.images.get(self.0);
        let pxw = img.width() as f64;
        let pxh = img.height() as f64;
        let px_ratio = pxw / pxh;

        // Find out whether the image is wider or taller than the target size.
        let &Regions { current, expand, .. } = regions;
        let current_ratio = current.x / current.y;
        let wide = px_ratio > current_ratio;

        // The space into which the image will be placed according to its fit.
        let target = if expand.x && expand.y {
            current
        } else if expand.x || (!expand.y && wide && current.x.is_finite()) {
            Size::new(current.x, current.y.min(current.x.safe_div(px_ratio)))
        } else if current.y.is_finite() {
            Size::new(current.x.min(current.y * px_ratio), current.y)
        } else {
            Size::new(Length::pt(pxw), Length::pt(pxh))
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
        frame.push(Point::zero(), Element::Image(self.0, fitted));
        frame.resize(target, Align::CENTER_HORIZON);

        // Create a clipping group if only part of the image should be visible.
        if fit == ImageFit::Cover && !target.fits(fitted) {
            frame.clip();
        }

        // Apply link if it exists.
        if let Some(url) = styles.get_ref(TextNode::LINK) {
            frame.link(url);
        }

        vec![frame.constrain(Constraints::tight(regions))]
    }
}

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
    Expected: "string",
    Value::Str(string) => match string.as_str() {
        "cover" => Self::Cover,
        "contain" => Self::Contain,
        "stretch" => Self::Stretch,
        _ => Err(r#"expected "cover", "contain" or "stretch""#)?,
    },
}
