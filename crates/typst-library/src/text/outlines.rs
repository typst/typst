use comemo::Tracked;
use typst_syntax::Span;

use crate::diag::SourceResult;
use crate::engine::Engine;
use crate::foundations::{
    Array, Content, Context, Dict, IntoValue, NativeElement, Smart, array, dict, func,
    scope,
};
use crate::layout::{
    Axes, Frame, FrameItem, Length, Point, Ratio, Rel, Transform, measure_frame,
};
use crate::text::{TextElem, TextItemView};
use crate::visualize::{
    CloseMode, Curve, CurveClose, CurveCubic, CurveItem, CurveLine, CurveMove,
};

#[scope]
impl TextElem {
    /// Extracts the outlines of the glyphs in content.
    ///
    /// Content is laid out just like @measure, including font fallback,
    /// shaping, and line breaking. The result contains `width`, `height`, and
    /// `baseline` lengths, plus a `glyphs` array in layout order. This order can
    /// differ from visual order for bidirectional text.
    ///
    /// Each glyph has these fields:
    ///
    /// - `text`: The text cluster that produced the glyph. Clusters and glyphs
    ///   do not map one-to-one. Default-ignorable characters at the edges are
    ///   omitted.
    /// - `range`: The cluster's UTF-8 byte range within the shaped text run.
    /// - `position`: The glyph origin, with `x` and `y` lengths.
    /// - `advance`: The glyph advance, with `x` and `y` lengths.
    /// - `components`: An array of @curve.move, @curve.line, @curve.cubic, and
    ///   @curve.close elements, or `{none}` if the glyph has no outline.
    ///
    /// Positions and component coordinates are measured from the top-left corner
    /// of the laid-out content and use a downward Y axis. They include glyph
    /// offsets and transformations of enclosing content. Components can be
    /// inspected, modified, or passed directly to @curve. Convert their lengths
    /// with @length.pt before encoding them for a @plugin.
    ///
    /// The result contains font contours only. Space glyphs and glyphs rendered
    /// through color, SVG, or bitmap data have `{none}` components. Non-text
    /// items, decorations, clipping, fills, and strokes are omitted. Converting
    /// text to curves removes its text semantics.
    ///
    /// ```example
    /// #context {
    ///   let paths = text.outlines[Hi!]
    ///   curve(
    ///     fill: blue,
    ///     ..paths.glyphs
    ///       .filter(g => g.components != none)
    ///       .map(g => g.components).flatten(),
    ///   )
    /// }
    /// ```
    #[func(contextual, since = "0.16.0")]
    pub fn outlines(
        engine: &mut Engine,
        context: Tracked<Context>,
        span: Span,
        /// The width available for layout. As with @measure, `{auto}` means
        /// infinite available width.
        #[named]
        #[default(Smart::Auto)]
        width: Smart<Length>,
        /// The height available for layout. As with @measure, `{auto}` means
        /// infinite available height.
        #[named]
        #[default(Smart::Auto)]
        height: Smart<Length>,
        /// The content whose glyph outlines to extract.
        content: Content,
    ) -> SourceResult<Dict> {
        let frame = measure_frame(engine, context, span, width, height, &content)?;
        let mut glyphs = Array::new();
        collect_glyphs(&frame, Transform::identity(), &mut glyphs);
        Ok(dict! {
            "width" => frame.width(),
            "height" => frame.height(),
            "baseline" => frame.baseline(),
            "glyphs" => glyphs,
        })
    }
}

/// Appends glyph records in root-frame coordinates.
fn collect_glyphs(frame: &Frame, transform: Transform, glyphs: &mut Array) {
    for (pos, item) in frame.items() {
        let transform = transform.pre_concat(Transform::translate(pos.x, pos.y));
        match item {
            FrameItem::Group(group) => collect_glyphs(
                &group.frame,
                transform.pre_concat(group.transform),
                glyphs,
            ),
            FrameItem::Text(text) => {
                let view = TextItemView::full(text);
                // Convert the glyph run's upward Y axis to the frame's downward axis.
                let transform =
                    transform.pre_concat(Transform::scale(Ratio::one(), -Ratio::one()));
                for (glyph_pos, glyph) in text.positioned_glyphs() {
                    let glyph_transform = transform
                        .pre_concat(Transform::translate(glyph_pos.x, glyph_pos.y));
                    let pos = Point::zero().transform(glyph_transform);
                    let end = glyph.advance_at(text.size).transform(glyph_transform);
                    let components = text
                        .font
                        .outline_glyph(glyph.id, text.size)
                        .map(|curve| components(curve, glyph_transform));
                    let record = dict! {
                        "text" => view.glyph_text(glyph),
                        "range" => array![glyph.range.start, glyph.range.end],
                        "position" => coordinates(pos),
                        "advance" => coordinates(end - pos),
                        "components" => components,
                    };
                    glyphs.push(record.into_value());
                }
            }
            _ => {}
        }
    }
}

fn coordinates(point: Point) -> Dict {
    dict! {
        "x" => Length::from(point.x),
        "y" => Length::from(point.y),
    }
}

/// Converts contours to editable curve components, preserving straight closes.
fn components(curve: Curve, transform: Transform) -> Array {
    let point = |p: Point| {
        let p = p.transform(transform);
        Axes::new(Rel::from(Length::from(p.x)), Rel::from(Length::from(p.y)))
    };
    curve
        .0
        .into_iter()
        .map(|item| {
            match item {
                CurveItem::Move(p) => CurveMove::new(point(p)).pack(),
                CurveItem::Line(p) => CurveLine::new(point(p)).pack(),
                CurveItem::Cubic(a, b, c) => CurveCubic::new(
                    Some(Smart::Custom(point(a))),
                    Some(point(b)),
                    point(c),
                )
                .pack(),
                CurveItem::Close => {
                    CurveClose::new().with_mode(CloseMode::Straight).pack()
                }
            }
            .into_value()
        })
        .collect()
}
