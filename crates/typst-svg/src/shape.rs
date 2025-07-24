use ecow::EcoString;
use ttf_parser::OutlineBuilder;
use typst_library::layout::{Abs, Ratio, Size, Transform};
use typst_library::visualize::{
    Curve, CurveItem, FixedStroke, Geometry, LineCap, LineJoin, Paint, RelativeTo, Shape,
};

use crate::paint::ColorEncode;
use crate::{SVGRenderer, State, SvgPathBuilder};

impl SVGRenderer<'_> {
    /// Render a shape element.
    pub(super) fn render_shape(&mut self, state: State, shape: &Shape) {
        self.xml.start_element("path");
        self.xml.write_attribute("class", "typst-shape");

        if let Some(paint) = &shape.fill {
            self.write_fill(
                paint,
                shape.fill_rule,
                self.shape_fill_size(state, paint, shape),
                self.shape_paint_transform(state, paint, shape),
            );
        } else {
            self.xml.write_attribute("fill", "none");
        }

        if let Some(stroke) = &shape.stroke {
            self.write_stroke(
                stroke,
                self.shape_fill_size(state, &stroke.paint, shape),
                self.shape_paint_transform(state, &stroke.paint, shape),
            );
        }

        let path = convert_geometry_to_path(&shape.geometry);
        self.xml.write_attribute("d", &path);
        self.xml.end_element();
    }

    /// Calculate the transform of the shape's fill or stroke.
    fn shape_paint_transform(
        &self,
        state: State,
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
    fn shape_fill_size(&self, state: State, paint: &Paint, shape: &Shape) -> Size {
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
        stroke: &FixedStroke,
        size: Size,
        fill_transform: Transform,
    ) {
        match &stroke.paint {
            Paint::Solid(color) => self.xml.write_attribute("stroke", &color.encode()),
            Paint::Gradient(gradient) => {
                let id = self.push_gradient(gradient, size, fill_transform);
                self.xml.write_attribute_fmt("stroke", format_args!("url(#{id})"));
            }
            Paint::Tiling(tiling) => {
                let id = self.push_tiling(tiling, size, fill_transform);
                self.xml.write_attribute_fmt("stroke", format_args!("url(#{id})"));
            }
        }

        self.xml.write_attribute("stroke-width", &stroke.thickness.to_pt());
        self.xml.write_attribute(
            "stroke-linecap",
            match stroke.cap {
                LineCap::Butt => "butt",
                LineCap::Round => "round",
                LineCap::Square => "square",
            },
        );
        self.xml.write_attribute(
            "stroke-linejoin",
            match stroke.join {
                LineJoin::Miter => "miter",
                LineJoin::Round => "round",
                LineJoin::Bevel => "bevel",
            },
        );
        self.xml
            .write_attribute("stroke-miterlimit", &stroke.miter_limit.get());
        if let Some(dash) = &stroke.dash {
            self.xml.write_attribute("stroke-dashoffset", &dash.phase.to_pt());
            self.xml.write_attribute(
                "stroke-dasharray",
                &dash
                    .array
                    .iter()
                    .map(|dash| dash.to_pt().to_string())
                    .collect::<Vec<_>>()
                    .join(" "),
            );
        }
    }
}

/// Convert a geometry to an SVG path.
#[comemo::memoize]
fn convert_geometry_to_path(geometry: &Geometry) -> EcoString {
    let mut builder = SvgPathBuilder::default();
    match geometry {
        Geometry::Line(t) => {
            builder.move_to(0.0, 0.0);
            builder.line_to(t.x.to_pt() as f32, t.y.to_pt() as f32);
        }
        Geometry::Rect(rect) => {
            let x = rect.x.to_pt() as f32;
            let y = rect.y.to_pt() as f32;
            builder.rect(x, y);
        }
        Geometry::Curve(p) => return convert_curve(p),
    };
    builder.0
}

pub fn convert_curve(curve: &Curve) -> EcoString {
    let mut builder = SvgPathBuilder::default();
    for item in &curve.0 {
        match item {
            CurveItem::Move(m) => builder.move_to(m.x.to_pt() as f32, m.y.to_pt() as f32),
            CurveItem::Line(l) => builder.line_to(l.x.to_pt() as f32, l.y.to_pt() as f32),
            CurveItem::Cubic(c1, c2, t) => builder.curve_to(
                c1.x.to_pt() as f32,
                c1.y.to_pt() as f32,
                c2.x.to_pt() as f32,
                c2.y.to_pt() as f32,
                t.x.to_pt() as f32,
                t.y.to_pt() as f32,
            ),
            CurveItem::Close => builder.close(),
        }
    }
    builder.0
}
