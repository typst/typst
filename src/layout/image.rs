use super::*;
use crate::image::ImageId;

/// An image node.
#[derive(Debug)]
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

        let width = self.width.map(|w| w.resolve(base.w));
        let height = self.height.map(|w| w.resolve(base.h));

        let img = ctx.images.get(self.id);
        let pixel_size = Spec::new(img.width() as f64, img.height() as f64);
        let pixel_ratio = pixel_size.x / pixel_size.y;

        let size = match (width, height) {
            (Some(width), Some(height)) => Size::new(width, height),
            (Some(width), None) => Size::new(width, width / pixel_ratio),
            (None, Some(height)) => Size::new(height * pixel_ratio, height),
            (None, None) => {
                constraints.exact.x = Some(current.w);
                if current.w.is_finite() {
                    Size::new(current.w, current.w / pixel_ratio)
                } else {
                    // Totally unbounded region, we have to make up something,
                    // so it is 1pt per pixel.
                    pixel_size.map(Length::pt).to_size()
                }
            }
        };

        let mut frame = Frame::new(size, size.h);
        frame.push(Point::zero(), Element::Image(self.id, size));
        vec![frame.constrain(constraints)]
    }
}

impl From<ImageNode> for LayoutNode {
    fn from(image: ImageNode) -> Self {
        Self::new(image)
    }
}
