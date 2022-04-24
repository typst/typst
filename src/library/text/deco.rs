use kurbo::{BezPath, Line, ParamCurve};
use ttf_parser::{GlyphId, OutlineBuilder};

use super::TextNode;
use crate::font::FontStore;
use crate::library::prelude::*;

/// Typeset underline, stricken-through or overlined text.
#[derive(Debug, Hash)]
pub struct DecoNode<const L: DecoLine>(pub Content);

/// Typeset underlined text.
pub type UnderlineNode = DecoNode<UNDERLINE>;

/// Typeset stricken-through text.
pub type StrikethroughNode = DecoNode<STRIKETHROUGH>;

/// Typeset overlined text.
pub type OverlineNode = DecoNode<OVERLINE>;

#[node(showable)]
impl<const L: DecoLine> DecoNode<L> {
    /// How to stroke the line. The text color and thickness read from the font
    /// tables if `auto`.
    #[property(shorthand, resolve, fold)]
    pub const STROKE: Smart<RawStroke> = Smart::Auto;

    /// Position of the line relative to the baseline, read from the font tables
    /// if `auto`.
    #[property(resolve)]
    pub const OFFSET: Smart<RawLength> = Smart::Auto;
    /// Amount that the line will be longer or shorter than its associated text.
    #[property(resolve)]
    pub const EXTENT: RawLength = RawLength::zero();

    /// Whether the line skips sections in which it would collide
    /// with the glyphs. Does not apply to strikethrough.
    pub const EVADE: bool = true;

    fn construct(_: &mut Context, args: &mut Args) -> TypResult<Content> {
        Ok(Content::show(Self(args.expect::<Content>("body")?)))
    }
}

impl<const L: DecoLine> Show for DecoNode<L> {
    fn encode(&self) -> Dict {
        dict! { "body" => Value::Content(self.0.clone()) }
    }

    fn realize(&self, _: &mut Context, styles: StyleChain) -> TypResult<Content> {
        Ok(self.0.clone().styled(TextNode::DECO, Decoration {
            line: L,
            stroke: styles.get(Self::STROKE).unwrap_or_default(),
            offset: styles.get(Self::OFFSET),
            extent: styles.get(Self::EXTENT),
            evade: styles.get(Self::EVADE),
        }))
    }
}

/// Defines a line that is positioned over, under or on top of text.
///
/// For more details, see [`DecoNode`].
#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub struct Decoration {
    pub line: DecoLine,
    pub stroke: RawStroke<Length>,
    pub offset: Smart<Length>,
    pub extent: Length,
    pub evade: bool,
}

/// A kind of decorative line.
pub type DecoLine = usize;

/// A line under text.
pub const UNDERLINE: DecoLine = 0;

/// A line through text.
pub const STRIKETHROUGH: DecoLine = 1;

/// A line over text.
pub const OVERLINE: DecoLine = 2;

/// Add line decorations to a single run of shaped text.
pub fn decorate(
    frame: &mut Frame,
    deco: &Decoration,
    fonts: &FontStore,
    text: &Text,
    pos: Point,
    width: Length,
) {
    let face = fonts.get(text.face_id);
    let face_metrics = face.metrics();
    let metrics = match deco.line {
        STRIKETHROUGH => face_metrics.strikethrough,
        OVERLINE => face_metrics.overline,
        UNDERLINE | _ => face_metrics.underline,
    };

    let evade = deco.evade && deco.line != STRIKETHROUGH;
    let offset = deco.offset.unwrap_or(-metrics.position.at(text.size));
    let stroke = deco.stroke.unwrap_or(Stroke {
        paint: text.fill,
        thickness: metrics.thickness.at(text.size),
    });

    let gap_padding = 0.08 * text.size;
    let min_width = 0.162 * text.size;

    let mut start = pos.x - deco.extent;
    let end = pos.x + (width + 2.0 * deco.extent);

    let mut push_segment = |from: Length, to: Length| {
        let origin = Point::new(from, pos.y + offset);
        let target = Point::new(to - from, Length::zero());

        if target.x >= min_width || !evade {
            let shape = Geometry::Line(target).stroked(stroke);
            frame.push(origin, Element::Shape(shape));
        }
    };

    if !evade {
        push_segment(start, end);
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
            BezPathBuilder::new(face_metrics.units_per_em, text.size, dx.to_raw());

        let bbox = face.ttf().outline_glyph(GlyphId(glyph.id), &mut builder);
        let path = builder.finish();

        x += glyph.x_advance.at(text.size);

        // Only do the costly segments intersection test if the line
        // intersects the bounding box.
        if bbox.map_or(false, |bbox| {
            let y_min = -face.to_em(bbox.y_max).at(text.size);
            let y_max = -face.to_em(bbox.y_min).at(text.size);

            offset >= y_min && offset <= y_max
        }) {
            // Find all intersections of segments with the line.
            intersections.extend(
                path.segments()
                    .flat_map(|seg| seg.intersect_line(line))
                    .map(|is| Length::raw(line.eval(is.line_t).x)),
            );
        }
    }

    // When emitting the decorative line segments, we move from left to
    // right. The intersections are not necessarily in this order, yet.
    intersections.sort();

    for gap in intersections.chunks_exact(2) {
        let l = gap[0] - gap_padding;
        let r = gap[1] + gap_padding;

        if start >= end {
            break;
        }

        if start >= l {
            start = r;
            continue;
        }

        push_segment(start, l);
        start = r;
    }

    if start < end {
        push_segment(start, end);
    }
}

/// Builds a kurbo [`BezPath`] for a glyph.
struct BezPathBuilder {
    path: BezPath,
    units_per_em: f64,
    font_size: Length,
    x_offset: f64,
}

impl BezPathBuilder {
    fn new(units_per_em: f64, font_size: Length, x_offset: f64) -> Self {
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
