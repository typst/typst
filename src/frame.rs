//! Finished layouts.

use std::fmt::{self, Debug, Formatter};
use std::sync::Arc;

use crate::font::FaceId;
use crate::geom::{Align, Em, Length, Paint, Path, Point, Size, Spec, Transform};
use crate::image::ImageId;

/// A finished layout with elements at fixed positions.
#[derive(Default, Clone, Eq, PartialEq)]
pub struct Frame {
    /// The size of the frame.
    pub size: Size,
    /// The baseline of the frame measured from the top. If this is `None`, the
    /// frame's implicit baseline is at the bottom.
    pub baseline: Option<Length>,
    /// The elements composing this layout.
    pub elements: Vec<(Point, Element)>,
}

impl Frame {
    /// Create a new, empty frame.
    #[track_caller]
    pub fn new(size: Size) -> Self {
        assert!(size.is_finite());
        Self { size, baseline: None, elements: vec![] }
    }

    /// The baseline of the frame.
    pub fn baseline(&self) -> Length {
        self.baseline.unwrap_or(self.size.y)
    }

    /// Add an element at a position in the background.
    pub fn prepend(&mut self, pos: Point, element: Element) {
        self.elements.insert(0, (pos, element));
    }

    /// Add an element at a position in the foreground.
    pub fn push(&mut self, pos: Point, element: Element) {
        self.elements.push((pos, element));
    }

    /// The layer the next item will be added on. This corresponds to the number
    /// of elements in the frame.
    pub fn layer(&self) -> usize {
        self.elements.len()
    }

    /// Insert an element at the given layer in the frame.
    ///
    /// This panics if the layer is greater than the number of layers present.
    pub fn insert(&mut self, layer: usize, pos: Point, element: Element) {
        self.elements.insert(layer, (pos, element));
    }

    /// Add a group element.
    pub fn push_frame(&mut self, pos: Point, frame: Arc<Self>) {
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

    /// Resize the frame to a new size, distributing new space according to the
    /// given alignments.
    pub fn resize(&mut self, target: Size, aligns: Spec<Align>) {
        if self.size != target {
            let offset = Point::new(
                aligns.x.resolve(target.x - self.size.x),
                aligns.y.resolve(target.y - self.size.y),
            );
            self.size = target;
            self.translate(offset);
        }
    }

    /// Move the baseline and contents of the frame by an offset.
    pub fn translate(&mut self, offset: Point) {
        if !offset.is_zero() {
            if let Some(baseline) = &mut self.baseline {
                *baseline += offset.y;
            }
            for (point, _) in &mut self.elements {
                *point += offset;
            }
        }
    }

    /// Arbitrarily transform the contents of the frame.
    pub fn transform(&mut self, transform: Transform) {
        self.group(|g| g.transform = transform);
    }

    /// Clip the contents of a frame to its size.
    pub fn clip(&mut self) {
        self.group(|g| g.clips = true);
    }

    /// Wrap the frame's contents in a group and modify that group with `f`.
    pub fn group<F>(&mut self, f: F)
    where
        F: FnOnce(&mut Group),
    {
        let mut wrapper = Frame { elements: vec![], ..*self };
        let mut group = Group::new(Arc::new(std::mem::take(self)));
        f(&mut group);
        wrapper.push(Point::zero(), Element::Group(group));
        *self = wrapper;
    }

    /// Link the whole frame to a resource.
    pub fn link(&mut self, url: impl Into<String>) {
        self.push(Point::zero(), Element::Link(url.into(), self.size));
    }
}

impl Debug for Frame {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        f.debug_struct("Frame")
            .field("size", &self.size)
            .field("baseline", &self.baseline)
            .field(
                "children",
                &crate::util::debug(|f| {
                    f.debug_map()
                        .entries(self.elements.iter().map(|(k, v)| (k, v)))
                        .finish()
                }),
            )
            .finish()
    }
}

/// The building block frames are composed of.
#[derive(Debug, Clone, Eq, PartialEq)]
pub enum Element {
    /// A group of elements.
    Group(Group),
    /// A run of shaped text.
    Text(Text),
    /// A geometric shape with optional fill and stroke.
    Shape(Shape),
    /// An image and its size.
    Image(ImageId, Size),
    /// A link to an external resource and its trigger region.
    Link(String, Size),
}

/// A group of elements with optional clipping.
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct Group {
    /// The group's frame.
    pub frame: Arc<Frame>,
    /// A transformation to apply to the group.
    pub transform: Transform,
    /// Whether the frame should be a clipping boundary.
    pub clips: bool,
}

impl Group {
    /// Create a new group with default settings.
    pub fn new(frame: Arc<Frame>) -> Self {
        Self {
            frame,
            transform: Transform::identity(),
            clips: false,
        }
    }
}

/// A run of shaped text.
#[derive(Debug, Clone, Eq, PartialEq)]
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
    /// The width of the text run.
    pub fn width(&self) -> Length {
        self.glyphs.iter().map(|g| g.x_advance.resolve(self.size)).sum()
    }
}

/// A glyph in a run of shaped text.
#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub struct Glyph {
    /// The glyph's index in the face.
    pub id: u16,
    /// The advance width of the glyph.
    pub x_advance: Em,
    /// The horizontal offset of the glyph.
    pub x_offset: Em,
}

/// A geometric shape with optional fill and stroke.
#[derive(Debug, Clone, Eq, PartialEq)]
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
#[derive(Debug, Clone, Eq, PartialEq)]
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
#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
pub struct Stroke {
    /// The stroke's paint.
    pub paint: Paint,
    /// The stroke's thickness.
    pub thickness: Length,
}
