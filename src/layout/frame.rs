use fontdock::FaceId;
use ttf_parser::GlyphId;

use crate::color::Color;
use crate::env::ResourceId;
use crate::geom::{Length, Path, Point, Size};

/// A finished layout with elements at fixed positions.
#[derive(Debug, Clone, PartialEq)]
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
    pub fn new(size: Size, baseline: Length) -> Self {
        Self { size, baseline, elements: vec![] }
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
    Text(ShapedText),
    /// A geometric shape.
    Geometry(Geometry),
    /// A raster image.
    Image(Image),
}

/// A shaped run of text.
#[derive(Debug, Clone, PartialEq)]
pub struct ShapedText {
    /// The font face the text was shaped with.
    pub face: FaceId,
    /// The font size.
    pub size: Length,
    /// The width.
    pub width: Length,
    /// The extent to the top.
    pub top: Length,
    /// The extent to the bottom.
    pub bottom: Length,
    /// The glyph fill color / texture.
    pub color: Fill,
    /// The shaped glyphs.
    pub glyphs: Vec<GlyphId>,
    /// The horizontal offsets of the glyphs. This is indexed parallel to
    /// `glyphs`. Vertical offsets are not yet supported.
    pub offsets: Vec<Length>,
}

impl ShapedText {
    /// Create a new shape run with `width` zero and empty `glyphs` and `offsets`.
    pub fn new(
        face: FaceId,
        size: Length,
        top: Length,
        bottom: Length,
        color: Fill,
    ) -> Self {
        Self {
            face,
            size,
            width: Length::ZERO,
            top,
            bottom,
            glyphs: vec![],
            offsets: vec![],
            color,
        }
    }

    /// Encode the glyph ids into a big-endian byte buffer.
    pub fn encode_glyphs_be(&self) -> Vec<u8> {
        let mut bytes = Vec::with_capacity(2 * self.glyphs.len());
        for &GlyphId(g) in &self.glyphs {
            bytes.push((g >> 8) as u8);
            bytes.push((g & 0xff) as u8);
        }
        bytes
    }
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
    /// A rectangle with its origin in the topleft corner.
    Rect(Size),
    /// An ellipse with its origin in the center.
    Ellipse(Size),
    /// A bezier path.
    Path(Path),
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
