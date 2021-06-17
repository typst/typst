use ::image::GenericImageView;

use super::*;
use crate::image::ImageId;
use crate::layout::{AnyNode, Constrained, Constraints, Element, Frame, Layout, LayoutContext, Regions};

/// `image`: An image.
///
/// Supports PNG and JPEG files.
///
/// # Positional parameters
/// - Path to image file: of type `string`.
///
/// # Return value
/// A template that inserts an image.
pub fn image(ctx: &mut EvalContext, args: &mut FuncArgs) -> Value {
    let path = args.eat_expect::<Spanned<String>>(ctx, "path to image file");
    let width = args.eat_named(ctx, "width");
    let height = args.eat_named(ctx, "height");

    let mut node = None;
    if let Some(path) = &path {
        if let Some((resolved, _)) = ctx.resolve(&path.v, path.span) {
            if let Some(id) = ctx.cache.image.load(ctx.loader, &resolved) {
                let img = ctx.cache.image.get(id);
                let dimensions = img.buf.dimensions();
                node = Some(ImageNode { id, dimensions, width, height });
            } else {
                ctx.diag(error!(path.span, "failed to load image"));
            }
        }
    }

    Value::template("image", move |ctx| {
        if let Some(node) = node {
            ctx.push_into_par(node);
        }
    })
}

/// An image node.
#[derive(Debug, Copy, Clone, PartialEq, Hash)]
struct ImageNode {
    /// The id of the image file.
    id: ImageId,
    /// The pixel dimensions of the image.
    dimensions: (u32, u32),
    /// The fixed width, if any.
    width: Option<Linear>,
    /// The fixed height, if any.
    height: Option<Linear>,
}

impl Layout for ImageNode {
    fn layout(&self, _: &mut LayoutContext, regions: &Regions) -> Vec<Constrained<Frame>> {
        let Regions { current, base, .. } = regions;
        let mut constraints = Constraints::new(regions.expand);
        constraints.set_base_using_linears(Spec::new(self.width, self.height), regions);

        let width = self.width.map(|w| w.resolve(base.width));
        let height = self.height.map(|w| w.resolve(base.height));

        let pixel_width = self.dimensions.0 as f64;
        let pixel_height = self.dimensions.1 as f64;
        let pixel_ratio = pixel_width / pixel_height;

        let size = match (width, height) {
            (Some(width), Some(height)) => Size::new(width, height),
            (Some(width), None) => Size::new(width, width / pixel_ratio),
            (None, Some(height)) => Size::new(height * pixel_ratio, height),
            (None, None) => {
                constraints.exact = current.to_spec().map(Some);

                let ratio = current.width / current.height;
                if ratio < pixel_ratio && current.width.is_finite() {
                    Size::new(current.width, current.width / pixel_ratio)
                } else if current.height.is_finite() {
                    // TODO: Fix issue with line spacing.
                    Size::new(current.height * pixel_ratio, current.height)
                } else {
                    // Totally unbounded region, we have to make up something.
                    Size::new(Length::pt(pixel_width), Length::pt(pixel_height))
                }
            }
        };

        let mut frame = Frame::new(size, size.height);
        frame.push(Point::zero(), Element::Image(self.id, size));
        vec![frame.constrain(constraints)]
    }
}

impl From<ImageNode> for AnyNode {
    fn from(image: ImageNode) -> Self {
        Self::new(image)
    }
}
