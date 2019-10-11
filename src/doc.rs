//! Representation of typesetted documents.

use crate::layout::LayoutAction;
use crate::size::Size;


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
