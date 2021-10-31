//! Finished layouts.

use std::fmt::{self, Debug, Formatter};
use std::rc::Rc;

use serde::{Deserialize, Serialize};

use crate::font::FaceId;
use crate::geom::{Em, Length, Paint, Path, Point, Size};
use crate::image::ImageId;

/// A finished layout with elements at fixed positions.
#[derive(Default, Clone, Eq, PartialEq, Serialize, Deserialize)]
pub struct Frame {
    /// The size of the frame.
    pub size: Size,
    /// The baseline of the frame measured from the top.
    pub baseline: Length,
    /// The elements composing this layout.
    pub children: Vec<(Point, FrameChild)>,
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
        self.children.push((pos, FrameChild::Group(subframe)))
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

    /// An iterator over all elements in the frame and its children.
    pub fn elements(&self) -> Elements {
        Elements { stack: vec![(0, Point::zero(), self)] }
    }
}

impl Debug for Frame {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        struct Children<'a>(&'a [(Point, FrameChild)]);

        impl Debug for Children<'_> {
            fn fmt(&self, f: &mut Formatter) -> fmt::Result {
                f.debug_map().entries(self.0.iter().map(|(k, v)| (k, v))).finish()
            }
        }

        f.debug_struct("Frame")
            .field("size", &self.size)
            .field("baseline", &self.baseline)
            .field("children", &Children(&self.children))
            .finish()
    }
}

/// A frame can contain two different kinds of children: a leaf element or a
/// nested frame.
#[derive(Clone, Eq, PartialEq, Serialize, Deserialize)]
pub enum FrameChild {
    /// A leaf node in the frame tree.
    Element(Element),
    /// An interior group.
    Group(Rc<Frame>),
}

impl Debug for FrameChild {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        match self {
            Self::Element(element) => element.fmt(f),
            Self::Group(frame) => frame.fmt(f),
        }
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
            Some((pos, FrameChild::Group(f))) => {
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
    /// stroked (which one depends on the kind of geometry).
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
