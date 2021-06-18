use std::rc::Rc;

use serde::{Deserialize, Serialize};

use super::{Constrained, Constraints};
use crate::color::Color;
use crate::font::FaceId;
use crate::geom::{Length, Path, Point, Size};
use crate::image::ImageId;

/// A finished layout with elements at fixed positions.
#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Frame {
    /// The size of the frame.
    pub size: Size,
    /// The baseline of the frame measured from the top.
    pub baseline: Length,
    /// The elements composing this layout.
    structure: Vec<(Point, Structure)>,
}

/// An iterator over all elements in a frame.
#[derive(Debug, Clone)]
pub struct ElementIter<'a> {
    stack: Vec<(usize, Point, &'a Frame)>,
}

impl<'a> Iterator for ElementIter<'a> {
    type Item = (Point, &'a Element);

    /// Get the next element, if any.
    fn next(&mut self) -> Option<Self::Item> {
        if let Some((cursor, offset, frame)) = self.stack.pop() {
            if cursor < frame.structure.len() {
                match &frame.structure[cursor] {
                    (pos, Structure::Frame(f)) => {
                        self.stack.push((cursor, offset, frame));
                        self.stack.push((0, offset + *pos, f.as_ref()));
                        self.next()
                    }
                    (pos, Structure::Element(e)) => {
                        self.stack.push((cursor + 1, offset, frame));
                        Some((*pos + offset, e))
                    }
                }
            } else {
                if let Some((c, o, f)) = self.stack.pop() {
                    self.stack.push((c + 1, o, f));
                }
                self.next()
            }
        } else {
            None
        }
    }
}

impl Frame {
    /// Create a new, empty frame.
    pub fn new(size: Size, baseline: Length) -> Self {
        assert!(size.is_finite());
        Self { size, baseline, structure: vec![] }
    }

    /// Add an element at a position.
    pub fn push(&mut self, pos: Point, element: Element) {
        self.structure.push((pos, Structure::Element(element)));
    }

    /// Add a frame element.
    pub fn push_frame(&mut self, pos: Point, subframe: Rc<Self>) {
        self.structure.push((pos, Structure::Frame(subframe)))
    }

    /// Add all elements of another frame, placing them relative to the given
    /// position.
    pub fn merge_frame(&mut self, pos: Point, subframe: Self) {
        if pos == Point::zero() && self.structure.is_empty() {
            self.structure = subframe.structure;
        } else {
            for (subpos, structure) in subframe.structure {
                self.structure.push((pos + subpos, structure));
            }
        }
    }

    /// Wraps the frame with constraints.
    pub fn constrain(self, constraints: Constraints) -> Constrained<Rc<Self>> {
        Constrained { item: Rc::new(self), constraints }
    }

    /// Returns an iterator over all elements in the frame and its structure.
    pub fn elements(&self) -> ElementIter {
        ElementIter { stack: vec![(0, Point::zero(), self)] }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
enum Structure {
    Element(Element),
    /// Another frame.
    Frame(Rc<Frame>),
}

/// The building block frames are composed of.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Element {
    /// Shaped text.
    Text(Text),
    /// A filled geometric shape.
    Geometry(Shape, Fill),
    /// A raster image.
    Image(ImageId, Size),
}

/// A run of shaped text.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Text {
    /// The font face the glyphs are contained in.
    pub face_id: FaceId,
    /// The font size.
    pub size: Length,
    /// The glyph's fill color.
    pub fill: Fill,
    /// The glyphs.
    pub glyphs: Vec<Glyph>,
}

/// A glyph in a run of shaped text.
#[derive(Debug, Copy, Clone, PartialEq, Serialize, Deserialize)]
pub struct Glyph {
    /// The glyph's index in the face.
    pub id: u16,
    /// The advance width of the glyph.
    pub x_advance: Length,
    /// The horizontal offset of the glyph.
    pub x_offset: Length,
}

impl Text {
    /// Encode the glyph ids into a big-endian byte buffer.
    pub fn encode_glyphs_be(&self) -> Vec<u8> {
        let mut bytes = Vec::with_capacity(2 * self.glyphs.len());
        for glyph in &self.glyphs {
            let id = glyph.id;
            bytes.push((id >> 8) as u8);
            bytes.push((id & 0xff) as u8);
        }
        bytes
    }
}

/// A geometric shape.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Shape {
    /// A rectangle with its origin in the topleft corner.
    Rect(Size),
    /// An ellipse with its origin in the center.
    Ellipse(Size),
    /// A line to a `Point` (relative to its position) with a stroke width.
    Line(Point, Length),
    /// A bezier path.
    Path(Path),
}

/// How text and shapes are filled.
#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash, Serialize, Deserialize)]
pub enum Fill {
    /// A solid color.
    Color(Color),
}
