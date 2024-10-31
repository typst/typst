use std::sync::Arc;

use image::imageops::FilterType;
use image::{GenericImageView, Rgba};
use tiny_skia as sk;
use typst::layout::Size;
use typst::visualize::{Image, ImageKind};

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

    let pixmap = scaled_texture(image, w, h)?;
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
fn scaled_texture(image: &Image, w: u32, h: u32) -> Option<Arc<sk::Pixmap>> {
    let mut pixmap = sk::Pixmap::new(w, h)?;
    match image.kind() {
        ImageKind::Raster(raster) => {
            let downscale = w < raster.width();
            let filter =
                if downscale { FilterType::Lanczos3 } else { FilterType::CatmullRom };
            let buf = raster.dynamic().resize(w, h, filter);
            for ((_, _, src), dest) in buf.pixels().zip(pixmap.pixels_mut()) {
                let Rgba([r, g, b, a]) = src;
                *dest = sk::ColorU8::from_rgba(r, g, b, a).premultiply();
            }
        }
        // Safety: We do not keep any references to tree nodes beyond the scope
        // of `with`.
        ImageKind::Svg(svg) => {
            let tree = svg.tree();
            let ts = tiny_skia::Transform::from_scale(
                w as f32 / tree.size().width(),
                h as f32 / tree.size().height(),
            );
            resvg::render(tree, ts, &mut pixmap.as_mut())
        }
    }
    Some(Arc::new(pixmap))
}
