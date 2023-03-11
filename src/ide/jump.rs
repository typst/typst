use std::num::NonZeroUsize;

use crate::doc::{Element, Frame, Location};
use crate::geom::Point;
use crate::syntax::{LinkedNode, Source, Span, SyntaxKind};
use crate::World;

/// Find the source file and byte offset for a click position.
pub fn jump_to_source<'a>(
    world: &'a dyn World,
    frame: &Frame,
    click: Point,
) -> Option<(&'a Source, usize)> {
    for (mut pos, element) in frame.elements() {
        if let Element::Text(text) = element {
            for glyph in &text.glyphs {
                if glyph.span.is_detached() {
                    continue;
                }

                let width = glyph.x_advance.at(text.size);
                if pos.x <= click.x
                    && pos.x + width >= click.x
                    && pos.y >= click.y
                    && pos.y - text.size <= click.y
                {
                    let source = world.source(glyph.span.source());
                    let node = source.find(glyph.span);
                    let pos = if node.kind() == SyntaxKind::Text {
                        let range = node.range();
                        (range.start + usize::from(glyph.offset)).min(range.end)
                    } else {
                        node.offset()
                    };
                    return Some((source, pos));
                }

                pos.x += width;
            }
        }

        if let Element::Group(group) = element {
            if let Some(span) = jump_to_source(world, &group.frame, click - pos) {
                return Some(span);
            }
        }
    }

    None
}

/// Find the output location for a cursor position.
pub fn jump_to_preview(
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
        if let Element::Text(text) = element {
            for glyph in &text.glyphs {
                if glyph.span == span {
                    return Some(pos);
                }
                pos.x += glyph.x_advance.at(text.size);
            }
        }

        if let Element::Group(group) = element {
            if let Some(point) = find_in_frame(&group.frame, span) {
                return Some(point + pos);
            }
        }
    }

    None
}
