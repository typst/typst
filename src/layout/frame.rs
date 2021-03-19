use super::Shaped;
use crate::color::Color;
use crate::env::ResourceId;
use crate::geom::{Point, Size};

/// A finished layout with elements at fixed positions.
#[derive(Debug, Clone, PartialEq)]
pub struct Frame {
    /// The size of the frame.
    pub size: Size,
    /// The elements composing this layout.
    pub elements: Vec<(Point, Element)>,
}

impl Frame {
    /// Create a new, empty frame.
    pub fn new(size: Size) -> Self {
        Self { size, elements: vec![] }
    }

    /// Add an element at a position.
    pub fn push(&mut self, pos: Point, element: Element) {
        self.elements.push((pos, element));
    }

    /// Add all elements of another frame, placing them relative to the given
    /// position.
    pub fn push_frame(&mut self, pos: Point, subframe: Self) {
        for (subpos, element) in subframe.elements {
            self.push(pos + subpos, element);
        }
    }
}

/// The building block frames are composed of.
#[derive(Debug, Clone, PartialEq)]
pub enum Element {
    /// Shaped text.
    Text(Shaped),
    /// A geometric shape.
    Geometry(Geometry),
    /// A raster image.
    Image(Image),
}

/// A shape with some kind of fill.
#[derive(Debug, Clone, PartialEq)]
pub struct Geometry {
    /// The shape to draw.
    pub shape: Shape,
    /// How the shape looks on the inside.
    //
    // TODO: This could be made into a Vec<Fill> or something such that
    //       the user can compose multiple fills with alpha values less
    //       than one to achieve cool effects.
    pub fill: Fill,
}

/// Some shape.
#[derive(Debug, Clone, PartialEq)]
pub enum Shape {
    /// A rectangle.
    Rect(Size),
}

/// The kind of graphic fill to be applied to a [`Shape`].
#[derive(Debug, Copy, Clone, PartialEq)]
pub enum Fill {
    /// The fill is a color.
    Color(Color),
    /// The fill is an image.
    Image(Image),
}

/// An image element.
#[derive(Debug, Copy, Clone, PartialEq)]
pub struct Image {
    /// The image resource.
    pub res: ResourceId,
    /// The size of the image in the document.
    pub size: Size,
}
