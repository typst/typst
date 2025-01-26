use kurbo::{
    BezPath, CubicBez, Line, ParamCurve, ParamCurveDeriv, PathSeg, QuadBez, Shape, Vec2,
};
use ttf_parser::{GlyphId, OutlineBuilder};
use typst_library::layout::{Abs, Em, Frame, FrameItem, Point, Size};
use typst_library::text::{
    BottomEdge, DecoLine, Decoration, TextEdgeBounds, TextItem, TopEdge,
};
use typst_library::visualize::{FixedStroke, Geometry};
use typst_syntax::Span;

use crate::shapes::styled_rect;

/// Add line decorations to a single run of shaped text.
pub fn decorate(
    frame: &mut Frame,
    deco: &Decoration,
    text: &TextItem,
    width: Abs,
    shift: Abs,
    pos: Point,
) {
    let font_metrics = text.font.metrics();

    if let DecoLine::Highlight { fill, stroke, top_edge, bottom_edge, radius } =
        &deco.line
    {
        let (top, bottom) = determine_edges(text, *top_edge, *bottom_edge);
        let size = Size::new(width + 2.0 * deco.extent, top + bottom);
        let rects = styled_rect(size, radius, fill.clone(), stroke);
        let origin = Point::new(pos.x - deco.extent, pos.y - top - shift);
        frame.prepend_multiple(
            rects
                .into_iter()
                .map(|shape| (origin, FrameItem::Shape(shape, Span::detached()))),
        );
        return;
    }

    let (stroke, metrics, offset, evade, background) = match &deco.line {
        DecoLine::Strikethrough { stroke, offset, background } => {
            (stroke, font_metrics.strikethrough, offset, false, *background)
        }
        DecoLine::Overline { stroke, offset, evade, background } => {
            (stroke, font_metrics.overline, offset, *evade, *background)
        }
        DecoLine::Underline { stroke, offset, evade, background } => {
            (stroke, font_metrics.underline, offset, *evade, *background)
        }
        _ => return,
    };

    let offset = offset.unwrap_or(-metrics.position.at(text.size)) - shift;
    let stroke = stroke.clone().unwrap_or(FixedStroke::from_pair(
        text.fill.as_decoration(),
        metrics.thickness.at(text.size),
    ));

    let gap_padding = 0.08 * text.size;
    let min_width = 0.162 * text.size;

    let start = pos.x - deco.extent;
    let end = pos.x + width + deco.extent;

    let mut push_segment = |from: Abs, to: Abs, prepend: bool| {
        let origin = Point::new(from, pos.y + offset);
        let target = Point::new(to - from, Abs::zero());

        if target.x >= min_width || !evade {
            let shape = Geometry::Line(target).stroked(stroke.clone());

            if prepend {
                frame.prepend(origin, FrameItem::Shape(shape, Span::detached()));
            } else {
                frame.push(origin, FrameItem::Shape(shape, Span::detached()));
            }
        }
    };

    if !evade {
        push_segment(start, end, background);
        return;
    }

    let line = Line::new(
        kurbo::Point::new(pos.x.to_raw(), offset.to_raw()),
        kurbo::Point::new((pos.x + width).to_raw(), offset.to_raw()),
    );

    let mut x = pos.x;
    let mut intersections = vec![];

    for glyph in text.glyphs.iter() {
        let dx = glyph.x_offset.at(text.size) + x;
        let mut builder =
            BezPathBuilder::new(font_metrics.units_per_em, text.size, dx.to_raw());

        let bbox = text.font.ttf().outline_glyph(GlyphId(glyph.id), &mut builder);
        let path = builder.finish();

        x += glyph.x_advance.at(text.size);

        // Only do the costly segments intersection test if the line
        // intersects the bounding box.
        let intersect = bbox.is_some_and(|bbox| {
            let y_min = -text.font.to_em(bbox.y_max).at(text.size);
            let y_max = -text.font.to_em(bbox.y_min).at(text.size);
            offset >= y_min && offset <= y_max
        });

        if intersect {
            // Move path and line such that all coordinates are positive.
            // Workaround for https://github.com/linebender/kurbo/issues/411
            let offset = Vec2::new(
                -line.p0.x.min(path.bounding_box().min_x()),
                -line.p0.y.min(path.bounding_box().min_y()),
            );

            // Find all intersections of segments with the line.
            intersections.extend(path.segments().flat_map(|seg| {
                let intersections = seg.translate(offset).intersect_line(line + offset);
                intersections.into_iter().map(move |is| {
                    // Check whether the tangent line at the intersection point
                    // is horizontal, i.e. the line is tangential to the glyph.
                    let deriv = seg.deriv_at(is.segment_t);
                    let tangential = (deriv.y / deriv.x).abs() < 1e-6;
                    (Abs::raw(line.eval(is.line_t).x), tangential)
                })
            }));
        }
    }

    // Add start and end points, taking padding into account.
    intersections.push((start - gap_padding, false));
    intersections.push((end + gap_padding, false));
    // When emitting the decorative line segments, we move from left to
    // right. The intersections are not necessarily in this order, yet.
    intersections.sort();
    // When the line hits the glyph just where two path segments meet, kurbo
    // may report two intersections very close to each other. We remove these
    // duplicates here, so they don't mess with the "inside" flag below.
    intersections.dedup_by(|(l, _), (r, _)| l.approx_eq(*r));

    let mut inside = false;
    for edge in intersections.windows(2) {
        let (l, _) = edge[0];
        let (r, tangential) = edge[1];

        if !inside && r - l >= gap_padding {
            // Only draw the segment if it's outside the glyph and the
            // intersections points are not too close to each other.
            push_segment(l + gap_padding, r - gap_padding, background);
        }

        if !tangential {
            inside = !inside;
        }
    }
}

// Return the top/bottom edge of the text given the metric of the font.
fn determine_edges(
    text: &TextItem,
    top_edge: TopEdge,
    bottom_edge: BottomEdge,
) -> (Abs, Abs) {
    let mut top = Abs::zero();
    let mut bottom = Abs::zero();

    for g in text.glyphs.iter() {
        let (t, b) = text.font.edges(
            top_edge,
            bottom_edge,
            text.size,
            TextEdgeBounds::Glyph(g.id),
        );
        top.set_max(t);
        bottom.set_max(b);
    }

    (top, bottom)
}

/// Builds a kurbo [`BezPath`] for a glyph.
struct BezPathBuilder {
    path: BezPath,
    units_per_em: f64,
    font_size: Abs,
    x_offset: f64,
}

impl BezPathBuilder {
    fn new(units_per_em: f64, font_size: Abs, x_offset: f64) -> Self {
        Self {
            path: BezPath::new(),
            units_per_em,
            font_size,
            x_offset,
        }
    }

    fn finish(self) -> BezPath {
        self.path
    }

    fn p(&self, x: f32, y: f32) -> kurbo::Point {
        kurbo::Point::new(self.s(x) + self.x_offset, -self.s(y))
    }

    fn s(&self, v: f32) -> f64 {
        Em::from_units(v, self.units_per_em).at(self.font_size).to_raw()
    }
}

impl OutlineBuilder for BezPathBuilder {
    fn move_to(&mut self, x: f32, y: f32) {
        self.path.move_to(self.p(x, y));
    }

    fn line_to(&mut self, x: f32, y: f32) {
        self.path.line_to(self.p(x, y));
    }

    fn quad_to(&mut self, x1: f32, y1: f32, x: f32, y: f32) {
        self.path.quad_to(self.p(x1, y1), self.p(x, y));
    }

    fn curve_to(&mut self, x1: f32, y1: f32, x2: f32, y2: f32, x: f32, y: f32) {
        self.path.curve_to(self.p(x1, y1), self.p(x2, y2), self.p(x, y));
    }

    fn close(&mut self) {
        self.path.close_path();
    }
}

trait PathSegExt {
    fn translate(&self, offset: Vec2) -> Self;
    fn deriv_at(&self, t: f64) -> Vec2;
}

impl PathSegExt for PathSeg {
    fn translate(&self, offset: Vec2) -> Self {
        match self {
            PathSeg::Line(line) => PathSeg::Line(*line + offset),
            PathSeg::Quad(quad) => PathSeg::Quad(QuadBez::new(
                quad.p0 + offset,
                quad.p1 + offset,
                quad.p2 + offset,
            )),
            PathSeg::Cubic(cubic) => PathSeg::Cubic(CubicBez::new(
                cubic.p0 + offset,
                cubic.p1 + offset,
                cubic.p2 + offset,
                cubic.p3 + offset,
            )),
        }
    }

    fn deriv_at(&self, t: f64) -> Vec2 {
        match self {
            PathSeg::Line(line) => line.deriv().eval(t).to_vec2(),
            PathSeg::Quad(quad) => quad.deriv().eval(t).to_vec2(),
            PathSeg::Cubic(cubic) => cubic.deriv().eval(t).to_vec2(),
        }
    }
}
