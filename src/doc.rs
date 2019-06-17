//! Representation of typesetted documents.

use crate::font::Font;
use crate::size::{Size, Size2D};


/// A complete typesetted document, which can be exported.
#[derive(Debug, Clone)]
pub struct Document {
    /// The pages of the document.
    pub pages: Vec<Page>,
    /// The fonts used (the page contents index into this).
    pub fonts: Vec<Font>,
}

/// A page of a document.
#[derive(Debug, Clone)]
pub struct Page {
    /// The width of the page.
    pub width: Size,
    /// The height of the page.
    pub height: Size,
    /// Text actions specifying how to draw text content on the page.
    pub actions: Vec<TextAction>,
}

/// A text layouting action.
#[derive(Debug, Clone)]
pub enum TextAction {
    /// Move to an absolute position.
    MoveAbsolute(Size2D),
    /// Move from the _start_ of the current line by an (x, y) offset.
    MoveNewline(Size2D),
    /// Write text starting at the current position.
    WriteText(String),
    /// Set the font by index and font size.
    SetFont(usize, f32),
}
