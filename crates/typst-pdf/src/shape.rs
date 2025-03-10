use crate::convert::{FrameContext, GlobalContext};
use crate::paint;
use crate::util::{convert_path, AbsExt, TransformExt};
use krilla::geom::Rect;
use krilla::path::{Path, PathBuilder};
use krilla::surface::Surface;
use typst_library::diag::SourceResult;
use typst_library::visualize::{Geometry, Shape};

pub(crate) fn handle_shape(
    fc: &mut FrameContext,
    shape: &Shape,
    surface: &mut Surface,
    gc: &mut GlobalContext,
) -> SourceResult<()> {
    surface.push_transform(&fc.state().transform().to_krilla());

    if let Some(path) = convert_geometry(&shape.geometry) {
        if let Some(paint) = &shape.fill {
            let fill = paint::convert_fill(
                gc,
                paint,
                shape.fill_rule,
                false,
                surface,
                fc.state(),
                shape.geometry.bbox_size(),
            )?;

            surface.fill_path(&path, fill);
        }

        let stroke = shape.stroke.as_ref().and_then(|stroke| {
            if stroke.thickness.to_f32() > 0.0 {
                Some(stroke)
            } else {
                None
            }
        });

        if let Some(stroke) = &stroke {
            let stroke = paint::convert_stroke(
                gc,
                stroke,
                false,
                surface,
                fc.state(),
                shape.geometry.bbox_size(),
            )?;

            surface.stroke_path(&path, stroke);
        }
    }

    surface.pop();

    Ok(())
}

fn convert_geometry(geometry: &Geometry) -> Option<Path> {
    let mut path_builder = PathBuilder::new();

    match geometry {
        Geometry::Line(l) => {
            path_builder.move_to(0.0, 0.0);
            path_builder.line_to(l.x.to_f32(), l.y.to_f32());
        }
        Geometry::Rect(size) => {
            let w = size.x.to_f32();
            let h = size.y.to_f32();
            let rect = if w < 0.0 || h < 0.0 {
                // Skia doesn't normally allow for negative dimensions, but
                // Typst supports them, so we apply a transform if needed
                // Because this operation is expensive according to tiny-skia's
                // docs, we prefer to not apply it if not needed
                let transform =
                    krilla::geom::Transform::from_scale(w.signum(), h.signum());
                Rect::from_xywh(0.0, 0.0, w.abs(), h.abs())
                    .and_then(|rect| rect.transform(transform))
            } else {
                Rect::from_xywh(0.0, 0.0, w, h)
            };

            if let Some(rect) = rect {
                path_builder.push_rect(rect);
            }
        }
        Geometry::Curve(c) => {
            convert_path(c, &mut path_builder);
        }
    }

    path_builder.finish()
}
