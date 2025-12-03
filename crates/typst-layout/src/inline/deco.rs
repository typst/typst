use kurbo::{BezPath, Line, ParamCurve};
use ttf_parser::{GlyphId, OutlineBuilder};
use typst_library::layout::{Abs, Em, Frame, FrameItem, Point, Size};
use typst_library::text::{
    BottomEdge, DecoLine, Decoration, Glyph, TextEdgeBounds, TextItem, TopEdge,
    is_default_ignorable,
};
use typst_library::visualize::{FixedStroke, Geometry};
use typst_syntax::Span;

use super::shaping::is_of_cj_script;
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
    let (trim_start, trim_end) = adjust_cjk_latin_spacing(width, text);
    let line_start = pos.x + trim_start;
    let line_end = pos.x + width - trim_end;

    if line_end <= line_start {
        return;
    }

    if let DecoLine::Highlight { fill, stroke, top_edge, bottom_edge, radius } =
        &deco.line
    {
        let trimmed_width = line_end - line_start;
        let (top, bottom) = determine_edges(text, *top_edge, *bottom_edge);
        let size = Size::new(trimmed_width + 2.0 * deco.extent, top + bottom);
        let rects = styled_rect(size, radius, fill.clone(), stroke);
        let origin = Point::new(line_start - deco.extent, pos.y - top - shift);
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

    let start = line_start - deco.extent;
    let end = line_end + deco.extent;

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
        kurbo::Point::new(line_start.to_raw(), offset.to_raw()),
        kurbo::Point::new(line_end.to_raw(), offset.to_raw()),
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
            // Find all intersections of segments with the line.
            intersections.extend(
                path.segments()
                    .flat_map(|seg| seg.intersect_line(line))
                    .map(|is| Abs::raw(line.eval(is.line_t).x)),
            );
        }
    }

    // Add start and end points, taking padding into account.
    intersections.push(start - gap_padding);
    intersections.push(end + gap_padding);
    // When emitting the decorative line segments, we move from left to
    // right. The intersections are not necessarily in this order, yet.
    intersections.sort();

    for edge in intersections.windows(2) {
        let l = edge[0];
        let r = edge[1];

        // If we are too close, don't draw the segment
        if r - l < gap_padding {
            continue;
        } else {
            push_segment(l + gap_padding, r - gap_padding, background);
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

/// Clamp the CJK-Latin padding for the given run width to avoid over-shrinking
/// the decoration geometry.
fn adjust_cjk_latin_spacing(width: Abs, text: &TextItem) -> (Abs, Abs) {
    // `add_cjk_latin_spacing` currently inserts 1/4 em padding.
    let spacing = Em::new(0.25).at(text.size);
    if spacing.approx_empty() {
        return (Abs::zero(), Abs::zero());
    }

    let tolerance = spacing / 5.0;
    let mut leading = Abs::zero();
    let mut trailing = Abs::zero();

    if let Some(first) = text.glyphs.first() {
        if glyph_primary_char(text, first).is_some_and(is_of_cj_script) {
            let offset = first.x_offset.at(text.size);
            if offset + tolerance >= spacing {
                leading = spacing.min(offset);
            }
        }
    }

    if let Some(last) = text.glyphs.last() {
        if glyph_primary_char(text, last).is_some_and(is_of_cj_script) {
            let base = text.font.x_advance(last.id).unwrap_or(last.x_advance);
            let extra_em = last.x_advance - base;
            if extra_em > Em::zero() {
                let extra = extra_em.at(text.size);
                if extra + tolerance >= spacing {
                    trailing = spacing.min(extra);
                }
            }
        }
    }

    if leading > width {
        leading = width;
    }

    let remaining = width - leading;
    if trailing > remaining {
        trailing = remaining;
    }

    (leading, trailing)
}

/// Extract the primary (non-ignorable) character represented by a glyph.
fn glyph_primary_char(text: &TextItem, glyph: &Glyph) -> Option<char> {
    let slice: &str = &text.text[glyph.range()];
    slice.chars().find(|&c| !is_default_ignorable(c))
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
