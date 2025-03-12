use std::num::NonZeroUsize;

use typst::layout::{Frame, FrameItem, PagedDocument, Point, Position, Size};
use typst::model::{Destination, Url};
use typst::syntax::{FileId, LinkedNode, Side, Source, Span, SyntaxKind};
use typst::visualize::Geometry;
use typst::WorldExt;

use crate::IdeWorld;

/// Where to [jump](jump_from_click) to.
#[derive(Debug, Clone, Eq, PartialEq)]
pub enum Jump {
    /// Jump to a position in a file.
    File(FileId, usize),
    /// Jump to an external URL.
    Url(Url),
    /// Jump to a point on a page.
    Position(Position),
}

impl Jump {
    fn from_span(world: &dyn IdeWorld, span: Span) -> Option<Self> {
        let id = span.id()?;
        let offset = world.range(span)?.start;
        Some(Self::File(id, offset))
    }
}

/// Determine where to jump to based on a click in a frame.
pub fn jump_from_click(
    world: &dyn IdeWorld,
    document: &PagedDocument,
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
                let pos = click - pos;
                if let Some(clip) = &group.clip {
                    if !clip.contains(pos) {
                        continue;
                    }
                }
                // Realistic transforms should always be invertible.
                // An example of one that isn't is a scale of 0, which would
                // not be clickable anyway.
                let Some(inv_transform) = group.transform.invert() else {
                    continue;
                };
                let pos = pos.transform_inf(inv_transform);
                if let Some(span) = jump_from_click(world, document, &group.frame, pos) {
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
                        let pos = if matches!(
                            node.kind(),
                            SyntaxKind::Text | SyntaxKind::MathText
                        ) {
                            let range = node.range();
                            let mut offset = range.start + usize::from(span_offset);
                            if (click.x - pos.x) > width / 2.0 {
                                offset += glyph.range().len();
                            }
                            offset.min(range.end)
                        } else {
                            node.offset()
                        };
                        return Some(Jump::File(source.id(), pos));
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
    document: &PagedDocument,
    source: &Source,
    cursor: usize,
) -> Vec<Position> {
    fn is_text(node: &LinkedNode) -> bool {
        matches!(node.kind(), SyntaxKind::Text | SyntaxKind::MathText)
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
            if let Some(point) = find_in_frame(&group.frame, span) {
                return Some(pos + point.transform(group.transform));
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

    use std::borrow::Borrow;
    use std::num::NonZeroUsize;

    use typst::layout::{Abs, Point, Position};

    use super::{jump_from_click, jump_from_cursor, Jump};
    use crate::tests::{FilePos, TestWorld, WorldLike};

    fn point(x: f64, y: f64) -> Point {
        Point::new(Abs::pt(x), Abs::pt(y))
    }

    fn cursor(cursor: usize) -> Option<Jump> {
        Some(Jump::File(TestWorld::main_id(), cursor))
    }

    fn pos(page: usize, x: f64, y: f64) -> Option<Position> {
        Some(Position {
            page: NonZeroUsize::new(page).unwrap(),
            point: point(x, y),
        })
    }

    macro_rules! assert_approx_eq {
        ($l:expr, $r:expr) => {
            assert!(($l - $r).abs() < Abs::pt(0.1), "{:?} ≉ {:?}", $l, $r);
        };
    }

    #[track_caller]
    fn test_click(world: impl WorldLike, click: Point, expected: Option<Jump>) {
        let world = world.acquire();
        let world = world.borrow();
        let doc = typst::compile(world).output.unwrap();
        let jump = jump_from_click(world, &doc, &doc.pages[0].frame, click);
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
    fn test_cursor(world: impl WorldLike, pos: impl FilePos, expected: Option<Position>) {
        let world = world.acquire();
        let world = world.borrow();
        let doc = typst::compile(world).output.unwrap();
        let (source, cursor) = pos.resolve(world);
        let pos = jump_from_cursor(&doc, &source, cursor);
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
    fn test_jump_from_click_math() {
        test_click("$a + b$", point(28.0, 14.0), cursor(5));
    }

    #[test]
    fn test_jump_from_click_transform_clip() {
        let margin = point(10.0, 10.0);
        test_click(
            "#rect(width: 20pt, height: 20pt)",
            point(10.0, 10.0) + margin,
            cursor(1),
        );
        test_click("#rect(width: 60pt, height: 10pt)", point(5.0, 30.0) + margin, None);
        test_click(
            "#rotate(90deg, origin: bottom + left, rect(width: 60pt, height: 10pt))",
            point(5.0, 30.0) + margin,
            cursor(38),
        );
        test_click(
            "#scale(x: 300%, y: 300%, origin: top + left, rect(width: 10pt, height: 10pt))",
            point(20.0, 20.0) + margin,
            cursor(45),
        );
        test_click(
            "#box(width: 10pt, height: 10pt, clip: true, scale(x: 300%, y: 300%, \
             origin: top + left, rect(width: 10pt, height: 10pt)))",
            point(20.0, 20.0) + margin,
            None,
        );
        test_click(
            "#box(width: 10pt, height: 10pt, clip: false, rect(width: 30pt, height: 30pt))",
            point(20.0, 20.0) + margin,
            cursor(45),
        );
        test_click(
            "#box(width: 10pt, height: 10pt, clip: true, rect(width: 30pt, height: 30pt))",
            point(20.0, 20.0) + margin,
            None,
        );
        test_click(
            "#rotate(90deg, origin: bottom + left)[hello world]",
            point(5.0, 15.0) + margin,
            cursor(40),
        );
    }

    #[test]
    fn test_jump_from_cursor() {
        let s = "*Hello* #box[ABC] World";
        test_cursor(s, 12, None);
        test_cursor(s, 14, pos(1, 37.55, 16.58));
    }

    #[test]
    fn test_jump_from_cursor_math() {
        test_cursor("$a + b$", -3, pos(1, 27.51, 16.83));
    }

    #[test]
    fn test_jump_from_cursor_transform() {
        test_cursor(
            r#"#rotate(90deg, origin: bottom + left, [hello world])"#,
            -5,
            pos(1, 10.0, 16.58),
        );
    }

    #[test]
    fn test_backlink() {
        let s = "#footnote[Hi]";
        test_click(s, point(10.0, 10.0), pos(1, 18.5, 37.1).map(Jump::Position));
    }
}
