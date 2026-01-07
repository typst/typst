use ecow::EcoString;
use typst_library::layout::{Abs, Point, Ratio, Size, Transform};
use typst_library::visualize::{
    Curve, CurveItem, FixedStroke, Geometry, LineCap, LineJoin, Paint, RelativeTo, Shape,
};

use crate::path::SvgPathBuilder;
use crate::write::{SvgElem, SvgTransform, SvgUrl, SvgWrite};
use crate::{SVGRenderer, State};

impl SVGRenderer<'_> {
    /// Render a shape element.
    pub(super) fn render_shape(
        &mut self,
        svg: &mut SvgElem,
        state: &State,
        shape: &Shape,
    ) {
        let svg = &mut svg.elem("path");

        if let Some(paint) = &shape.fill {
            self.write_fill(
                svg,
                paint,
                shape.fill_rule,
                self.shape_fill_size(state, paint, shape).aspect_ratio(),
                self.shape_paint_transform(state, paint, shape),
            );
        } else {
            svg.attr("fill", "none");
        }

        if let Some(stroke) = &shape.stroke {
            self.write_stroke(
                svg,
                stroke,
                self.shape_fill_size(state, &stroke.paint, shape).aspect_ratio(),
                self.shape_paint_transform(state, &stroke.paint, shape),
            );
        }

        if !state.transform.is_identity() {
            svg.attr("transform", SvgTransform(state.transform));
        }

        let path = convert_geometry_to_path(&shape.geometry);
        svg.attr("d", path);
    }

    /// Calculate the transform of the shape's fill or stroke.
    fn shape_paint_transform(
        &self,
        state: &State,
        paint: &Paint,
        shape: &Shape,
    ) -> Transform {
        let mut shape_size = shape.geometry.bbox_size();
        // Edge cases for strokes.
        if shape_size.x.to_pt() == 0.0 {
            shape_size.x = Abs::pt(1.0);
        }

        if shape_size.y.to_pt() == 0.0 {
            shape_size.y = Abs::pt(1.0);
        }

        if let Paint::Gradient(gradient) = paint {
            match gradient.unwrap_relative(false) {
                RelativeTo::Self_ => Transform::scale(
                    Ratio::new(shape_size.x.to_pt()),
                    Ratio::new(shape_size.y.to_pt()),
                ),
                RelativeTo::Parent => Transform::scale(
                    Ratio::new(state.size.x.to_pt()),
                    Ratio::new(state.size.y.to_pt()),
                )
                .post_concat(state.transform.invert().unwrap()),
            }
        } else if let Paint::Tiling(tiling) = paint {
            match tiling.unwrap_relative(false) {
                RelativeTo::Self_ => Transform::identity(),
                RelativeTo::Parent => state.transform.invert().unwrap(),
            }
        } else {
            Transform::identity()
        }
    }

    /// Calculate the size of the shape's fill.
    fn shape_fill_size(&self, state: &State, paint: &Paint, shape: &Shape) -> Size {
        let mut shape_size = shape.geometry.bbox_size();
        // Edge cases for strokes.
        if shape_size.x.to_pt() == 0.0 {
            shape_size.x = Abs::pt(1.0);
        }

        if shape_size.y.to_pt() == 0.0 {
            shape_size.y = Abs::pt(1.0);
        }

        if let Paint::Gradient(gradient) = paint {
            match gradient.unwrap_relative(false) {
                RelativeTo::Self_ => shape_size,
                RelativeTo::Parent => state.size,
            }
        } else {
            shape_size
        }
    }

    /// Write a stroke attribute.
    pub(super) fn write_stroke(
        &mut self,
        svg: &mut SvgElem,
        stroke: &FixedStroke,
        aspect_ratio: Ratio,
        fill_transform: Transform,
    ) {
        match &stroke.paint {
            Paint::Solid(color) => {
                svg.attr("stroke", color);
            }
            Paint::Gradient(gradient) => {
                let id = self.push_gradient(gradient, aspect_ratio, fill_transform);
                svg.attr("stroke", SvgUrl(id));
            }
            Paint::Tiling(tiling) => {
                let id = self.push_tiling(tiling, fill_transform);
                svg.attr("stroke", SvgUrl(id));
            }
        }

        svg.attr("stroke-width", stroke.thickness.to_pt());
        svg.attr(
            "stroke-linecap",
            match stroke.cap {
                LineCap::Butt => "butt",
                LineCap::Round => "round",
                LineCap::Square => "square",
            },
        );
        svg.attr(
            "stroke-linejoin",
            match stroke.join {
                LineJoin::Miter => "miter",
                LineJoin::Round => "round",
                LineJoin::Bevel => "bevel",
            },
        );
        svg.attr("stroke-miterlimit", stroke.miter_limit.get());
        if let Some(dash) = &stroke.dash {
            svg.attr("stroke-dashoffset", dash.phase.to_pt());
            svg.attr_with("stroke-dasharray", |attr| {
                attr.push_nums(dash.array.iter().map(|dash| dash.to_pt()));
            });
        }
    }
}

/// Convert a geometry to an SVG path.
#[comemo::memoize]
fn convert_geometry_to_path(geometry: &Geometry) -> EcoString {
    let mut builder = SvgPathBuilder::with_translate(Point::zero());
    match geometry {
        &Geometry::Line(t) => builder.line_to(t),
        &Geometry::Rect(size) => builder.rect(size),
        Geometry::Curve(p) => {
            return convert_curve(Point::zero(), p);
        }
    };
    builder.finsish()
}

pub fn convert_curve(initial_point: Point, curve: &Curve) -> EcoString {
    let mut builder = SvgPathBuilder::with_translate(initial_point);
    for item in curve.0.iter() {
        match *item {
            CurveItem::Move(pos) => builder.move_to(pos),
            CurveItem::Line(pos) => builder.line_to(pos),
            CurveItem::Cubic(p1, p2, p3) => builder.curve_to(p1, p2, p3),
            CurveItem::Close => builder.close(),
        }
    }
    builder.finsish()
}
