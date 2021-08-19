use super::*;
use crate::image::ImageId;

use ::image::GenericImageView;

/// An image node.
#[derive(Debug, Copy, Clone, Eq, PartialEq)]
#[cfg_attr(feature = "layout-cache", derive(Hash))]
pub struct ImageNode {
    /// The id of the image file.
    pub id: ImageId,
    /// The fixed width, if any.
    pub width: Option<Linear>,
    /// The fixed height, if any.
    pub height: Option<Linear>,
}

impl Layout for ImageNode {
    fn layout(
        &self,
        ctx: &mut LayoutContext,
        &Regions { current, base, expand, .. }: &Regions,
    ) -> Vec<Constrained<Rc<Frame>>> {
        let mut constraints = Constraints::new(expand);
        constraints.set_base_if_linear(base, Spec::new(self.width, self.height));

        let width = self.width.map(|w| w.resolve(base.width));
        let height = self.height.map(|w| w.resolve(base.height));

        let dimensions = ctx.images.get(self.id).buf.dimensions();
        let pixel_width = dimensions.0 as f64;
        let pixel_height = dimensions.1 as f64;
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

impl From<ImageNode> for LayoutNode {
    fn from(image: ImageNode) -> Self {
        Self::new(image)
    }
}
