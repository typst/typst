use std::num::NonZeroUsize;

use crate::doc::{Destination, Element, Frame, Location, Meta};
use crate::geom::{Point, Size};
use crate::syntax::{LinkedNode, Source, SourceId, Span, SyntaxKind};
use crate::World;

/// Where to [jump](jump_from_click) to.
#[derive(Debug, Clone, Eq, PartialEq)]
pub enum Jump {
    /// Jump to a position in a source file.
    Source(SourceId, usize),
    /// Jump to position in the output or to an external URL.
    Dest(Destination),
}

/// Determine where to jump to based on a click in a frame.
pub fn jump_from_click(world: &dyn World, frame: &Frame, click: Point) -> Option<Jump> {
    for (mut pos, element) in frame.elements() {
        if let Element::Group(group) = element {
            // TODO: Handle transformation.
            if let Some(span) = jump_from_click(world, &group.frame, click - pos) {
                return Some(span);
            }
        }

        if let Element::Text(text) = element {
            for glyph in &text.glyphs {
                if glyph.span.is_detached() {
                    continue;
                }

                let width = glyph.x_advance.at(text.size);
                if is_in_rect(
                    Point::new(pos.x, pos.y - text.size),
                    Size::new(width, text.size),
                    click,
                ) {
                    let source = world.source(glyph.span.source());
                    let node = source.find(glyph.span);
                    let pos = if node.kind() == SyntaxKind::Text {
                        let range = node.range();
                        (range.start + usize::from(glyph.offset)).min(range.end)
                    } else {
                        node.offset()
                    };
                    return Some(Jump::Source(source.id(), pos));
                }

                pos.x += width;
            }
        }

        if let Element::Meta(Meta::Link(dest), size) = element {
            if is_in_rect(pos, *size, click) {
                return Some(Jump::Dest(dest.clone()));
            }
        }
    }

    None
}

/// Find the output location in the document for a cursor position.
pub fn jump_from_cursor(
    frames: &[Frame],
    source: &Source,
    cursor: usize,
) -> Option<Location> {
    let node = LinkedNode::new(source.root()).leaf_at(cursor)?;
    if node.kind() != SyntaxKind::Text {
        return None;
    }

    let span = node.span();
    for (i, frame) in frames.iter().enumerate() {
        if let Some(pos) = find_in_frame(frame, span) {
            return Some(Location { page: NonZeroUsize::new(i + 1).unwrap(), pos });
        }
    }

    None
}

/// Find the position of a span in a frame.
fn find_in_frame(frame: &Frame, span: Span) -> Option<Point> {
    for (mut pos, element) in frame.elements() {
        if let Element::Group(group) = element {
            // TODO: Handle transformation.
            if let Some(point) = find_in_frame(&group.frame, span) {
                return Some(point + pos);
            }
        }

        if let Element::Text(text) = element {
            for glyph in &text.glyphs {
                if glyph.span == span {
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
