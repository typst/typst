//! Styles for layouting.

use crate::font::FontFamily;
use crate::size::{Size, Size2D, SizeBox};


/// Default styles for text.
#[derive(Debug, Clone)]
pub struct TextStyle {
    /// A fallback list of font families to use.
    pub font_families: Vec<FontFamily>,
    /// The font size.
    pub font_size: f32,
    /// The line spacing (as a multiple of the font size).
    pub line_spacing: f32,
    /// The paragraphs spacing (as a multiple of the line spacing).
    pub paragraph_spacing: f32,
}

impl Default for TextStyle {
    fn default() -> TextStyle {
        use FontFamily::*;
        TextStyle {
            // Default font family, font size and line spacing.
            font_families: vec![SansSerif, Serif, Monospace],
            font_size: 11.0,
            line_spacing: 1.25,
            paragraph_spacing: 1.5,
        }
    }
}

/// Default styles for pages.
#[derive(Debug, Clone)]
pub struct PageStyle {
    /// Width and height of the page.
    pub dimensions: Size2D,
    /// The amount of white space on each side.
    pub margins: SizeBox,
}

impl Default for PageStyle {
    fn default() -> PageStyle {
        PageStyle {
            // A4 paper.
            dimensions: Size2D {
                x: Size::from_mm(210.0),
                y: Size::from_mm(297.0),
            },

            // All the same margins.
            margins: SizeBox {
                left: Size::from_cm(3.0),
                top: Size::from_cm(3.0),
                right: Size::from_cm(3.0),
                bottom: Size::from_cm(3.0),
            },
        }
    }
}
