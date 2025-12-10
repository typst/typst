use ecow::EcoVec;
use typst::introspection::{DocumentPosition, HtmlPosition};
use typst::layout::{Frame, FrameItem, PagedDocument, Point, Position, Size};
use typst::model::{Destination, Url};
use typst::syntax::{FileId, LinkedNode, Side, Source, Span, SyntaxKind};
use typst::visualize::{Curve, CurveItem, FillRule, Geometry};
use typst::{AsDocument, WorldExt};
use typst_html::{HtmlDocument, HtmlElement, HtmlNode, HtmlSliceExt};

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
    use typst::introspection::{HtmlPosition, InnerHtmlPosition};
    use typst::layout::{PagedDocument, Position};
    use typst_html::{HtmlDocument, HtmlNode, HtmlSliceExt};

    use super::{Jump, jump_from_click_in_frame};
    use crate::IdeWorld;

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
            let mut prefix_len = 0;

            let indices_count = position.element().count();
            for (i, index) in position.element().enumerate() {
                let reached_leaf_node = i == indices_count - 1;
                match current_node {
                    HtmlNode::Element(html_element) => {
                        let (child_index, (mut child, _)) = html_element
                            .children
                            .iter_with_dom_indices()
                            .enumerate()
                            .find(|(_, (child, dom_index))| {
                                !matches!(child, HtmlNode::Tag(_)) && dom_index == index
                            })?;

                        // In some scenarios, Typst will emit multiple
                        // consecutive text nodes (called text node parts below),
                        // the firsts of which may have detached spans. This is
                        // for example the case with the default figure
                        // captions: the supplement, counter, and separator will
                        // be individual spanless text nodes (and only the actual
                        // caption body will have a span).
                        //
                        // Because the HTML document as parsed by an external
                        // program will probably contain a single text node for
                        // all that, jumping from the caption body will ask for
                        // a jump from the 0-th child of the <figcaption>, at a
                        // certain offset. Because `nth_child` doesn't take this
                        // offset into account, it will then pick the first text
                        // node: in the previous example, the spanless
                        // supplement.
                        //
                        // Below, we compensate for that, and make sure the
                        // position can be correctly resolved by picking another
                        // text node if needed.
                        if reached_leaf_node
                            && let HtmlNode::Text(_, _) = child
                            && let Some(InnerHtmlPosition::Character(offset)) =
                                position.details()
                        {
                            let mut text_char_count = 0;
                            let mut text_node_part = child;
                            let mut text_node_offset = 0;

                            // The requested offset is expressed as a character
                            // index (not a byte offset).
                            while text_char_count < *offset {
                                prefix_len = text_char_count;

                                // Get the current text node part
                                text_node_part = html_element
                                    .children
                                    .get(child_index + text_node_offset)?;

                                // And measure its length (in characters), to be
                                // able to tell if we are far enough to have
                                // reached the character to which we want to
                                // jump to.
                                let text_node_part_len =
                                    if let HtmlNode::Text(text, _) = text_node_part {
                                        text.chars().count()
                                    } else {
                                        0
                                    };

                                // Prepare the iteration to the next text node,
                                // that will happen if we have not yet reached
                                // the requested character offset.
                                text_char_count += text_node_part_len;
                                text_node_offset += 1;
                            }

                            child = text_node_part
                        }

                        current_node = child;
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

            let source_range = source.range(span)?;
            Some(Jump::File(
                id,
                source_range.start
                    + match (is_text_node, &position.details()) {
                        (true, Some(InnerHtmlPosition::Character(i))) => {
                            let source_text = &source.text()[source_range];
                            let slice: String = source_text
                                .chars()
                                .take(i.saturating_sub(prefix_len))
                                .collect();
                            slice.len()
                        }
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
            match dest {
                Destination::Url(url) => return Some(Jump::Url(url.clone())),
                Destination::Position(pos) => return Some(Jump::Position(*pos)),
                Destination::Location(loc) => {
                    if let DocumentPosition::Paged(pos) =
                        document.introspector().position(*loc)
                    {
                        return Some(Jump::Position(pos));
                    }
                }
            }
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
    document.find_span(span)
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
    use typst::introspection::HtmlPosition;
    use typst::layout::{PagedDocument, Position};
    use typst::syntax::Span;
    use typst_html::HtmlDocument;

    use super::{find_in_elem, find_in_frame};

    /// See [`super::JumpInDocument`].
    pub trait JumpInDocument {
        type Position;

        fn find_span(&self, span: Span) -> Vec<Self::Position>;
    }

    impl JumpInDocument for PagedDocument {
        type Position = Position;

        fn find_span(&self, span: Span) -> Vec<Self::Position> {
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

        fn find_span(&self, span: Span) -> Vec<Self::Position> {
            find_in_elem(&self.root, span, &mut EcoVec::new())
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
    elem: &HtmlElement,
    span: Span,
    current_position: &mut EcoVec<usize>,
) -> Vec<HtmlPosition> {
    let mut result = Vec::new();

    for (child, dom_index) in elem.children.iter_with_dom_indices() {
        match child {
            HtmlNode::Tag(_) => {}
            HtmlNode::Element(e) => {
                current_position.push(dom_index);
                result.extend(find_in_elem(e, span, current_position));
                current_position.pop();
            }
            HtmlNode::Text(_, node_span) => {
                if *node_span == span {
                    return vec![HtmlPosition::new(current_position.clone())];
                }
            }
            HtmlNode::Frame(frame) => {
                if let Some(frame_pos) = find_in_frame(&frame.inner, span) {
                    let mut position = current_position.clone();
                    position.push(dom_index);
                    return vec![HtmlPosition::new(position).in_frame(frame_pos)];
                }
            }
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
    use typst::introspection::HtmlPosition;
    use typst::layout::{Abs, PagedDocument, Point, Position};
    use typst::utils::NonZeroExt;
    use typst_html::HtmlDocument;

    use super::{Jump, jump_from_click, jump_from_cursor};
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
        test_click_html(
            "This is a test.\n\nWith multiple elements.\n\nAnd some *formatting*.",
            // Click in the middle of "some"
            HtmlPosition::new(eco_vec![1, 2, 0]).at_char(6),
            cursor(48),
        );
    }

    #[test]
    fn test_jump_from_click_html_introspection() {
        test_click_html(
            // Raw blocks have introspection tags around them, check that they
            // are ignored.
            "This is a test.\n\n```\nwith some code\n```\n\nAnd `some` *formatting*.",
            // Click in the middle of "some"
            HtmlPosition::new(eco_vec![1, 2, 1, 0]).at_char(2),
            cursor(48),
        );
    }

    #[test]
    fn test_jump_from_click_html_frame() {
        test_click_html(
            "A math formula:\n\n#html.frame($a x + b = 0$)",
            // Click on the "b" in the math formula
            HtmlPosition::new(eco_vec![1, 1]).in_frame(point(27.0, 5.0)),
            cursor(37),
        );
    }

    #[test]
    fn test_jump_from_click_html_bindings() {
        let src = "#let a = [This]; #let b = [exists]; #a#b";
        test_click_html(
            src,
            // Click at "exis|ts"
            HtmlPosition::new(eco_vec![1, 0, 0]).at_char(8),
            cursor(src.find("ts];").unwrap()),
        );
    }

    #[test]
    fn test_jump_from_click_html_figcaption() {
        let src = "#figure([Hello, world!], caption: [Output of the program.])";
        test_click_html(
            src,
            // Click on the first "t" in the caption
            HtmlPosition::new(eco_vec![1, 0, 1, 0])
                .at_char("Fig. 1 — Out".chars().count()),
            cursor(src.find("tput of").unwrap()),
        );
    }

    // Make sure that the jump_from_click function uses character indices for
    // the click, and bytes for the returned jump location and not other units
    // to express text offsets.
    #[test]
    fn test_jump_from_click_html_offset_units() {
        test_click_html(
            "Ça va ?",
            // Clicking on the "Ç" in the emitted <p> should map to the "Ç" in
            // the source.
            HtmlPosition::new(eco_vec![1, 0]).at_char(1),
            cursor(2),
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
