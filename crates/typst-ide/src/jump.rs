use std::num::NonZeroUsize;

use ecow::EcoString;
use typst::layout::{Frame, FrameItem, Point, Position, Size};
use typst::model::{Destination, Document};
use typst::syntax::{FileId, LinkedNode, Side, Source, Span, SyntaxKind};
use typst::visualize::Geometry;
use typst::World;

/// Where to [jump](jump_from_click) to.
#[derive(Debug, Clone, Eq, PartialEq)]
pub enum Jump {
    /// Jump to a position in a source file.
    Source(FileId, usize),
    /// Jump to an external URL.
    Url(EcoString),
    /// Jump to a point on a page.
    Position(Position),
}

impl Jump {
    fn from_span(world: &dyn World, span: Span) -> Option<Self> {
        let id = span.id()?;
        let source = world.source(id).ok()?;
        let node = source.find(span)?;
        Some(Self::Source(id, node.offset()))
    }
}

/// Determine where to jump to based on a click in a frame.
pub fn jump_from_click(
    world: &dyn World,
    document: &Document,
    frame: &Frame,
    click: Point,
) -> Option<Jump> {
    // Try to find a link first.
    for (pos, item) in frame.items() {
        if let FrameItem::Link(dest, size) = item {
            if is_in_rect(*pos, *size, click) {
                return Some(match dest {
                    Destination::Url(url) => Jump::Url(url.clone()),
                    Destination::Position(pos) => Jump::Position(*pos),
                    Destination::Location(loc) => {
                        Jump::Position(document.introspector.position(*loc))
                    }
                });
            }
        }
    }

    // If there's no link, search for a jump target.
    for (mut pos, item) in frame.items().rev() {
        match item {
            FrameItem::Group(group) => {
                // TODO: Handle transformation.
                if let Some(span) =
                    jump_from_click(world, document, &group.frame, click - pos)
                {
                    return Some(span);
                }
            }

            FrameItem::Text(text) => {
                for glyph in &text.glyphs {
                    let width = glyph.x_advance.at(text.size);
                    if is_in_rect(
                        Point::new(pos.x, pos.y - text.size),
                        Size::new(width, text.size),
                        click,
                    ) {
                        let (span, span_offset) = glyph.span;
                        let Some(id) = span.id() else { continue };
                        let source = world.source(id).ok()?;
                        let node = source.find(span)?;
                        let pos = if node.kind() == SyntaxKind::Text {
                            let range = node.range();
                            let mut offset = range.start + usize::from(span_offset);
                            if (click.x - pos.x) > width / 2.0 {
                                offset += glyph.range().len();
                            }
                            offset.min(range.end)
                        } else {
                            node.offset()
                        };
                        return Some(Jump::Source(source.id(), pos));
                    }

                    pos.x += width;
                }
            }

            FrameItem::Shape(shape, span) => {
                let Geometry::Rect(size) = shape.geometry else { continue };
                if is_in_rect(pos, size, click) {
                    return Jump::from_span(world, *span);
                }
            }

            FrameItem::Image(_, size, span) if is_in_rect(pos, *size, click) => {
                return Jump::from_span(world, *span);
            }

            _ => {}
        }
    }

    None
}

/// Find the output location in the document for a cursor position.
pub fn jump_from_cursor(
    document: &Document,
    source: &Source,
    cursor: usize,
) -> Vec<Position> {
    fn is_text(node: &LinkedNode) -> bool {
        node.get().kind() == SyntaxKind::Text
    }

    let root = LinkedNode::new(source.root());
    let Some(node) = root
        .leaf_at(cursor, Side::Before)
        .filter(is_text)
        .or_else(|| root.leaf_at(cursor, Side::After).filter(is_text))
    else {
        return vec![];
    };

    let span = node.span();
    document
        .pages
        .iter()
        .enumerate()
        .filter_map(|(i, page)| {
            find_in_frame(&page.frame, span)
                .map(|point| Position { page: NonZeroUsize::new(i + 1).unwrap(), point })
        })
        .collect()
}

/// Find the position of a span in a frame.
fn find_in_frame(frame: &Frame, span: Span) -> Option<Point> {
    for (mut pos, item) in frame.items() {
        if let FrameItem::Group(group) = item {
            // TODO: Handle transformation.
            if let Some(point) = find_in_frame(&group.frame, span) {
                return Some(point + pos);
            }
        }

        if let FrameItem::Text(text) = item {
            for glyph in &text.glyphs {
                if glyph.span.0 == span {
                    return Some(pos);
                }
                pos.x += glyph.x_advance.at(text.size);
            }
        }
    }

    None
}

/// Whether a rectangle with the given size at the given position contains the
/// click position.
fn is_in_rect(pos: Point, size: Size, click: Point) -> bool {
    pos.x <= click.x
        && pos.x + size.x >= click.x
        && pos.y <= click.y
        && pos.y + size.y >= click.y
}

#[cfg(test)]
mod tests {
    //! This can be used in a normal test to determine positions:
    //! ```
    //! #set page(background: place(
    //!   dx: 10pt,
    //!   dy: 10pt,
    //!   square(size: 2pt, fill: red),
    //! ))
    //! ```

    use std::num::NonZeroUsize;

    use typst::layout::{Abs, Point, Position};

    use super::{jump_from_click, jump_from_cursor, Jump};
    use crate::tests::TestWorld;

    fn point(x: f64, y: f64) -> Point {
        Point::new(Abs::pt(x), Abs::pt(y))
    }

    fn cursor(cursor: usize) -> Option<Jump> {
        Some(Jump::Source(TestWorld::main_id(), cursor))
    }

    fn pos(page: usize, x: f64, y: f64) -> Option<Position> {
        Some(Position {
            page: NonZeroUsize::new(page).unwrap(),
            point: point(x, y),
        })
    }

    macro_rules! assert_approx_eq {
        ($l:expr, $r:expr) => {
            assert!(($l.to_raw() - $r.to_raw()).abs() < 0.1, "{:?} ≉ {:?}", $l, $r);
        };
    }

    #[track_caller]
    fn test_click(text: &str, click: Point, expected: Option<Jump>) {
        let world = TestWorld::new(text);
        let doc = typst::compile(&world).output.unwrap();
        let jump = jump_from_click(&world, &doc, &doc.pages[0].frame, click);
        if let (Some(Jump::Position(pos)), Some(Jump::Position(expected))) =
            (&jump, &expected)
        {
            assert_eq!(pos.page, expected.page);
            assert_approx_eq!(pos.point.x, expected.point.x);
            assert_approx_eq!(pos.point.y, expected.point.y);
        } else {
            assert_eq!(jump, expected);
        }
    }

    #[track_caller]
    fn test_cursor(text: &str, cursor: usize, expected: Option<Position>) {
        let world = TestWorld::new(text);
        let doc = typst::compile(&world).output.unwrap();
        let pos = jump_from_cursor(&doc, &world.main, cursor);
        assert_eq!(!pos.is_empty(), expected.is_some());
        if let (Some(pos), Some(expected)) = (pos.first(), expected) {
            assert_eq!(pos.page, expected.page);
            assert_approx_eq!(pos.point.x, expected.point.x);
            assert_approx_eq!(pos.point.y, expected.point.y);
        }
    }

    #[test]
    fn test_jump_from_click() {
        let s = "*Hello* #box[ABC] World";
        test_click(s, point(0.0, 0.0), None);
        test_click(s, point(70.0, 5.0), None);
        test_click(s, point(45.0, 15.0), cursor(14));
        test_click(s, point(48.0, 15.0), cursor(15));
        test_click(s, point(72.0, 10.0), cursor(20));
    }

    #[test]
    fn test_jump_from_click_par_indents() {
        // There was a bug with span mapping due to indents generating
        // extra spacing.
        let s = "#set par(first-line-indent: 1cm, hanging-indent: 1cm);Hello";
        test_click(s, point(21.0, 12.0), cursor(56));
    }

    #[test]
    fn test_jump_from_cursor() {
        let s = "*Hello* #box[ABC] World";
        test_cursor(s, 12, None);
        test_cursor(s, 14, pos(1, 37.55, 16.58));
    }

    #[test]
    fn test_backlink() {
        let s = "#footnote[Hi]";
        test_click(s, point(10.0, 10.0), pos(1, 18.5, 37.1).map(Jump::Position));
    }
}
