use std::sync::Arc;

use image::imageops::FilterType;
use image::{GenericImageView, Rgba};
use tiny_skia as sk;
use typst_library::foundations::Smart;
use typst_library::layout::Size;
use typst_library::visualize::{Image, ImageKind, ImageScaling};

use crate::{AbsExt, State};

/// Render a raster or SVG image into the canvas.
pub fn render_image(
    canvas: &mut sk::Pixmap,
    state: State,
    image: &Image,
    size: Size,
) -> Option<()> {
    let ts = state.transform;
    let view_width = size.x.to_f32();
    let view_height = size.y.to_f32();

    // For better-looking output, resize `image` to its final size before
    // painting it to `canvas`. For the math, see:
    // https://github.com/typst/typst/issues/1404#issuecomment-1598374652
    let theta = f32::atan2(-ts.kx, ts.sx);

    // To avoid division by 0, choose the one of { sin, cos } that is
    // further from 0.
    let prefer_sin = theta.sin().abs() > std::f32::consts::FRAC_1_SQRT_2;
    let scale_x =
        f32::abs(if prefer_sin { ts.kx / theta.sin() } else { ts.sx / theta.cos() });

    let aspect = (image.width() as f32) / (image.height() as f32);
    let w = (scale_x * view_width.max(aspect * view_height)).ceil() as u32;
    let h = ((w as f32) / aspect).ceil() as u32;

    let pixmap = build_texture(image, w, h)?;
    let paint_scale_x = view_width / pixmap.width() as f32;
    let paint_scale_y = view_height / pixmap.height() as f32;

    let paint = sk::Paint {
        shader: sk::Pattern::new(
            (*pixmap).as_ref(),
            sk::SpreadMode::Pad,
            sk::FilterQuality::Nearest,
            1.0,
            sk::Transform::from_scale(paint_scale_x, paint_scale_y),
        ),
        ..Default::default()
    };

    let rect = sk::Rect::from_xywh(0.0, 0.0, view_width, view_height)?;
    canvas.fill_rect(rect, &paint, ts, state.mask);

    Some(())
}

/// Prepare a texture for an image at a scaled size.
#[comemo::memoize]
fn build_texture(image: &Image, w: u32, h: u32) -> Option<Arc<sk::Pixmap>> {
    let mut texture = sk::Pixmap::new(w, h)?;
    match image.kind() {
        ImageKind::Raster(raster) => {
            scale_image(&mut texture, raster.dynamic(), image.scaling())
        }
        ImageKind::Svg(svg) => {
            let tree = svg.tree();
            let ts = tiny_skia::Transform::from_scale(
                w as f32 / tree.size().width(),
                h as f32 / tree.size().height(),
            );
            resvg::render(tree, ts, &mut texture.as_mut());
        }
    }
    Some(Arc::new(texture))
}

/// Scale a rastered image to a given size and write it into the `texture`.
fn scale_image(
    texture: &mut sk::Pixmap,
    image: &image::DynamicImage,
    scaling: Smart<ImageScaling>,
) {
    let w = texture.width();
    let h = texture.height();

    let buf;
    let resized = if (w, h) == (image.width(), image.height()) {
        // Small optimization to not allocate in case image is not resized.
        image
    } else {
        let upscale = w > image.width();
        let filter = match scaling {
            Smart::Custom(ImageScaling::Pixelated) => FilterType::Nearest,
            _ if upscale => FilterType::CatmullRom,
            _ => FilterType::Lanczos3, // downscale
        };
        buf = image.resize_exact(w, h, filter);
        &buf
    };

    for ((_, _, src), dest) in resized.pixels().zip(texture.pixels_mut()) {
        let Rgba([r, g, b, a]) = src;
        *dest = sk::ColorU8::from_rgba(r, g, b, a).premultiply();
    }
}
