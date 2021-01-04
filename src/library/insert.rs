use image::GenericImageView;

use crate::env::{ImageResource, ResourceId};
use crate::layout::*;
use crate::prelude::*;

/// `image`: Insert an image.
///
/// Supports PNG and JPEG files.
///
/// # Positional arguments
/// - Path to image file: of type `string`.
pub fn image(ctx: &mut EvalContext, args: &mut Args) -> Value {
    let path = args.require::<Spanned<String>>(ctx, "path to image file");
    let width = args.get(ctx, "width");
    let height = args.get(ctx, "height");

    if let Some(path) = path {
        let mut env = ctx.env.borrow_mut();
        let loaded = env.resources.load(path.v, ImageResource::parse);

        if let Some((res, img)) = loaded {
            let dimensions = img.buf.dimensions();
            drop(env);
            ctx.push(NodeImage {
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
struct NodeImage {
    /// The resource id of the image file.
    res: ResourceId,
    /// The pixel dimensions of the image.
    dimensions: (u32, u32),
    /// The fixed width, if any.
    width: Option<Linear>,
    /// The fixed height, if any.
    height: Option<Linear>,
    /// How to align this image node in its parent.
    align: ChildAlign,
}

impl Layout for NodeImage {
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

        let mut frame = Frame::new(size);
        frame.push(Point::ZERO, Element::Image(Image { res: self.res, size }));

        Layouted::Frame(frame, self.align)
    }
}

impl From<NodeImage> for NodeAny {
    fn from(image: NodeImage) -> Self {
        Self::new(image)
    }
}
