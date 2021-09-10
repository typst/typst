use std::rc::Rc;

use serde::{Deserialize, Serialize};

use super::{Constrained, Constraints};
use crate::color::Color;
use crate::font::FaceId;
use crate::geom::{Em, Length, Path, Point, Size};
use crate::image::ImageId;

/// A finished layout with elements at fixed positions.
#[derive(Debug, Default, Clone, Eq, PartialEq, Serialize, Deserialize)]
pub struct Frame {
    /// The size of the frame.
    pub size: Size,
    /// The baseline of the frame measured from the top.
    pub baseline: Length,
    /// The elements composing this layout.
    pub children: Vec<(Point, FrameChild)>,
}

/// A frame can contain two different kinds of children: a leaf element or a
/// nested frame.
#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
pub enum FrameChild {
    /// A leaf node in the frame tree.
    Element(Element),
    /// An interior node with an optional index.
    Frame(Option<usize>, Rc<Frame>),
}

impl Frame {
    /// Create a new, empty frame.
    #[track_caller]
    pub fn new(size: Size, baseline: Length) -> Self {
        assert!(size.is_finite());
        Self { size, baseline, children: vec![] }
    }

    /// Add an element at a position in the foreground.
    pub fn push(&mut self, pos: Point, element: Element) {
        self.children.push((pos, FrameChild::Element(element)));
    }

    /// Add an element at a position in the background.
    pub fn prepend(&mut self, pos: Point, element: Element) {
        self.children.insert(0, (pos, FrameChild::Element(element)));
    }

    /// Add a frame element.
    pub fn push_frame(&mut self, pos: Point, subframe: Rc<Self>) {
        self.children.push((pos, FrameChild::Frame(None, subframe)))
    }

    /// Add a frame element with an index of arbitrary use.
    pub fn push_indexed_frame(&mut self, pos: Point, index: usize, subframe: Rc<Self>) {
        self.children.push((pos, FrameChild::Frame(Some(index), subframe)));
    }

    /// Add all elements of another frame, placing them relative to the given
    /// position.
    pub fn merge_frame(&mut self, pos: Point, subframe: Self) {
        if pos == Point::zero() && self.children.is_empty() {
            self.children = subframe.children;
        } else {
            for (subpos, child) in subframe.children {
                self.children.push((pos + subpos, child));
            }
        }
    }

    /// Wrap the frame with constraints.
    pub fn constrain(self, constraints: Constraints) -> Constrained<Rc<Self>> {
        Constrained { item: Rc::new(self), constraints }
    }

    /// An iterator over all elements in the frame and its children.
    pub fn elements(&self) -> Elements {
        Elements { stack: vec![(0, Point::zero(), self)] }
    }
}

/// An iterator over all elements in a frame, alongside with their positions.
pub struct Elements<'a> {
    stack: Vec<(usize, Point, &'a Frame)>,
}

impl<'a> Iterator for Elements<'a> {
    type Item = (Point, &'a Element);

    fn next(&mut self) -> Option<Self::Item> {
        let (cursor, offset, frame) = self.stack.last_mut()?;
        match frame.children.get(*cursor) {
            Some((pos, FrameChild::Frame(_, f))) => {
                let new_offset = *offset + *pos;
                self.stack.push((0, new_offset, f.as_ref()));
                self.next()
            }
            Some((pos, FrameChild::Element(e))) => {
                *cursor += 1;
                Some((*offset + *pos, e))
            }
            None => {
                self.stack.pop();
                if let Some((cursor, _, _)) = self.stack.last_mut() {
                    *cursor += 1;
                }
                self.next()
            }
        }
    }
}

/// The building block frames are composed of.
#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
pub enum Element {
    /// Shaped text.
    Text(Text),
    /// A geometric shape and the paint which with it should be filled or
    /// stroked.
    Geometry(Geometry, Paint),
    /// A raster image.
    Image(ImageId, Size),
    /// A link to an external resource.
    Link(String, Size),
}

/// A run of shaped text.
#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
pub struct Text {
    /// The font face the glyphs are contained in.
    pub face_id: FaceId,
    /// The font size.
    pub size: Length,
    /// The width of the text run.
    pub width: Length,
    /// Glyph color.
    pub fill: Paint,
    /// The glyphs.
    pub glyphs: Vec<Glyph>,
}

/// A glyph in a run of shaped text.
#[derive(Debug, Copy, Clone, Eq, PartialEq, Serialize, Deserialize)]
pub struct Glyph {
    /// The glyph's index in the face.
    pub id: u16,
    /// The advance width of the glyph.
    pub x_advance: Em,
    /// The horizontal offset of the glyph.
    pub x_offset: Em,
}

/// A geometric shape.
#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
pub enum Geometry {
    /// A filled rectangle with its origin in the topleft corner.
    Rect(Size),
    /// A filled ellipse with its origin in the center.
    Ellipse(Size),
    /// A stroked line to a point (relative to its position) with a thickness.
    Line(Point, Length),
    /// A filled bezier path.
    Path(Path),
}

/// How a fill or stroke should be painted.
#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash, Serialize, Deserialize)]
pub enum Paint {
    /// A solid color.
    Color(Color),
}
