use tiny_skia as sk;
use typst_library::layout::{Abs, Axes, Ratio};
use typst_library::visualize::{
    Curve, CurveItem, DashPattern, FillRule, FixedStroke, Geometry, LineCap, LineJoin,
    Shape,
};

use crate::{AbsExt, State, paint};

/// Render a geometrical shape into the canvas.
pub fn render_shape(canvas: &mut sk::Pixmap, state: State, shape: &Shape) -> Option<()> {
    let ts = state.transform;
    let path = match &shape.geometry {
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
        Geometry::Curve(curve) => convert_curve(curve)?,
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

        let rule = match shape.fill_rule {
            FillRule::NonZero => sk::FillRule::Winding,
            FillRule::EvenOdd => sk::FillRule::EvenOdd,
        };
        canvas.fill_path(&path, &paint, rule, ts, state.mask);
    }

    if let Some(FixedStroke { paint, thickness, cap, join, dash, miter_limit }) =
        &shape.stroke
    {
        let width = thickness.to_f32();

        // Don't draw zero-pt stroke.
        if width > 0.0 {
            let dash = dash.as_ref().and_then(to_sk_dash_pattern);
            let bbox = shape.geometry.bbox(shape.stroke.as_ref());
            let fill_transform =
                sk::Transform::from_translate(bbox.min.x.to_f32(), bbox.min.y.to_f32());
            let gradient_map = match shape.geometry {
                // Special handling for fill of rectangles (mirrors gradients for negative sizes)
                Geometry::Rect(rect) => Some((
                    bbox.min * state.pixel_per_pt as f64,
                    Axes::new(
                        Ratio::new(bbox.size().x / rect.x),
                        Ratio::new(bbox.size().y / rect.y),
                    ),
                )),
                _ => None,
            };

            let mut pixmap = None;
            let paint = paint::to_sk_paint(
                paint,
                state,
                bbox.size(),
                false,
                Some(fill_transform),
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

/// Convert a Typst curve into a tiny-skia path.
pub fn convert_curve(curve: &Curve) -> Option<sk::Path> {
    let mut builder = sk::PathBuilder::new();
    for elem in &curve.0 {
        match elem {
            CurveItem::Move(p) => {
                builder.move_to(p.x.to_f32(), p.y.to_f32());
            }
            CurveItem::Line(p) => {
                builder.line_to(p.x.to_f32(), p.y.to_f32());
            }
            CurveItem::Cubic(p1, p2, p3) => {
                builder.cubic_to(
                    p1.x.to_f32(),
                    p1.y.to_f32(),
                    p2.x.to_f32(),
                    p2.y.to_f32(),
                    p3.x.to_f32(),
                    p3.y.to_f32(),
                );
            }
            CurveItem::Close => {
                builder.close();
            }
        };
    }
    builder.finish()
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

pub fn to_sk_dash_pattern(dash: &DashPattern<Abs, Abs>) -> Option<sk::StrokeDash> {
    // tiny-skia only allows dash patterns with an even number of elements,
    // while pdf allows any number.
    let pattern_len = dash.array.len();
    let len = if pattern_len % 2 == 1 { 2 * pattern_len } else { pattern_len };
    let dash_array = dash.array.iter().map(|l| l.to_f32()).cycle().take(len).collect();
    sk::StrokeDash::new(dash_array, dash.phase.to_f32())
}
