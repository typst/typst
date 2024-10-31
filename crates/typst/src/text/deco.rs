use kurbo::{BezPath, Line, ParamCurve};
use smallvec::smallvec;
use ttf_parser::{GlyphId, OutlineBuilder};

use crate::diag::SourceResult;
use crate::engine::Engine;
use crate::foundations::{elem, Content, Packed, Show, Smart, StyleChain};
use crate::layout::{
    Abs, Corners, Em, Frame, FrameItem, Length, Point, Rel, Sides, Size,
};
use crate::syntax::Span;
use crate::text::{
    BottomEdge, BottomEdgeMetric, TextEdgeBounds, TextElem, TextItem, TopEdge,
    TopEdgeMetric,
};
use crate::visualize::{styled_rect, Color, FixedStroke, Geometry, Paint, Stroke};

/// Underlines text.
///
/// # Example
/// ```example
/// This is #underline[important].
/// ```
#[elem(Show)]
pub struct UnderlineElem {
    /// How to [stroke] the line.
    ///
    /// If set to `{auto}`, takes on the text's color and a thickness defined in
    /// the current font.
    ///
    /// ```example
    /// Take #underline(
    ///   stroke: 1.5pt + red,
    ///   offset: 2pt,
    ///   [care],
    /// )
    /// ```
    #[resolve]
    #[fold]
    pub stroke: Smart<Stroke>,

    /// The position of the line relative to the baseline, read from the font
    /// tables if `{auto}`.
    ///
    /// ```example
    /// #underline(offset: 5pt)[
    ///   The Tale Of A Faraway Line I
    /// ]
    /// ```
    #[resolve]
    pub offset: Smart<Length>,

    /// The amount by which to extend the line beyond (or within if negative)
    /// the content.
    ///
    /// ```example
    /// #align(center,
    ///   underline(extent: 2pt)[Chapter 1]
    /// )
    /// ```
    #[resolve]
    pub extent: Length,

    /// Whether the line skips sections in which it would collide with the
    /// glyphs.
    ///
    /// ```example
    /// This #underline(evade: true)[is great].
    /// This #underline(evade: false)[is less great].
    /// ```
    #[default(true)]
    pub evade: bool,

    /// Whether the line is placed behind the content it underlines.
    ///
    /// ```example
    /// #set underline(stroke: (thickness: 1em, paint: maroon, cap: "round"))
    /// #underline(background: true)[This is stylized.] \
    /// #underline(background: false)[This is partially hidden.]
    /// ```
    #[default(false)]
    pub background: bool,

    /// The content to underline.
    #[required]
    pub body: Content,
}

impl Show for Packed<UnderlineElem> {
    #[typst_macros::time(name = "underline", span = self.span())]
    fn show(&self, _: &mut Engine, styles: StyleChain) -> SourceResult<Content> {
        Ok(self.body().clone().styled(TextElem::set_deco(smallvec![Decoration {
            line: DecoLine::Underline {
                stroke: self.stroke(styles).unwrap_or_default(),
                offset: self.offset(styles),
                evade: self.evade(styles),
                background: self.background(styles),
            },
            extent: self.extent(styles),
        }])))
    }
}

/// Adds a line over text.
///
/// # Example
/// ```example
/// #overline[A line over text.]
/// ```
#[elem(Show)]
pub struct OverlineElem {
    /// How to [stroke] the line.
    ///
    /// If set to `{auto}`, takes on the text's color and a thickness defined in
    /// the current font.
    ///
    /// ```example
    /// #set text(fill: olive)
    /// #overline(
    ///   stroke: green.darken(20%),
    ///   offset: -12pt,
    ///   [The Forest Theme],
    /// )
    /// ```
    #[resolve]
    #[fold]
    pub stroke: Smart<Stroke>,

    /// The position of the line relative to the baseline. Read from the font
    /// tables if `{auto}`.
    ///
    /// ```example
    /// #overline(offset: -1.2em)[
    ///   The Tale Of A Faraway Line II
    /// ]
    /// ```
    #[resolve]
    pub offset: Smart<Length>,

    /// The amount by which to extend the line beyond (or within if negative)
    /// the content.
    ///
    /// ```example
    /// #set overline(extent: 4pt)
    /// #set underline(extent: 4pt)
    /// #overline(underline[Typography Today])
    /// ```
    #[resolve]
    pub extent: Length,

    /// Whether the line skips sections in which it would collide with the
    /// glyphs.
    ///
    /// ```example
    /// #overline(
    ///   evade: false,
    ///   offset: -7.5pt,
    ///   stroke: 1pt,
    ///   extent: 3pt,
    ///   [Temple],
    /// )
    /// ```
    #[default(true)]
    pub evade: bool,

    /// Whether the line is placed behind the content it overlines.
    ///
    /// ```example
    /// #set overline(stroke: (thickness: 1em, paint: maroon, cap: "round"))
    /// #overline(background: true)[This is stylized.] \
    /// #overline(background: false)[This is partially hidden.]
    /// ```
    #[default(false)]
    pub background: bool,

    /// The content to add a line over.
    #[required]
    pub body: Content,
}

impl Show for Packed<OverlineElem> {
    #[typst_macros::time(name = "overline", span = self.span())]
    fn show(&self, _: &mut Engine, styles: StyleChain) -> SourceResult<Content> {
        Ok(self.body().clone().styled(TextElem::set_deco(smallvec![Decoration {
            line: DecoLine::Overline {
                stroke: self.stroke(styles).unwrap_or_default(),
                offset: self.offset(styles),
                evade: self.evade(styles),
                background: self.background(styles),
            },
            extent: self.extent(styles),
        }])))
    }
}

/// Strikes through text.
///
/// # Example
/// ```example
/// This is #strike[not] relevant.
/// ```
#[elem(title = "Strikethrough", Show)]
pub struct StrikeElem {
    /// How to [stroke] the line.
    ///
    /// If set to `{auto}`, takes on the text's color and a thickness defined in
    /// the current font.
    ///
    /// _Note:_ Please don't use this for real redaction as you can still copy
    /// paste the text.
    ///
    /// ```example
    /// This is #strike(stroke: 1.5pt + red)[very stricken through]. \
    /// This is #strike(stroke: 10pt)[redacted].
    /// ```
    #[resolve]
    #[fold]
    pub stroke: Smart<Stroke>,

    /// The position of the line relative to the baseline. Read from the font
    /// tables if `{auto}`.
    ///
    /// This is useful if you are unhappy with the offset your font provides.
    ///
    /// ```example
    /// #set text(font: "Inria Serif")
    /// This is #strike(offset: auto)[low-ish]. \
    /// This is #strike(offset: -3.5pt)[on-top].
    /// ```
    #[resolve]
    pub offset: Smart<Length>,

    /// The amount by which to extend the line beyond (or within if negative)
    /// the content.
    ///
    /// ```example
    /// This #strike(extent: -2pt)[skips] parts of the word.
    /// This #strike(extent: 2pt)[extends] beyond the word.
    /// ```
    #[resolve]
    pub extent: Length,

    /// Whether the line is placed behind the content.
    ///
    /// ```example
    /// #set strike(stroke: red)
    /// #strike(background: true)[This is behind.] \
    /// #strike(background: false)[This is in front.]
    /// ```
    #[default(false)]
    pub background: bool,

    /// The content to strike through.
    #[required]
    pub body: Content,
}

impl Show for Packed<StrikeElem> {
    #[typst_macros::time(name = "strike", span = self.span())]
    fn show(&self, _: &mut Engine, styles: StyleChain) -> SourceResult<Content> {
        Ok(self.body().clone().styled(TextElem::set_deco(smallvec![Decoration {
            // Note that we do not support evade option for strikethrough.
            line: DecoLine::Strikethrough {
                stroke: self.stroke(styles).unwrap_or_default(),
                offset: self.offset(styles),
                background: self.background(styles),
            },
            extent: self.extent(styles),
        }])))
    }
}

/// Highlights text with a background color.
///
/// # Example
/// ```example
/// This is #highlight[important].
/// ```
#[elem(Show)]
pub struct HighlightElem {
    /// The color to highlight the text with.
    ///
    /// ```example
    /// This is #highlight(
    ///   fill: blue
    /// )[highlighted with blue].
    /// ```
    #[default(Some(Color::from_u8(0xFF, 0xFD, 0x11, 0xA1).into()))]
    pub fill: Option<Paint>,

    /// The highlight's border color. See the
    /// [rectangle's documentation]($rect.stroke) for more details.
    ///
    /// ```example
    /// This is a #highlight(
    ///   stroke: fuchsia
    /// )[stroked highlighting].
    /// ```
    #[resolve]
    #[fold]
    pub stroke: Sides<Option<Option<Stroke>>>,

    /// The top end of the background rectangle.
    ///
    /// ```example
    /// #set highlight(top-edge: "ascender")
    /// #highlight[a] #highlight[aib]
    ///
    /// #set highlight(top-edge: "x-height")
    /// #highlight[a] #highlight[aib]
    /// ```
    #[default(TopEdge::Metric(TopEdgeMetric::Ascender))]
    pub top_edge: TopEdge,

    /// The bottom end of the background rectangle.
    ///
    /// ```example
    /// #set highlight(bottom-edge: "descender")
    /// #highlight[a] #highlight[ap]
    ///
    /// #set highlight(bottom-edge: "baseline")
    /// #highlight[a] #highlight[ap]
    /// ```
    #[default(BottomEdge::Metric(BottomEdgeMetric::Descender))]
    pub bottom_edge: BottomEdge,

    /// The amount by which to extend the background to the sides beyond
    /// (or within if negative) the content.
    ///
    /// ```example
    /// A long #highlight(extent: 4pt)[background].
    /// ```
    #[resolve]
    pub extent: Length,

    /// How much to round the highlight's corners. See the
    /// [rectangle's documentation]($rect.radius) for more details.
    ///
    /// ```example
    /// Listen #highlight(
    ///   radius: 5pt, extent: 2pt
    /// )[carefully], it will be on the test.
    /// ```
    #[resolve]
    #[fold]
    pub radius: Corners<Option<Rel<Length>>>,

    /// The content that should be highlighted.
    #[required]
    pub body: Content,
}

impl Show for Packed<HighlightElem> {
    #[typst_macros::time(name = "highlight", span = self.span())]
    fn show(&self, _: &mut Engine, styles: StyleChain) -> SourceResult<Content> {
        Ok(self.body().clone().styled(TextElem::set_deco(smallvec![Decoration {
            line: DecoLine::Highlight {
                fill: self.fill(styles),
                stroke: self
                    .stroke(styles)
                    .unwrap_or_default()
                    .map(|stroke| stroke.map(Stroke::unwrap_or_default)),
                top_edge: self.top_edge(styles),
                bottom_edge: self.bottom_edge(styles),
                radius: self.radius(styles).unwrap_or_default(),
            },
            extent: self.extent(styles),
        }])))
    }
}

/// A text decoration.
///
/// Can be positioned over, under, or on top of text, or highlight the text with
/// a background.
#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub struct Decoration {
    line: DecoLine,
    extent: Abs,
}

/// A kind of decorative line.
#[derive(Debug, Clone, Eq, PartialEq, Hash)]
enum DecoLine {
    Underline {
        stroke: Stroke<Abs>,
        offset: Smart<Abs>,
        evade: bool,
        background: bool,
    },
    Strikethrough {
        stroke: Stroke<Abs>,
        offset: Smart<Abs>,
        background: bool,
    },
    Overline {
        stroke: Stroke<Abs>,
        offset: Smart<Abs>,
        evade: bool,
        background: bool,
    },
    Highlight {
        fill: Option<Paint>,
        stroke: Sides<Option<FixedStroke>>,
        top_edge: TopEdge,
        bottom_edge: BottomEdge,
        radius: Corners<Rel<Abs>>,
    },
}

/// Add line decorations to a single run of shaped text.
pub(crate) fn decorate(
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
