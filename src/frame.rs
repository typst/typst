//! Finished layouts.

use std::fmt::{self, Debug, Formatter};
use std::rc::Rc;

use serde::{Deserialize, Serialize};

use crate::font::FaceId;
use crate::geom::{Em, Length, Paint, Path, Point, Size, Transform};
use crate::image::ImageId;

/// A finished layout with elements at fixed positions.
#[derive(Default, Clone, Eq, PartialEq, Serialize, Deserialize)]
pub struct Frame {
    /// The size of the frame.
    pub size: Size,
    /// The baseline of the frame measured from the top.
    pub baseline: Length,
    /// The elements composing this layout.
    pub elements: Vec<(Point, Element)>,
}

impl Frame {
    /// Create a new, empty frame.
    #[track_caller]
    pub fn new(size: Size, baseline: Length) -> Self {
        assert!(size.is_finite());
        Self { size, baseline, elements: vec![] }
    }

    /// Add an element at a position in the background.
    pub fn prepend(&mut self, pos: Point, element: Element) {
        self.elements.insert(0, (pos, element));
    }

    /// Add an element at a position in the foreground.
    pub fn push(&mut self, pos: Point, element: Element) {
        self.elements.push((pos, element));
    }

    /// Add a group element.
    pub fn push_frame(&mut self, pos: Point, frame: Rc<Self>) {
        self.elements.push((pos, Element::Group(Group::new(frame))));
    }

    /// Add all elements of another frame, placing them relative to the given
    /// position.
    pub fn merge_frame(&mut self, pos: Point, subframe: Self) {
        if pos == Point::zero() && self.elements.is_empty() {
            self.elements = subframe.elements;
        } else {
            for (subpos, child) in subframe.elements {
                self.elements.push((pos + subpos, child));
            }
        }
    }

    /// Move all elements in the frame by an offset.
    pub fn translate(&mut self, offset: Point) {
        for (point, _) in &mut self.elements {
            *point += offset;
        }
    }
}

impl Debug for Frame {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        struct Children<'a>(&'a [(Point, Element)]);

        impl Debug for Children<'_> {
            fn fmt(&self, f: &mut Formatter) -> fmt::Result {
                f.debug_map().entries(self.0.iter().map(|(k, v)| (k, v))).finish()
            }
        }

        f.debug_struct("Frame")
            .field("size", &self.size)
            .field("baseline", &self.baseline)
            .field("children", &Children(&self.elements))
            .finish()
    }
}

/// The building block frames are composed of.
#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
pub enum Element {
    /// A group of elements.
    Group(Group),
    /// A run of shaped text.
    Text(Text),
    /// A geometric shape with optional fill and stroke.
    Shape(Shape),
    /// A raster image and its size.
    Image(ImageId, Size),
    /// A link to an external resource and its trigger region.
    Link(String, Size),
}

/// A group of elements with optional clipping.
#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
pub struct Group {
    /// The group's frame.
    pub frame: Rc<Frame>,
    /// A transformation to apply to the group.
    pub transform: Transform,
    /// Whether the frame should be a clipping boundary.
    pub clips: bool,
}

impl Group {
    /// Create a new group with default settings.
    pub fn new(frame: Rc<Frame>) -> Self {
        Self {
            frame,
            transform: Transform::identity(),
            clips: false,
        }
    }

    /// Set the group's transform.
    pub fn transform(mut self, transform: Transform) -> Self {
        self.transform = transform;
        self
    }

    /// Set whether the group should be a clipping boundary.
    pub fn clips(mut self, clips: bool) -> Self {
        self.clips = clips;
        self
    }
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

/// A geometric shape with optional fill and stroke.
#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
pub struct Shape {
    /// The shape's geometry.
    pub geometry: Geometry,
    /// The shape's background fill.
    pub fill: Option<Paint>,
    /// The shape's border stroke.
    pub stroke: Option<Stroke>,
}

impl Shape {
    /// Create a filled shape without a stroke.
    pub fn filled(geometry: Geometry, fill: Paint) -> Self {
        Self { geometry, fill: Some(fill), stroke: None }
    }

    /// Create a stroked shape without a fill.
    pub fn stroked(geometry: Geometry, stroke: Stroke) -> Self {
        Self {
            geometry,
            fill: None,
            stroke: Some(stroke),
        }
    }
}

/// A shape's geometry.
#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
pub enum Geometry {
    /// A line to a point (relative to its position).
    Line(Point),
    /// A rectangle with its origin in the topleft corner.
    Rect(Size),
    /// A ellipse with its origin in the topleft corner.
    Ellipse(Size),
    /// A bezier path.
    Path(Path),
}

/// A stroke of a geometric shape.
#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash, Serialize, Deserialize)]
pub struct Stroke {
    /// The stroke's paint.
    pub paint: Paint,
    /// The stroke's thickness.
    pub thickness: Length,
}
