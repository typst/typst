use ecow::EcoVec;
use typst::introspection::HtmlPosition;
use typst::layout::{Frame, FrameItem, PagedDocument, Point, Position, Size};
use typst::model::{Destination, Url};
use typst::syntax::{FileId, LinkedNode, Side, Source, Span, SyntaxKind};
use typst::visualize::{Curve, CurveItem, FillRule, Geometry};
use typst::{AsDocument, WorldExt};
use typst_html::{HtmlDocument, HtmlElement, HtmlNode};

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

/// Determine where to jump to, based on a click in a rendered document.
pub fn jump_from_click<D: JumpFromDocument>(
    world: &dyn IdeWorld,
    document: &D,
    position: &D::Position,
) -> Option<Jump> {
    document.resolve_position(world, position)
}

/// Maps a position in a document to a [jump destination][`Jump`], allowing for
/// click-to-jump functionnality.
pub trait JumpFromDocument: jump_from_document_sealed::JumpFromDocument {}
// The actual implementations are in the sealed trait.
impl JumpFromDocument for PagedDocument {}
impl JumpFromDocument for HtmlDocument {}

mod jump_from_document_sealed {
    use crate::IdeWorld;
    use typst::{
        introspection::{HtmlPosition, InnerHtmlPosition},
        layout::{PagedDocument, Position},
    };
    use typst_html::{HtmlDocument, HtmlNode};

    use super::{Jump, jump_from_click_in_frame, nth_child};

    /// See [`super::JumpFromDocument`].
    pub trait JumpFromDocument {
        type Position;

        fn resolve_position(
            &self,
            world: &dyn IdeWorld,
            position: &Self::Position,
        ) -> Option<Jump>;
    }

    impl JumpFromDocument for PagedDocument {
        type Position = Position;

        fn resolve_position(
            &self,
            world: &dyn IdeWorld,
            position: &Self::Position,
        ) -> Option<Jump> {
            let page = self.pages.get(position.page.get() - 1)?;
            let click = position.point;

            jump_from_click_in_frame(world, self, &page.frame, click)
        }
    }

    impl JumpFromDocument for HtmlDocument {
        type Position = HtmlPosition;

        fn resolve_position(
            &self,
            world: &dyn IdeWorld,
            position: &Self::Position,
        ) -> Option<Jump> {
            let mut current_node: &HtmlNode = &HtmlNode::Element(self.root.clone());
            for index in position.element() {
                match current_node {
                    HtmlNode::Element(html_element) => {
                        current_node = nth_child(*index, html_element)?;
                    }
                    HtmlNode::Tag(_) | HtmlNode::Text(_, _) | HtmlNode::Frame(_) => {
                        return None;
                    }
                }
            }

            let span = current_node.span();
            let id = span.id()?;
            let source = world.source(id).ok()?;
            let ast_node = source.find(span);
            let is_text_node =
                ast_node.is_some_and(|x| x.is::<typst::syntax::ast::Text>());

            if let (HtmlNode::Frame(frame), Some(InnerHtmlPosition::Frame(point))) =
                (current_node, &position.details())
            {
                return jump_from_click_in_frame(world, self, &frame.inner, *point);
            }

            Some(Jump::File(
                id,
                source.range(span).unwrap_or_default().start
                    + match (is_text_node, &position.details()) {
                        (true, Some(InnerHtmlPosition::Character(i))) => *i,
                        _ => 0,
                    },
            ))
        }
    }
}

/// Determine where to jump to based on a click in a frame.
pub fn jump_from_click_in_frame(
    world: &dyn IdeWorld,
    document: impl AsDocument,
    frame: &Frame,
    click: Point,
) -> Option<Jump> {
    let document = document.as_document();

    // Try to find a link first.
    for (pos, item) in frame.items() {
        if let FrameItem::Link(dest, size) = item
            && is_in_rect(*pos, *size, click)
        {
            return Some(match dest {
                Destination::Url(url) => Jump::Url(url.clone()),
                Destination::Position(pos) => Jump::Position(*pos),
                Destination::Location(loc) => Jump::Position(
                    document.introspector().position(*loc).as_paged_or_default(),
                ),
            });
        }
    }

    // If there's no link, search for a jump target.
    for &(mut pos, ref item) in frame.items().rev() {
        match item {
            FrameItem::Group(group) => {
                let pos = click - pos;
                if let Some(clip) = &group.clip
                    && !clip.contains(FillRule::NonZero, pos)
                {
                    continue;
                }
                // Realistic transforms should always be invertible.
                // An example of one that isn't is a scale of 0, which would
                // not be clickable anyway.
                let Some(inv_transform) = group.transform.invert() else {
                    continue;
                };
                let pos = pos.transform_inf(inv_transform);
                if let Some(span) =
                    jump_from_click_in_frame(world, document, &group.frame, pos)
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
                if shape.fill.is_some() {
                    let within = match &shape.geometry {
                        Geometry::Line(..) => false,
                        Geometry::Rect(size) => is_in_rect(pos, *size, click),
                        Geometry::Curve(curve) => {
                            curve.contains(shape.fill_rule, click - pos)
                        }
                    };
                    if within {
                        return Jump::from_span(world, *span);
                    }
                }

                if let Some(stroke) = &shape.stroke {
                    let within = !stroke.thickness.approx_empty() && {
                        // This curve is rooted at (0, 0), not `pos`.
                        let base_curve = match &shape.geometry {
                            Geometry::Line(to) => &Curve(vec![CurveItem::Line(*to)]),
                            Geometry::Rect(size) => &Curve::rect(*size),
                            Geometry::Curve(curve) => curve,
                        };
                        base_curve.stroke_contains(stroke, click - pos)
                    };
                    if within {
                        return Jump::from_span(world, *span);
                    }
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

/// Whether a rectangle with the given size at the given position contains the
/// click position.
fn is_in_rect(pos: Point, size: Size, click: Point) -> bool {
    pos.x <= click.x
        && pos.x + size.x >= click.x
        && pos.y <= click.y
        && pos.y + size.y >= click.y
}

/// Returns the n-th child of an HTML element, ignoring introspection tags, and
/// grouping sibling text nodes together as one.
fn nth_child(n: usize, elem: &HtmlElement) -> Option<&HtmlNode> {
    let mut i = 0;
    let mut was_text = false;
    for ch in &elem.children {
        if matches!(ch, HtmlNode::Tag(_)) {
            continue;
        }

        let is_text = matches!(ch, HtmlNode::Text(_, _));
        let contiguous_text_node = is_text && was_text;
        if i == n && !contiguous_text_node {
            return Some(ch);
        }

        if !contiguous_text_node {
            i += 1
        }

        was_text = is_text;
    }

    None
}

/// Find the output location in the document for a cursor position.
pub fn jump_from_cursor<D: JumpInDocument>(
    document: &D,
    source: &Source,
    cursor: usize,
) -> Vec<D::Position> {
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
    document.find_span(source, span)
}

/// Jump to a position in the document, given a cursor position in a source
/// file.
pub trait JumpInDocument: jump_in_document_sealed::JumpInDocument {}
// The actual implementations are in the sealed trait.
impl JumpInDocument for PagedDocument {}
impl JumpInDocument for HtmlDocument {}

/// Sealing for [`JumpInDocument`].
mod jump_in_document_sealed {
    use std::num::NonZeroUsize;

    use ecow::EcoVec;
    use typst::{
        introspection::HtmlPosition,
        layout::{PagedDocument, Position},
        syntax::{Source, Span},
    };
    use typst_html::HtmlDocument;

    use super::{find_in_elem, find_in_frame};

    /// See [`super::JumpInDocument`].
    pub trait JumpInDocument {
        type Position;

        fn find_span(&self, source: &Source, span: Span) -> Vec<Self::Position>;
    }

    impl JumpInDocument for PagedDocument {
        type Position = Position;

        fn find_span(&self, _: &Source, span: Span) -> Vec<Self::Position> {
            self.pages
                .iter()
                .enumerate()
                .filter_map(|(i, page)| {
                    find_in_frame(&page.frame, span).map(|point| Position {
                        page: NonZeroUsize::new(i + 1).unwrap(),
                        point,
                    })
                })
                .collect()
        }
    }

    impl JumpInDocument for HtmlDocument {
        type Position = HtmlPosition;

        fn find_span(&self, source: &Source, span: Span) -> Vec<Self::Position> {
            find_in_elem(source, &self.root, span, &mut EcoVec::new())
        }
    }
}

/// Find the position of a span in a frame.
fn find_in_frame(frame: &Frame, span: Span) -> Option<Point> {
    for &(mut pos, ref item) in frame.items() {
        if let FrameItem::Group(group) = item
            && let Some(point) = find_in_frame(&group.frame, span)
        {
            return Some(pos + point.transform(group.transform));
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

/// Find the position of a span in an HTML element.
fn find_in_elem(
    source: &Source,
    elem: &HtmlElement,
    span: Span,
    current_position: &mut EcoVec<usize>,
) -> Vec<HtmlPosition> {
    let mut result = Vec::new();
    let mut i = 0;
    let mut last_text_start = None;
    for child in &elem.children {
        match child {
            HtmlNode::Element(e) => {
                current_position.push(i);
                for pos in find_in_elem(source, e, span, current_position) {
                    result.push(pos)
                }
                current_position.pop();

                i += 1;
                last_text_start = None;
            }
            HtmlNode::Text(_, node_span) => {
                let text_start = last_text_start.get_or_insert(*node_span);

                if *node_span == span {
                    let span_range = source.range(*text_start);
                    return vec![
                        HtmlPosition::new(current_position.clone())
                            .at_char(span_range.unwrap_or_default().start),
                    ];
                }

                i += 1;
            }
            HtmlNode::Frame(frame) => {
                if let Some(frame_pos) = find_in_frame(&frame.inner, span) {
                    let mut position = current_position.clone();
                    position.push(i);
                    return vec![HtmlPosition::new(position).in_frame(frame_pos)];
                }

                i += 1;
                last_text_start = None;
            }
            _ => {}
        }
    }

    result
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

    use ecow::eco_vec;
    use typst::{
        World,
        introspection::HtmlPosition,
        layout::{Abs, PagedDocument, Point, Position},
        utils::NonZeroExt,
    };
    use typst_html::HtmlDocument;

    use super::{Jump, jump_from_cursor};
    use crate::{
        jump_from_click,
        tests::{FilePos, TestWorld, WorldLike},
    };

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
    fn assert_jump_eq(jump: Option<Jump>, expected: Option<Jump>) {
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
    fn test_click(world: impl WorldLike, click: Point, expected: Option<Jump>) {
        let world = world.acquire();
        let world = world.borrow();
        let doc: PagedDocument = typst::compile(world).output.unwrap();
        let jump = jump_from_click(
            world,
            &doc,
            &Position { page: NonZeroUsize::ONE, point: click },
        );
        assert_jump_eq(jump, expected);
    }

    #[track_caller]
    fn test_click_html(
        world: impl WorldLike,
        click: HtmlPosition,
        expected: Option<Jump>,
    ) {
        let world = world.acquire();
        let world = world.borrow();
        let doc: HtmlDocument = typst::compile(world).output.unwrap();
        let jump = jump_from_click(world, &doc, &click);
        assert_jump_eq(jump, expected);
    }

    #[track_caller]
    fn test_cursor(world: impl WorldLike, pos: impl FilePos, expected: Option<Position>) {
        let world = world.acquire();
        let world = world.borrow();
        let doc: PagedDocument = typst::compile(world).output.unwrap();
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
            "#rect(width: 20pt, height: 20pt, fill: black)",
            point(10.0, 10.0) + margin,
            cursor(1),
        );
        test_click(
            "#rect(width: 60pt, height: 10pt, fill: black)",
            point(5.0, 30.0) + margin,
            None,
        );
        test_click(
            "#rotate(90deg, origin: bottom + left, rect(width: 60pt, height: 10pt, fill: black))",
            point(5.0, 30.0) + margin,
            cursor(38),
        );
        test_click(
            "#scale(x: 300%, y: 300%, origin: top + left, rect(width: 10pt, height: 10pt, fill: black))",
            point(20.0, 20.0) + margin,
            cursor(45),
        );
        test_click(
            "#box(width: 10pt, height: 10pt, clip: true, scale(x: 300%, y: 300%, \
             origin: top + left, rect(width: 10pt, height: 10pt, fill: black)))",
            point(20.0, 20.0) + margin,
            None,
        );
        test_click(
            "#box(width: 10pt, height: 10pt, clip: false, rect(width: 30pt, height: 30pt, fill: black))",
            point(20.0, 20.0) + margin,
            cursor(45),
        );
        test_click(
            "#box(width: 10pt, height: 10pt, clip: true, rect(width: 30pt, height: 30pt, fill: black))",
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
    fn test_jump_from_click_shapes() {
        let margin = point(10.0, 10.0);

        test_click(
            "#rect(width: 30pt, height: 30pt, fill: black)",
            point(15.0, 15.0) + margin,
            cursor(1),
        );

        let circle = "#circle(width: 30pt, height: 30pt, fill: black)";
        test_click(circle, point(15.0, 15.0) + margin, cursor(1));
        test_click(circle, point(1.0, 1.0) + margin, None);

        let bowtie =
            "#polygon(fill: black, (0pt, 0pt), (20pt, 20pt), (20pt, 0pt), (0pt, 20pt))";
        test_click(bowtie, point(1.0, 2.0) + margin, cursor(1));
        test_click(bowtie, point(2.0, 1.0) + margin, None);
        test_click(bowtie, point(19.0, 10.0) + margin, cursor(1));

        let evenodd = r#"#polygon(fill: black, fill-rule: "even-odd",
            (0pt, 10pt), (30pt, 10pt), (30pt, 20pt), (20pt, 20pt),
            (20pt, 0pt), (10pt, 0pt), (10pt, 30pt), (20pt, 30pt),
            (20pt, 20pt), (0pt, 20pt))"#;
        test_click(evenodd, point(15.0, 15.0) + margin, None);
        test_click(evenodd, point(5.0, 15.0) + margin, cursor(1));
        test_click(evenodd, point(15.0, 5.0) + margin, cursor(1));
    }

    #[test]
    fn test_jump_from_click_shapes_stroke() {
        let margin = point(10.0, 10.0);

        let rect =
            "#place(dx: 10pt, dy: 10pt, rect(width: 10pt, height: 10pt, stroke: 5pt))";
        test_click(rect, point(15.0, 15.0) + margin, None);
        test_click(rect, point(10.0, 15.0) + margin, cursor(27));

        test_click(
            "#line(angle: 45deg, length: 10pt, stroke: 2pt)",
            point(2.0, 2.0) + margin,
            cursor(1),
        );
    }

    #[test]
    fn test_jump_from_click_html() {
        let src = "This is a test.\n\nWith multiple elements.\n\nAnd some *formatting*.";
        let main = src.acquire().main();
        test_click_html(
            src,
            // Click in the middle of "some"
            HtmlPosition::new(eco_vec![1, 2, 0]).at_char(6),
            Some(Jump::File(main, 48)),
        );
    }

    #[test]
    fn test_jump_from_click_html_introspection() {
        // raw blocks have introspection tags around them, check that they are ignored.
        let src =
            "This is a test.\n\n```\nwith some code\n```\n\nAnd `some` *formatting*.";
        let main = src.acquire().main();
        test_click_html(
            src,
            // Click in the middle of "some"
            HtmlPosition::new(eco_vec![1, 2, 1, 0]).at_char(2),
            Some(Jump::File(main, 48)),
        );
    }

    #[test]
    fn test_jump_from_click_html_frame() {
        let src = "A math formula:\n\n#html.frame($a x + b = 0$)";
        let main = src.acquire().main();
        test_click_html(
            src,
            // Click on the "b" in the math formula
            HtmlPosition::new(eco_vec![1, 1]).in_frame(point(27.0, 5.0)),
            Some(Jump::File(main, 37)),
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
    fn test_footnote_links() {
        let s = "#footnote[Hi]";
        test_click(s, point(10.0, 10.0), pos(1, 10.0, 31.58).map(Jump::Position));
        test_click(s, point(19.0, 33.0), pos(1, 10.0, 16.58).map(Jump::Position));
    }

    #[test]
    fn test_footnote_link_entry_customized() {
        let s = "#show footnote.entry: [Replaced]; #footnote[Hi]";
        test_click(s, point(10.0, 10.0), pos(1, 10.0, 31.58).map(Jump::Position));
    }
}
