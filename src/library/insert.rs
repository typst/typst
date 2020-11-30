use image::GenericImageView;

use crate::env::{ImageResource, ResourceId};
use crate::layout::*;
use crate::prelude::*;

/// `image`: Insert an image.
///
/// # Positional arguments
/// - The path to the image (string)
///
/// Supports PNG and JPEG files.
pub fn image(mut args: Args, ctx: &mut EvalContext) -> Value {
    let path = args.need::<_, Spanned<String>>(ctx, 0, "path");
    let width = args.get::<_, Linear>(ctx, "width");
    let height = args.get::<_, Linear>(ctx, "height");

    if let Some(path) = path {
        let mut env = ctx.env.borrow_mut();
        let loaded = env.resources.load(path.v, ImageResource::parse);

        if let Some((res, img)) = loaded {
            let dimensions = img.buf.dimensions();
            drop(env);
            ctx.push(Image {
                res,
                dimensions,
                width,
                height,
                align: ctx.state.align,
            });
        } else {
            drop(env);
            ctx.diag(error!(path.span, "failed to load image"));
        }
    }

    Value::None
}

/// An image node.
#[derive(Debug, Clone, PartialEq)]
struct Image {
    /// The resource id of the image file.
    res: ResourceId,
    /// The pixel dimensions of the image.
    dimensions: (u32, u32),
    /// The fixed width, if any.
    width: Option<Linear>,
    /// The fixed height, if any.
    height: Option<Linear>,
    /// How to align this image node in its parent.
    align: BoxAlign,
}

impl Layout for Image {
    fn layout(&self, _: &mut LayoutContext, areas: &Areas) -> Layouted {
        let Area { rem, full } = areas.current;
        let pixel_ratio = (self.dimensions.0 as f64) / (self.dimensions.1 as f64);

        let width = self.width.map(|w| w.resolve(full.width));
        let height = self.height.map(|w| w.resolve(full.height));

        let size = match (width, height) {
            (Some(width), Some(height)) => Size::new(width, height),
            (Some(width), None) => Size::new(width, width / pixel_ratio),
            (None, Some(height)) => Size::new(height * pixel_ratio, height),
            (None, None) => {
                let ratio = rem.width / rem.height;
                if ratio < pixel_ratio {
                    Size::new(rem.width, rem.width / pixel_ratio)
                } else {
                    // TODO: Fix issue with line spacing.
                    Size::new(rem.height * pixel_ratio, rem.height)
                }
            }
        };

        let mut boxed = BoxLayout::new(size);
        boxed.push(
            Point::ZERO,
            LayoutElement::Image(ImageElement { res: self.res, size }),
        );

        Layouted::Layout(boxed, self.align)
    }
}

impl From<Image> for LayoutNode {
    fn from(image: Image) -> Self {
        Self::dynamic(image)
    }
}
