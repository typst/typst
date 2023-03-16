use std::num::NonZeroUsize;

use crate::doc::{Destination, Element, Frame, Location, Meta};
use crate::geom::{Geometry, Point, Size};
use crate::model::Introspector;
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

impl Jump {
    fn from_span(world: &dyn World, span: Span) -> Self {
        let source = world.source(span.source());
        let node = source.find(span);
        Self::Source(source.id(), node.offset())
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

    // Prefer metadata.
    for (pos, element) in frame.elements() {
        if let Element::Meta(Meta::Link(link), size) = element {
            if is_in_rect(*pos, *size, click) {
                let dest = link.resolve(|| {
                    introspector.get_or_insert_with(|| Introspector::new(frames))
                });

                let Some(dest) = dest else { continue };
                return Some(Jump::Dest(dest));
            }
        }
    }

    for (mut pos, element) in frame.elements().rev() {
        match element {
            Element::Group(group) => {
                // TODO: Handle transformation.
                if let Some(span) =
                    jump_from_click(world, frames, &group.frame, click - pos)
                {
                    return Some(span);
                }
            }

            Element::Text(text) => {
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
                            let mut offset = range.start + usize::from(glyph.offset);
                            if (click.x - pos.x) > width / 2.0 {
                                offset += glyph.c.len_utf8();
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

            Element::Shape(shape, span) => {
                let Geometry::Rect(size) = shape.geometry else { continue };
                if is_in_rect(pos, size, click) {
                    return Some(Jump::from_span(world, *span));
                }
            }

            Element::Image(_, size, span) if is_in_rect(pos, *size, click) => {
                return Some(Jump::from_span(world, *span));
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
