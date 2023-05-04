use std::num::NonZeroUsize;

use ecow::EcoString;

use crate::doc::{Destination, Frame, FrameItem, Meta, Position};
use crate::geom::{Geometry, Point, Size};
use crate::model::Introspector;
use crate::syntax::{LinkedNode, Source, SourceId, Span, SyntaxKind};
use crate::World;

/// Where to [jump](jump_from_click) to.
#[derive(Debug, Clone, Eq, PartialEq)]
pub enum Jump {
    /// Jump to a position in a source file.
    Source(SourceId, usize),
    /// Jump to an external URL.
    Url(EcoString),
    /// Jump to a point on a page.
    Position(Position),
}

impl Jump {
    fn from_span(world: &dyn World, span: Span) -> Option<Self> {
        let source = world.source(span.source());
        let node = source.find(span)?;
        Some(Self::Source(source.id(), node.offset()))
    }
}

/// Determine where to jump to based on a click in a frame.
pub fn jump_from_click(
    world: &dyn World,
    frames: &[Frame],
    frame: &Frame,
    click: Point,
) -> Option<Jump> {
    let mut introspector = None;

    // Try to find a link first.
    for (pos, item) in frame.items() {
        if let FrameItem::Meta(Meta::Link(dest), size) = item {
            if is_in_rect(*pos, *size, click) {
                return Some(match dest {
                    Destination::Url(url) => Jump::Url(url.clone()),
                    Destination::Position(pos) => Jump::Position(*pos),
                    Destination::Location(loc) => Jump::Position(
                        introspector
                            .get_or_insert_with(|| Introspector::new(frames))
                            .position(*loc),
                    ),
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
                    jump_from_click(world, frames, &group.frame, click - pos)
                {
                    return Some(span);
                }
            }

            FrameItem::Text(text) => {
                for glyph in &text.glyphs {
                    let (span, span_offset) = glyph.span;
                    if span.is_detached() {
                        continue;
                    }

                    let width = glyph.x_advance.at(text.size);
                    if is_in_rect(
                        Point::new(pos.x, pos.y - text.size),
                        Size::new(width, text.size),
                        click,
                    ) {
                        let source = world.source(span.source());
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
    frames: &[Frame],
    source: &Source,
    cursor: usize,
) -> Option<Position> {
    let node = LinkedNode::new(source.root()).leaf_at(cursor)?;
    if node.kind() != SyntaxKind::Text {
        return None;
    }

    let span = node.span();
    for (i, frame) in frames.iter().enumerate() {
        if let Some(pos) = find_in_frame(frame, span) {
            return Some(Position {
                page: NonZeroUsize::new(i + 1).unwrap(),
                point: pos,
            });
        }
    }

    None
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
