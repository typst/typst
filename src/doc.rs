//! Representation of typesetted documents.

use std::io::{self, Write};
use crate::size::{Size, Size2D};


/// A complete typesetted document, which can be exported.
#[derive(Debug, Clone)]
pub struct Document {
    /// The pages of the document.
    pub pages: Vec<Page>,
}

/// A page of a document.
#[derive(Debug, Clone)]
pub struct Page {
    /// The width of the page.
    pub width: Size,
    /// The height of the page.
    pub height: Size,
    /// Layouting actions specifying how to draw content on the page.
    pub actions: Vec<LayoutAction>,
}

/// A layouting action.
#[derive(Debug, Clone)]
pub enum LayoutAction {
    /// Move to an absolute position.
    MoveAbsolute(Size2D),
    /// Set the font by index and font size.
    SetFont(usize, f32),
    /// Write text starting at the current position.
    WriteText(String),
}

impl LayoutAction {
    /// Serialize this layout action into a string representation.
    pub fn serialize<W: Write>(&self, f: &mut W) -> io::Result<()> {
        use LayoutAction::*;
        match self {
            MoveAbsolute(s) => write!(f, "m {:.4} {:.4}", s.x.to_pt(), s.y.to_pt()),
            SetFont(i, s) => write!(f, "f {} {}", i, s),
            WriteText(s) => write!(f, "w {}", s),
        }
    }
}
