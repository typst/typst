use std::io;

use super::prelude::*;
use crate::diag::Error;
use crate::image::ImageId;

/// `image`: An image.
pub fn image(ctx: &mut EvalContext, args: &mut Args) -> TypResult<Value> {
    let path = args.expect::<Spanned<EcoString>>("path to image file")?;
    let width = args.named("width")?;
    let height = args.named("height")?;
    let fit = args.named("fit")?.unwrap_or_default();

    // Load the image.
    let full = ctx.make_path(&path.v);
    let id = ctx.images.load(&full).map_err(|err| {
        Error::boxed(path.span, match err.kind() {
            io::ErrorKind::NotFound => "file not found".into(),
            _ => format!("failed to load image ({})", err),
        })
    })?;

    Ok(Value::Template(Template::from_inline(move |_| {
        ImageNode { id, fit }.pack().sized(Spec::new(width, height))
    })))
}

/// An image node.
#[derive(Debug, Hash)]
pub struct ImageNode {
    /// The id of the image file.
    pub id: ImageId,
    /// How the image should adjust itself to a given area.
    pub fit: ImageFit,
}

impl Layout for ImageNode {
    fn layout(
        &self,
        ctx: &mut LayoutContext,
        regions: &Regions,
    ) -> Vec<Constrained<Rc<Frame>>> {
        let img = ctx.images.get(self.id);
        let pxw = img.width() as f64;
        let pxh = img.height() as f64;
        let px_ratio = pxw / pxh;

        // Find out whether the image is wider or taller than the target size.
        let current = regions.current;
        let current_ratio = current.x / current.y;
        let wide = px_ratio > current_ratio;

        // The space into which the image will be placed according to its fit.
        let target = if regions.expand.x && regions.expand.y {
            current
        } else if regions.expand.x || (wide && current.x.is_finite()) {
            Size::new(current.x, current.y.min(current.x.safe_div(px_ratio)))
        } else if current.y.is_finite() {
            Size::new(current.x.min(current.y * px_ratio), current.y)
        } else {
            Size::new(Length::pt(pxw), Length::pt(pxh))
        };

        // The actual size of the fitted image.
        let fitted = match self.fit {
            ImageFit::Contain | ImageFit::Cover => {
                if wide == (self.fit == ImageFit::Contain) {
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
        frame.push(Point::zero(), Element::Image(self.id, fitted));
        frame.resize(target, Align::CENTER_HORIZON);

        // Create a clipping group if the fit mode is "cover".
        if self.fit == ImageFit::Cover {
            frame.clip();
        }

        vec![frame.constrain(Constraints::tight(regions))]
    }
}

/// How an image should adjust itself to a given area.
#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
pub enum ImageFit {
    /// The image should be fully contained in the area.
    Contain,
    /// The image should completely cover the area.
    Cover,
    /// The image should be stretched so that it exactly fills the area.
    Stretch,
}

castable! {
    ImageFit,
    Expected: "string",
    Value::Str(string) => match string.as_str() {
        "contain" => Self::Contain,
        "cover" => Self::Cover,
        "stretch" => Self::Stretch,
        _ => Err(r#"expected "contain", "cover" or "stretch""#)?,
    },
}

impl Default for ImageFit {
    fn default() -> Self {
        Self::Contain
    }
}
