//! Representation of typesetted documents.

use crate::font::Font;
use crate::engine::Size;


/// A complete typesetted document, which can be exported.
#[derive(Debug, Clone, PartialEq)]
pub struct Document {
    /// The pages of the document.
    pub pages: Vec<Page>,
    /// The fonts used in the document.
    pub fonts: Vec<Font>,
}

/// A page with text contents in a document.
#[derive(Debug, Clone, PartialEq)]
pub struct Page {
    /// The width of the page.
    pub width: Size,
    /// The height of the page.
    pub height: Size,
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
