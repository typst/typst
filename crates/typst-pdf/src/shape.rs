use krilla::geom::{Path, PathBuilder, Rect};
use krilla::surface::Surface;
use typst_library::diag::SourceResult;
use typst_library::pdf::ArtifactKind;
use typst_library::visualize::{Geometry, Shape};
use typst_syntax::Span;

use crate::convert::{FrameContext, GlobalContext};
use crate::util::{AbsExt, TransformExt, convert_path};
use crate::{paint, tags};

#[typst_macros::time(name = "handle shape")]
pub(crate) fn handle_shape(
    fc: &mut FrameContext,
    shape: &Shape,
    surface: &mut Surface,
    gc: &mut GlobalContext,
    span: Span,
) -> SourceResult<()> {
    let mut handle = tags::start_artifact(gc, surface, ArtifactKind::Other);
    let surface = handle.surface();

    surface.set_location(span.into_raw());
    surface.push_transform(&fc.state().transform().to_krilla());

    if let Some(path) = convert_geometry(&shape.geometry) {
        let fill = if let Some(paint) = &shape.fill {
            Some(paint::convert_fill(
                gc,
                paint,
                shape.fill_rule,
                false,
                surface,
                fc.state(),
                shape.geometry.bbox_size(),
            )?)
        } else {
            None
        };

        let stroke = shape.stroke.as_ref().and_then(|stroke| {
            if stroke.thickness.to_f32() > 0.0 { Some(stroke) } else { None }
        });

        let stroke = if let Some(stroke) = &stroke {
            let stroke = paint::convert_stroke(
                gc,
                stroke,
                false,
                surface,
                fc.state(),
                shape.geometry.bbox_size(),
            )?;

            Some(stroke)
        } else {
            None
        };

        // Otherwise, krilla will by default fill with a black paint.
        if fill.is_some() || stroke.is_some() {
            surface.set_fill(fill);
            surface.set_stroke(stroke);
            surface.draw_path(&path);
        }
    }

    surface.pop();
    surface.reset_location();

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
                // krilla doesn't normally allow for negative dimensions, but
                // Typst supports them, so we apply a transform if needed.
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
