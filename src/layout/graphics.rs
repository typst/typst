use std::fmt::{self, Debug, Formatter};

use super::*;

/// An image node.
#[derive(Clone, PartialEq)]
pub struct Image {
    /// The image.
    pub buf: RgbaImage,
    /// The fixed width, if any.
    pub width: Option<Linear>,
    /// The fixed height, if any.
    pub height: Option<Linear>,
    /// How to align this image node in its parent.
    pub align: BoxAlign,
}

impl Layout for Image {
    fn layout(&self, _: &mut LayoutContext, areas: &Areas) -> Layouted {
        let Area { rem, full } = areas.current;
        let (pixel_width, pixel_height) = self.buf.dimensions();
        let pixel_ratio = (pixel_width as f64) / (pixel_height as f64);

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
            LayoutElement::Image(ImageElement { buf: self.buf.clone(), size }),
        );

        Layouted::Layout(boxed, self.align)
    }
}

impl Debug for Image {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        f.pad("Image")
    }
}

impl From<Image> for LayoutNode {
    fn from(image: Image) -> Self {
        Self::dynamic(image)
    }
}
