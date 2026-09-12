//! OpenType glyph outlines.

use ttf_parser::{GlyphId, OutlineBuilder};

use crate::layout::{Abs, Point};
use crate::text::FontInstance;
use crate::text::color::should_outline;
use crate::visualize::Curve;

impl FontInstance {
    /// Returns a glyph's outline at the given font size.
    ///
    /// Coordinates are relative to the glyph origin and use OpenType's upward
    /// Y axis. Font variations are applied. Returns `None` if the glyph has no
    /// contour or is rendered through a bitmap, SVG, or color layer.
    #[comemo::memoize]
    pub fn outline_glyph(&self, glyph_id: u16, size: Abs) -> Option<Curve> {
        let glyph_id = GlyphId(glyph_id);
        should_outline(self, glyph_id).then_some(())?;
        let mut builder = CurveBuilder::new(size / self.units_per_em());
        self.ttf().outline_glyph(glyph_id, &mut builder)?;
        builder.drawn.then_some(builder.curve)
    }
}

/// Converts font units to absolute lengths and quadratic curves to cubics.
#[derive(Default)]
struct CurveBuilder {
    scale: Abs,
    curve: Curve,
    current: Point,
    start: Point,
    drawn: bool,
}

impl CurveBuilder {
    fn new(scale: Abs) -> Self {
        Self { scale, ..Self::default() }
    }

    fn point(&self, x: f32, y: f32) -> Point {
        Point::new(self.scale * f64::from(x), self.scale * f64::from(y))
    }
}

impl OutlineBuilder for CurveBuilder {
    fn move_to(&mut self, x: f32, y: f32) {
        self.current = self.point(x, y);
        self.start = self.current;
        self.curve.move_(self.current);
    }

    fn line_to(&mut self, x: f32, y: f32) {
        self.current = self.point(x, y);
        self.curve.line(self.current);
        self.drawn = true;
    }

    fn quad_to(&mut self, x1: f32, y1: f32, x: f32, y: f32) {
        let control = self.point(x1, y1);
        let end = self.point(x, y);
        self.curve.cubic(
            self.current + (control - self.current) * (2.0 / 3.0),
            end + (control - end) * (2.0 / 3.0),
            end,
        );
        self.current = end;
        self.drawn = true;
    }

    fn curve_to(&mut self, x1: f32, y1: f32, x2: f32, y2: f32, x: f32, y: f32) {
        self.current = self.point(x, y);
        self.curve.cubic(self.point(x1, y1), self.point(x2, y2), self.current);
        self.drawn = true;
    }

    fn close(&mut self) {
        self.curve.close();
        self.current = self.start;
    }
}

#[cfg(test)]
mod tests {
    use ecow::EcoString;
    use typst_syntax::Span;

    use super::*;
    use crate::foundations::Bytes;
    use crate::layout::Em;
    use crate::text::{Font, FontVariant, FontVariations, Glyph, Lang, TextItem};
    use crate::visualize::{Color, CurveItem, Paint};

    fn font() -> FontInstance {
        let data = typst_dev_assets::get("fonts/DejaVuSans.ttf").unwrap();
        Font::new(Bytes::new(data), 0).unwrap().instantiate(
            FontVariant::default(),
            Abs::pt(10.0),
            &FontVariations::default(),
        )
    }

    fn text_item(text: &str) -> TextItem {
        let font = font();
        let glyphs = text
            .char_indices()
            .map(|(start, c)| {
                let id = font.ttf().glyph_index(c).unwrap().0;
                Glyph {
                    id,
                    x_advance: font.x_advance(id).unwrap_or_default(),
                    x_offset: Em::default(),
                    y_advance: Em::default(),
                    y_offset: Em::default(),
                    range: start..start + c.len_utf8(),
                    span: (Span::detached(), 0),
                }
            })
            .collect();
        TextItem {
            font,
            size: Abs::pt(10.0),
            fill: Paint::Solid(Color::BLACK),
            stroke: None,
            lang: Lang::ENGLISH,
            region: None,
            text: EcoString::from(text),
            glyphs,
        }
    }

    #[test]
    fn elevates_quadratics_exactly() {
        let mut builder = CurveBuilder::new(Abs::pt(1.0));
        builder.move_to(0.0, 0.0);
        builder.quad_to(3.0, 3.0, 6.0, 0.0);

        assert_eq!(
            builder.curve.0,
            vec![
                CurveItem::Move(Point::zero()),
                CurveItem::Cubic(
                    Point::new(Abs::pt(2.0), Abs::pt(2.0)),
                    Point::new(Abs::pt(4.0), Abs::pt(2.0)),
                    Point::new(Abs::pt(6.0), Abs::zero()),
                ),
            ]
        );
    }

    #[test]
    fn close_restores_subpath_start() {
        let mut builder = CurveBuilder::new(Abs::pt(1.0));
        builder.move_to(2.0, 3.0);
        builder.line_to(5.0, 7.0);
        builder.close();
        builder.quad_to(5.0, 6.0, 8.0, 3.0);

        assert_eq!(
            builder.curve.0.last(),
            Some(&CurveItem::Cubic(
                Point::new(Abs::pt(4.0), Abs::pt(5.0)),
                Point::new(Abs::pt(6.0), Abs::pt(5.0)),
                Point::new(Abs::pt(8.0), Abs::pt(3.0)),
            ))
        );
    }

    #[test]
    fn ignores_contours_without_segments() {
        let mut builder = CurveBuilder::new(Abs::pt(1.0));
        builder.move_to(2.0, 3.0);
        builder.close();

        assert!(!builder.drawn);
    }

    #[test]
    fn distinguishes_contours_from_empty_glyphs() {
        let font = font();
        let glyph = |c| font.ttf().glyph_index(c).unwrap().0;

        assert!(font.outline_glyph(glyph('A'), Abs::pt(10.0)).is_some());
        assert!(font.outline_glyph(glyph(' '), Abs::pt(10.0)).is_none());
        assert!(
            font.outline_glyph(font.ttf().number_of_glyphs(), Abs::pt(10.0))
                .is_none()
        );
    }

    #[test]
    fn scales_outline_with_font_size() {
        let font = font();
        let glyph = font.ttf().glyph_index('A').unwrap().0;
        let small = font.outline_glyph(glyph, Abs::pt(10.0)).unwrap();
        let large = font.outline_glyph(glyph, Abs::pt(25.0)).unwrap();

        assert_eq!(large.bbox(None).size(), small.bbox(None).size() * 2.5);
    }

    #[test]
    fn text_outlines_preserve_positions_and_skip_empty_glyphs() {
        let text = text_item("A B");
        let positioned: Vec<_> = text.positioned_glyphs().collect();
        let outlined: Vec<_> = text.outlines().collect();

        assert_eq!(positioned.len(), 3);
        assert_eq!(outlined.len(), 2);
        assert_eq!(outlined[0].glyph.range, 0..1);
        assert_eq!(outlined[0].pos, Point::zero());
        assert_eq!(outlined[1].glyph.range, 2..3);
        assert_eq!(outlined[1].pos, positioned[2].0);
        let local = text.font.outline_glyph(outlined[1].glyph.id, text.size).unwrap();
        let (Some(CurveItem::Move(local)), Some(CurveItem::Move(positioned))) =
            (local.0.first(), outlined[1].curve.0.first())
        else {
            panic!("expected both outlines to start with a move");
        };
        assert_eq!(*positioned, *local + outlined[1].pos);
    }
}
