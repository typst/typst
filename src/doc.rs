//! Abstract representation of a typesetted document.

use std::ops;
use crate::font::Font;


/// A representation of a typesetted document.
#[derive(Debug, Clone, PartialEq)]
pub struct Document {
    /// The pages of the document.
    pub pages: Vec<Page>,
    /// The fonts used in the document.
    pub fonts: Vec<Font>,
}

/// Default styles for a document.
#[derive(Debug, Clone, PartialEq)]
pub struct Style {
    /// The width and height of the paper.
    pub paper_size: [Size; 2],
    /// The [left, top, right, bottom] margins of the paper.
    pub margins: [Size; 4],

    /// A fallback list of font families to use.
    pub font_families: Vec<String>,
    /// The default font size.
    pub font_size: f32,
}

impl Default for Style {
    fn default() -> Style {
        Style {
            // A4 paper with 1.5 cm margins in all directions
            paper_size: [Size::from_mm(210.0), Size::from_mm(297.0)],
            margins: [Size::from_cm(2.5); 4],

            // Default font family
            font_families: (&[
                "NotoSans", "NotoSansMath"
            ]).iter().map(ToString::to_string).collect(),
            font_size: 12.0,
        }
    }
}

/// A page with text contents in a document.
#[derive(Debug, Clone, PartialEq)]
pub struct Page {
    /// The width and height of the page.
    pub size: [Size; 2],
    /// Text content on the page.
    pub text: Vec<Text>,
}

/// A series of text command, that can be written on to a page.
#[derive(Debug, Clone, PartialEq)]
pub struct Text {
    /// The text commands.
    pub commands: Vec<TextCommand>,
}

/// Different commands for rendering text.
#[derive(Debug, Clone, PartialEq)]
pub enum TextCommand {
    /// Writing of the text.
    Text(String),
    /// Moves from the *start* of the current line by an (x,y) offset.
    Move(Size, Size),
    /// Use the indexed font in the documents font list with a given font size.
    SetFont(usize, f32),
}

/// A general distance type that can convert between units.
#[derive(Debug, Copy, Clone, PartialEq)]
pub struct Size {
    /// The size in typographic points (1/72 inches).
    points: f32,
}

impl Size {
    /// Create a size from a number of points.
    #[inline]
    pub fn from_points(points: f32) -> Size { Size { points } }

    /// Create a size from a number of inches.
    #[inline]
    pub fn from_inches(inches: f32) -> Size { Size { points: 72.0 * inches } }

    /// Create a size from a number of millimeters.
    #[inline]
    pub fn from_mm(mm: f32) -> Size { Size { points: 2.83465 * mm  } }

    /// Create a size from a number of centimeters.
    #[inline]
    pub fn from_cm(cm: f32) -> Size { Size { points: 28.3465 * cm } }

    /// Create a size from a number of points.
    #[inline]
    pub fn to_points(&self) -> f32 { self.points }

    /// Create a size from a number of inches.
    #[inline]
    pub fn to_inches(&self) -> f32 { self.points * 0.0138889 }

    /// Create a size from a number of millimeters.
    #[inline]
    pub fn to_mm(&self) -> f32 { self.points * 0.352778 }

    /// Create a size from a number of centimeters.
    #[inline]
    pub fn to_cm(&self) -> f32 { self.points * 0.0352778 }
}

impl ops::Add for Size {
    type Output = Size;

    fn add(self, other: Size) -> Size {
        Size { points: self.points + other.points }
    }
}

impl ops::Sub for Size {
    type Output = Size;

    fn sub(self, other: Size) -> Size {
        Size { points: self.points - other.points }
    }
}
