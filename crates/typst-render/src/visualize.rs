use image::imageops::FilterType;
use image::{GenericImageView, Rgba};
use std::sync::Arc;
use tiny_skia as sk;
use typst::layout::{Abs, Axes, Point, Ratio, Size};
use typst::visualize::{
    DashPattern, FixedStroke, Geometry, Image, ImageKind, LineCap, LineJoin, Path,
    PathItem, Shape,
};

use crate::{paint, AbsExt, State};

/// Render a geometrical shape into the canvas.
pub fn render_shape(canvas: &mut sk::Pixmap, state: State, shape: &Shape) -> Option<()> {
    let ts = state.transform;
    let path = match shape.geometry {
        Geometry::Line(target) => {
            let mut builder = sk::PathBuilder::new();
            builder.line_to(target.x.to_f32(), target.y.to_f32());
            builder.finish()?
        }
        Geometry::Rect(size) => {
            let w = size.x.to_f32();
            let h = size.y.to_f32();
            let rect = if w < 0.0 || h < 0.0 {
                // Skia doesn't normally allow for negative dimensions, but
                // Typst supports them, so we apply a transform if needed
                // Because this operation is expensive according to tiny-skia's
                // docs, we prefer to not apply it if not needed
                let transform = sk::Transform::from_scale(w.signum(), h.signum());
                let rect = sk::Rect::from_xywh(0.0, 0.0, w.abs(), h.abs())?;
                rect.transform(transform)?
            } else {
                sk::Rect::from_xywh(0.0, 0.0, w, h)?
            };

            sk::PathBuilder::from_rect(rect)
        }
        Geometry::Path(ref path) => convert_path(path)?,
    };

    if let Some(fill) = &shape.fill {
        let mut pixmap = None;
        let mut paint: sk::Paint = paint::to_sk_paint(
            fill,
            state,
            shape.geometry.bbox_size(),
            false,
            None,
            &mut pixmap,
            None,
        );

        if matches!(shape.geometry, Geometry::Rect(_)) {
            paint.anti_alias = false;
        }

        let rule = sk::FillRule::default();
        canvas.fill_path(&path, &paint, rule, ts, state.mask);
    }

    if let Some(FixedStroke { paint, thickness, cap, join, dash, miter_limit }) =
        &shape.stroke
    {
        let width = thickness.to_f32();

        // Don't draw zero-pt stroke.
        if width > 0.0 {
            let dash = dash.as_ref().and_then(to_sk_dash_pattern);

            let bbox = shape.geometry.bbox_size();
            let offset_bbox = (!matches!(shape.geometry, Geometry::Line(..)))
                .then(|| offset_bounding_box(bbox, *thickness))
                .unwrap_or(bbox);

            let fill_transform =
                (!matches!(shape.geometry, Geometry::Line(..))).then(|| {
                    sk::Transform::from_translate(
                        -thickness.to_f32(),
                        -thickness.to_f32(),
                    )
                });

            let gradient_map =
                (!matches!(shape.geometry, Geometry::Line(..))).then(|| {
                    (
                        Point::new(
                            -*thickness * state.pixel_per_pt as f64,
                            -*thickness * state.pixel_per_pt as f64,
                        ),
                        Axes::new(
                            Ratio::new(offset_bbox.x / bbox.x),
                            Ratio::new(offset_bbox.y / bbox.y),
                        ),
                    )
                });

            let mut pixmap = None;
            let paint = paint::to_sk_paint(
                paint,
                state,
                offset_bbox,
                false,
                fill_transform,
                &mut pixmap,
                gradient_map,
            );
            let stroke = sk::Stroke {
                width,
                line_cap: to_sk_line_cap(*cap),
                line_join: to_sk_line_join(*join),
                dash,
                miter_limit: miter_limit.get() as f32,
            };
            canvas.stroke_path(&path, &paint, &stroke, ts, state.mask);
        }
    }

    Some(())
}

/// Convert a Typst path into a tiny-skia path.
pub fn convert_path(path: &Path) -> Option<sk::Path> {
    let mut builder = sk::PathBuilder::new();
    for elem in &path.0 {
        match elem {
            PathItem::MoveTo(p) => {
                builder.move_to(p.x.to_f32(), p.y.to_f32());
            }
            PathItem::LineTo(p) => {
                builder.line_to(p.x.to_f32(), p.y.to_f32());
            }
            PathItem::CubicTo(p1, p2, p3) => {
                builder.cubic_to(
                    p1.x.to_f32(),
                    p1.y.to_f32(),
                    p2.x.to_f32(),
                    p2.y.to_f32(),
                    p3.x.to_f32(),
                    p3.y.to_f32(),
                );
            }
            PathItem::ClosePath => {
                builder.close();
            }
        };
    }
    builder.finish()
}

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
        ImageKind::Svg(svg) => unsafe {
            svg.with(|tree| {
                let ts = tiny_skia::Transform::from_scale(
                    w as f32 / tree.size.width(),
                    h as f32 / tree.size.height(),
                );
                resvg::render(tree, ts, &mut pixmap.as_mut())
            });
        },
    }
    Some(Arc::new(pixmap))
}

fn offset_bounding_box(bbox: Size, stroke_width: Abs) -> Size {
    Size::new(bbox.x + stroke_width * 2.0, bbox.y + stroke_width * 2.0)
}

pub fn to_sk_line_cap(cap: LineCap) -> sk::LineCap {
    match cap {
        LineCap::Butt => sk::LineCap::Butt,
        LineCap::Round => sk::LineCap::Round,
        LineCap::Square => sk::LineCap::Square,
    }
}

pub fn to_sk_line_join(join: LineJoin) -> sk::LineJoin {
    match join {
        LineJoin::Miter => sk::LineJoin::Miter,
        LineJoin::Round => sk::LineJoin::Round,
        LineJoin::Bevel => sk::LineJoin::Bevel,
    }
}

pub fn to_sk_dash_pattern(pattern: &DashPattern<Abs, Abs>) -> Option<sk::StrokeDash> {
    // tiny-skia only allows dash patterns with an even number of elements,
    // while pdf allows any number.
    let pattern_len = pattern.array.len();
    let len = if pattern_len % 2 == 1 { 2 * pattern_len } else { pattern_len };
    let dash_array = pattern.array.iter().map(|l| l.to_f32()).cycle().take(len).collect();
    sk::StrokeDash::new(dash_array, pattern.phase.to_f32())
}
