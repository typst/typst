use std::fmt::{self, Debug, Formatter};
use std::fs::File;
use std::io::BufReader;

use image::io::Reader;
use image::RgbaImage;

use crate::layout::*;
use crate::prelude::*;

/// `image`: Insert an image.
///
/// # Positional arguments
/// - The path to the image (string)
pub fn image(mut args: Args, ctx: &mut EvalContext) -> Value {
    let path = args.need::<_, Spanned<String>>(ctx, 0, "path");
    let width = args.get::<_, Linear>(ctx, "width");
    let height = args.get::<_, Linear>(ctx, "height");

    if let Some(path) = path {
        if let Ok(file) = File::open(path.v) {
            match Reader::new(BufReader::new(file))
                .with_guessed_format()
                .map_err(|err| err.into())
                .and_then(|reader| reader.decode())
                .map(|img| img.into_rgba8())
            {
                Ok(buf) => {
                    ctx.push(Image {
                        buf,
                        width,
                        height,
                        align: ctx.state.align,
                    });
                }
                Err(err) => ctx.diag(error!(path.span, "invalid image: {}", err)),
            }
        } else {
            ctx.diag(error!(path.span, "failed to open image file"));
        }
    }

    Value::None
}

/// An image node.
#[derive(Clone, PartialEq)]
struct Image {
    /// The image.
    buf: RgbaImage,
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
