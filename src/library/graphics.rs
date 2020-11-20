use std::fs::File;
use std::io::BufReader;

use image::io::Reader;

use crate::layout::Image;
use crate::prelude::*;

/// `image`: Include an image.
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
