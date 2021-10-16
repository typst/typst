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
        regions: &Regions,
    ) -> Vec<Constrained<Rc<Frame>>> {
        let img = ctx.images.get(self.id);
        let pixel_size = Spec::new(img.width() as f64, img.height() as f64);
        let pixel_ratio = pixel_size.x / pixel_size.y;

        let width = self.width.map(|w| w.resolve(regions.base.w));
        let height = self.height.map(|w| w.resolve(regions.base.h));

        let mut cts = Constraints::new(regions.expand);
        cts.set_base_if_linear(regions.base, Spec::new(self.width, self.height));

        let size = match (width, height) {
            (Some(width), Some(height)) => Size::new(width, height),
            (Some(width), None) => Size::new(width, width / pixel_ratio),
            (None, Some(height)) => Size::new(height * pixel_ratio, height),
            (None, None) => {
                cts.exact.x = Some(regions.current.w);
                if regions.current.w.is_finite() {
                    // Fit to width.
                    Size::new(regions.current.w, regions.current.w / pixel_ratio)
                } else {
                    // Unbounded width, we have to make up something,
                    // so it is 1pt per pixel.
                    pixel_size.map(Length::pt).to_size()
                }
            }
        };

        let mut frame = Frame::new(size, size.h);
        frame.push(Point::zero(), Element::Image(self.id, size));
        vec![frame.constrain(cts)]
    }
}

impl From<ImageNode> for LayoutNode {
    fn from(image: ImageNode) -> Self {
        Self::new(image)
    }
}
