use image::GenericImageView;

use crate::env::{ImageResource, ResourceId};
use crate::layout::*;
use crate::prelude::*;

/// `rect`: Layout content into a rectangle that also might have a fill.
///
/// # Named arguments
/// - Width of the box:  `width`, of type `linear` relative to parent width.
/// - Height of the box: `height`, of type `linear` relative to parent height.
pub fn rect(ctx: &mut EvalContext, args: &mut Args) -> Value {
    let snapshot = ctx.state.clone();

    let width = args.get(ctx, "width");
    let height = args.get(ctx, "height");
    let color = args.get(ctx, "color");

    let dirs = ctx.state.dirs;
    let align = ctx.state.align;

    ctx.start_content_group();

    if let Some(body) = args.find::<ValueTemplate>(ctx) {
        body.eval(ctx);
    }

    let children = ctx.end_content_group();

    let fill_if = |c| if c { Expansion::Fill } else { Expansion::Fit };
    let expand = Spec::new(fill_if(width.is_some()), fill_if(height.is_some()));

    ctx.push(NodeRect {
        width,
        height,
        color,
        child: NodeStack { dirs, align, expand, children }.into(),
    });

    ctx.state = snapshot;
    Value::None
}

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
        let loaded = ctx.env.resources.load(path.v, ImageResource::parse);
        if let Some((res, img)) = loaded {
            let dimensions = img.buf.dimensions();
            ctx.push(NodeImage {
                res,
                dimensions,
                width,
                height,
                align: ctx.state.align,
            });
        } else {
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
        let Areas { current, full, .. } = areas;

        let pixel_width = self.dimensions.0 as f64;
        let pixel_height = self.dimensions.1 as f64;
        let pixel_ratio = pixel_width / pixel_height;

        let width = self.width.map(|w| w.resolve(full.width));
        let height = self.height.map(|w| w.resolve(full.height));

        let size = match (width, height) {
            (Some(width), Some(height)) => Size::new(width, height),
            (Some(width), None) => Size::new(width, width / pixel_ratio),
            (None, Some(height)) => Size::new(height * pixel_ratio, height),
            (None, None) => {
                let ratio = current.width / current.height;
                if ratio < pixel_ratio && current.width.is_finite() {
                    Size::new(current.width, current.width / pixel_ratio)
                } else if current.height.is_finite() {
                    // TODO: Fix issue with line spacing.
                    Size::new(current.height * pixel_ratio, current.height)
                } else {
                    // Totally unbounded area, we have to make up something.
                    Size::new(Length::pt(pixel_width), Length::pt(pixel_height))
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
