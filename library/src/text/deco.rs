use kurbo::{BezPath, Line, ParamCurve};
use ttf_parser::{GlyphId, OutlineBuilder};

use super::TextNode;
use crate::prelude::*;

/// # Underline
/// Underline text.
///
/// ## Example
/// ```
/// This is #underline[important].
/// ```
///
/// ## Parameters
/// - body: Content (positional, required)
///   The content to underline.
///
/// ## Category
/// text
#[func]
#[capable(Show)]
#[derive(Debug, Hash)]
pub struct UnderlineNode(pub Content);

#[node]
impl UnderlineNode {
    /// How to stroke the line. The text color and thickness are read from the
    /// font tables if `{auto}`.
    ///
    /// # Example
    /// ```
    /// Take #underline(
    ///   stroke: 1.5pt + red,
    ///   offset: 2pt,
    ///   [care],
    /// )
    /// ```
    #[property(shorthand, resolve, fold)]
    pub const STROKE: Smart<PartialStroke> = Smart::Auto;

    /// Position of the line relative to the baseline, read from the font tables
    /// if `{auto}`.
    ///
    /// # Example
    /// ```
    /// #underline(offset: 5pt)[
    ///   The Tale Of A Faraway Line I
    /// ]
    /// ```
    #[property(resolve)]
    pub const OFFSET: Smart<Length> = Smart::Auto;

    /// Amount that the line will be longer or shorter than its associated text.
    ///
    /// # Example
    /// ```
    /// #align(center,
    ///   underline(extent: 2pt)[Chapter 1]
    /// )
    /// ```
    #[property(resolve)]
    pub const EXTENT: Length = Length::zero();

    /// Whether the line skips sections in which it would collide with the
    /// glyphs.
    ///
    /// # Example
    /// ```
    /// This #underline(evade: true)[is great].
    /// This #underline(evade: false)[is less great].
    /// ```
    pub const EVADE: bool = true;

    fn construct(_: &Vm, args: &mut Args) -> SourceResult<Content> {
        Ok(Self(args.expect("body")?).pack())
    }

    fn field(&self, name: &str) -> Option<Value> {
        match name {
            "body" => Some(Value::Content(self.0.clone())),
            _ => None,
        }
    }
}

impl Show for UnderlineNode {
    fn show(&self, _: &mut Vt, _: &Content, styles: StyleChain) -> SourceResult<Content> {
        Ok(self.0.clone().styled(
            TextNode::DECO,
            Decoration {
                line: DecoLine::Underline,
                stroke: styles.get(Self::STROKE).unwrap_or_default(),
                offset: styles.get(Self::OFFSET),
                extent: styles.get(Self::EXTENT),
                evade: styles.get(Self::EVADE),
            },
        ))
    }
}

/// # Overline
/// Add a line over text.
///
/// ## Example
/// ```
/// #overline[A line over text.]
/// ```
///
/// ## Parameters
/// - body: Content (positional, required)
///   The content to add a line over.
///
/// ## Category
/// text
#[func]
#[capable(Show)]
#[derive(Debug, Hash)]
pub struct OverlineNode(pub Content);

#[node]
impl OverlineNode {
    /// How to stroke the line. The text color and thickness are read from the
    /// font tables if `{auto}`.
    ///
    /// # Example
    /// ```
    /// #set text(fill: olive)
    /// #overline(
    ///   stroke: green.darken(20%),
    ///   offset: -12pt,
    ///   [The Forest Theme],
    /// )
    /// ```
    #[property(shorthand, resolve, fold)]
    pub const STROKE: Smart<PartialStroke> = Smart::Auto;

    /// Position of the line relative to the baseline, read from the font tables
    /// if `{auto}`.
    ///
    /// # Example
    /// ```
    /// #overline(offset: -1.2em)[
    ///   The Tale Of A Faraway Line II
    /// ]
    /// ```
    #[property(resolve)]
    pub const OFFSET: Smart<Length> = Smart::Auto;

    /// Amount that the line will be longer or shorter than its associated text.
    ///
    /// # Example
    /// ```
    /// #set overline(extent: 4pt)
    /// #set underline(extent: 4pt)
    /// #overline(underline[Typography Today])
    /// ```
    #[property(resolve)]
    pub const EXTENT: Length = Length::zero();

    /// Whether the line skips sections in which it would collide with the
    /// glyphs.
    ///
    /// # Example
    /// ```
    /// #overline(
    ///   evade: false,
    ///   offset: -7.5pt,
    ///   stroke: 1pt,
    ///   extent: 3pt,
    ///   [Temple],
    /// )
    /// ```
    pub const EVADE: bool = true;

    fn construct(_: &Vm, args: &mut Args) -> SourceResult<Content> {
        Ok(Self(args.expect("body")?).pack())
    }

    fn field(&self, name: &str) -> Option<Value> {
        match name {
            "body" => Some(Value::Content(self.0.clone())),
            _ => None,
        }
    }
}

impl Show for OverlineNode {
    fn show(&self, _: &mut Vt, _: &Content, styles: StyleChain) -> SourceResult<Content> {
        Ok(self.0.clone().styled(
            TextNode::DECO,
            Decoration {
                line: DecoLine::Overline,
                stroke: styles.get(Self::STROKE).unwrap_or_default(),
                offset: styles.get(Self::OFFSET),
                extent: styles.get(Self::EXTENT),
                evade: styles.get(Self::EVADE),
            },
        ))
    }
}

/// # Strikethrough
/// Strike through text.
///
/// ## Example
/// ```
/// This is #strike[not] relevant.
/// ```
///
/// ## Parameters
/// - body: Content (positional, required)
///   The content to strike through.
///
/// ## Category
/// text
#[func]
#[capable(Show)]
#[derive(Debug, Hash)]
pub struct StrikeNode(pub Content);

#[node]
impl StrikeNode {
    /// How to stroke the line. The text color and thickness are read from the
    /// font tables if `{auto}`.
    ///
    /// # Example
    /// ```
    /// This is #strike(stroke: 1.5pt + red)[very stricken through]. \
    /// This is #strike(stroke: 10pt)[redacted].
    /// ```
    #[property(shorthand, resolve, fold)]
    pub const STROKE: Smart<PartialStroke> = Smart::Auto;

    /// Position of the line relative to the baseline, read from the font tables
    /// if `{auto}`.
    ///
    /// This is useful if you are unhappy with the offset your font provides.
    ///
    /// # Example
    /// ```
    /// #set text(family: "Inria Serif")
    /// This is #strike(offset: auto)[low-ish]. \
    /// This is #strike(offset: -3.5pt)[on-top].
    /// ```
    #[property(resolve)]
    pub const OFFSET: Smart<Length> = Smart::Auto;

    /// Amount that the line will be longer or shorter than its associated text.
    ///
    /// # Example
    /// ```
    /// This #strike(extent: -2pt)[skips] parts of the word.
    /// This #strike(extent: 2pt)[extends] beyond the word.
    /// ```
    #[property(resolve)]
    pub const EXTENT: Length = Length::zero();

    fn construct(_: &Vm, args: &mut Args) -> SourceResult<Content> {
        Ok(Self(args.expect("body")?).pack())
    }

    fn field(&self, name: &str) -> Option<Value> {
        match name {
            "body" => Some(Value::Content(self.0.clone())),
            _ => None,
        }
    }
}

impl Show for StrikeNode {
    fn show(&self, _: &mut Vt, _: &Content, styles: StyleChain) -> SourceResult<Content> {
        Ok(self.0.clone().styled(
            TextNode::DECO,
            Decoration {
                line: DecoLine::Strikethrough,
                stroke: styles.get(Self::STROKE).unwrap_or_default(),
                offset: styles.get(Self::OFFSET),
                extent: styles.get(Self::EXTENT),
                evade: false,
            },
        ))
    }
}

/// Defines a line that is positioned over, under or on top of text.
///
/// For more details, see [`DecoNode`].
#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub struct Decoration {
    pub line: DecoLine,
    pub stroke: PartialStroke<Abs>,
    pub offset: Smart<Abs>,
    pub extent: Abs,
    pub evade: bool,
}

impl Fold for Decoration {
    type Output = Vec<Self>;

    fn fold(self, mut outer: Self::Output) -> Self::Output {
        outer.insert(0, self);
        outer
    }
}

/// A kind of decorative line.
#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
pub enum DecoLine {
    Underline,
    Strikethrough,
    Overline,
}

/// Add line decorations to a single run of shaped text.
pub(super) fn decorate(
    frame: &mut Frame,
    deco: &Decoration,
    text: &Text,
    shift: Abs,
    pos: Point,
    width: Abs,
) {
    let font_metrics = text.font.metrics();
    let metrics = match deco.line {
        DecoLine::Strikethrough => font_metrics.strikethrough,
        DecoLine::Overline => font_metrics.overline,
        DecoLine::Underline => font_metrics.underline,
    };

    let offset = deco.offset.unwrap_or(-metrics.position.at(text.size)) - shift;
    let stroke = deco.stroke.unwrap_or(Stroke {
        paint: text.fill,
        thickness: metrics.thickness.at(text.size),
    });

    let gap_padding = 0.08 * text.size;
    let min_width = 0.162 * text.size;

    let mut start = pos.x - deco.extent;
    let end = pos.x + (width + 2.0 * deco.extent);

    let mut push_segment = |from: Abs, to: Abs| {
        let origin = Point::new(from, pos.y + offset);
        let target = Point::new(to - from, Abs::zero());

        if target.x >= min_width || !deco.evade {
            let shape = Geometry::Line(target).stroked(stroke);
            frame.push(origin, Element::Shape(shape));
        }
    };

    if !deco.evade {
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
            BezPathBuilder::new(font_metrics.units_per_em, text.size, dx.to_raw());

        let bbox = text.font.ttf().outline_glyph(GlyphId(glyph.id), &mut builder);
        let path = builder.finish();

        x += glyph.x_advance.at(text.size);

        // Only do the costly segments intersection test if the line
        // intersects the bounding box.
        if bbox.map_or(false, |bbox| {
            let y_min = -text.font.to_em(bbox.y_max).at(text.size);
            let y_max = -text.font.to_em(bbox.y_min).at(text.size);
            offset >= y_min && offset <= y_max
        }) {
            // Find all intersections of segments with the line.
            intersections.extend(
                path.segments()
                    .flat_map(|seg| seg.intersect_line(line))
                    .map(|is| Abs::raw(line.eval(is.line_t).x)),
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
