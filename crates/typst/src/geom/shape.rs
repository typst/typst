use super::*;

/// A geometric shape with optional fill and stroke.
#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub struct Shape {
    /// The shape's geometry.
    pub geometry: Geometry,
    /// The shape's background fill.
    pub fill: Option<Paint>,
    /// The shape's border stroke.
    pub stroke: Option<FixedStroke>,
}

/// A shape's geometry.
#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub enum Geometry {
    /// A line to a point (relative to its position).
    Line(Point),
    /// A rectangle with its origin in the topleft corner.
    Rect(Size),
    /// A bezier path.
    Path(Path),
}

impl Geometry {
    /// Fill the geometry without a stroke.
    pub fn filled(self, fill: Paint) -> Shape {
        Shape { geometry: self, fill: Some(fill), stroke: None }
    }

    /// Stroke the geometry without a fill.
    pub fn stroked(self, stroke: FixedStroke) -> Shape {
        Shape { geometry: self, fill: None, stroke: Some(stroke) }
    }

    /// The bounding box of the geometry.
    pub fn bbox_size(&self) -> Size {
        match self {
            Self::Line(line) => Size::new(line.x, line.y),
            Self::Rect(s) => *s,
            Self::Path(p) => p.bbox_size(),
        }
    }
}
