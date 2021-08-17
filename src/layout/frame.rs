use std::rc::Rc;

use serde::{Deserialize, Serialize};

use super::{Constrained, Constraints};
use crate::color::Color;
use crate::font::FaceId;
use crate::geom::{Length, Path, Point, Size};
use crate::image::ImageId;

/// A finished layout with elements at fixed positions.
#[derive(Debug, Default, Clone, Eq, PartialEq, Serialize, Deserialize)]
pub struct Frame {
    /// The size of the frame.
    pub size: Size,
    /// The baseline of the frame measured from the top.
    pub baseline: Length,
    /// The elements composing this layout.
    children: Vec<(Point, Child)>,
}

/// A frame can contain multiple children: elements or other frames, complete
/// with their children.
#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
enum Child {
    Element(Element),
    Frame(Rc<Frame>),
}

impl Frame {
    /// Create a new, empty frame.
    pub fn new(size: Size, baseline: Length) -> Self {
        assert!(size.is_finite());
        Self { size, baseline, children: vec![] }
    }

    /// Add an element at a position in the foreground.
    pub fn push(&mut self, pos: Point, element: Element) {
        self.children.push((pos, Child::Element(element)));
    }

    /// Add an element at a position in the background.
    pub fn prepend(&mut self, pos: Point, element: Element) {
        self.children.insert(0, (pos, Child::Element(element)))
    }

    /// Add a frame element.
    pub fn push_frame(&mut self, pos: Point, subframe: Rc<Self>) {
        self.children.push((pos, Child::Frame(subframe)))
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
            Some((pos, Child::Frame(f))) => {
                let new_offset = *offset + *pos;
                self.stack.push((0, new_offset, f.as_ref()));
                self.next()
            }
            Some((pos, Child::Element(e))) => {
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
}

/// A run of shaped text.
#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
pub struct Text {
    /// The font face the glyphs are contained in.
    pub face_id: FaceId,
    /// The font size.
    pub size: Length,
    /// Glyph color.
    pub fill: Paint,
    /// The glyphs.
    pub glyphs: Vec<Glyph>,
}

impl Text {
    /// Encode the glyph ids into a big-endian byte buffer.
    pub fn encode_glyphs_be(&self) -> Vec<u8> {
        let mut bytes = Vec::with_capacity(2 * self.glyphs.len());
        for glyph in &self.glyphs {
            bytes.push((glyph.id >> 8) as u8);
            bytes.push((glyph.id & 0xff) as u8);
        }
        bytes
    }
}

/// A glyph in a run of shaped text.
#[derive(Debug, Copy, Clone, Eq, PartialEq, Serialize, Deserialize)]
pub struct Glyph {
    /// The glyph's index in the face.
    pub id: u16,
    /// The advance width of the glyph.
    pub x_advance: Length,
    /// The horizontal offset of the glyph.
    pub x_offset: Length,
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
