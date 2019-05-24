//! Layouting engine.

use crate::doc::Document;
use crate::font::{Font, FontLoader, FontFamily, FontError};
use crate::syntax::SyntaxTree;

mod size;
pub use size::Size;


/// Layout a syntax tree given a context.
#[allow(unused_variables)]
pub fn layout(tree: &SyntaxTree, ctx: &LayoutContext) -> LayoutResult<Layout> {
    Ok(Layout {})
}

/// A collection of layouted content.
pub struct Layout {}

impl Layout {
    /// Convert this layout into a document given the list of fonts referenced by it.
    pub fn into_document(self, fonts: Vec<Font>) -> Document {
        Document {
            pages: vec![],
            fonts,
        }
    }
}

/// The context for layouting.
pub struct LayoutContext<'a, 'p> {
    pub loader: &'a FontLoader<'p>,
}

/// Default styles for pages.
#[derive(Debug, Clone, PartialEq)]
pub struct PageStyle {
    /// The width of the paper.
    pub width: Size,
    /// The height of the paper.
    pub height: Size,

    /// The left margin of the paper.
    pub margin_left: Size,
    /// The top margin of the paper.
    pub margin_top: Size,
    /// The right margin of the paper.
    pub margin_right: Size,
    /// The bottom margin of the paper.
    pub margin_bottom: Size,
}

impl Default for PageStyle {
    fn default() -> PageStyle {
        PageStyle {
            // A4 paper.
            width: Size::from_mm(210.0),
            height: Size::from_mm(297.0),

            // Margins. A bit more on top and bottom.
            margin_left: Size::from_cm(3.0),
            margin_top: Size::from_cm(3.0),
            margin_right: Size::from_cm(3.0),
            margin_bottom: Size::from_cm(3.0),
        }
    }
}

/// Default styles for texts.
#[derive(Debug, Clone, PartialEq)]
pub struct TextStyle {
    /// A fallback list of font families to use.
    pub font_families: Vec<FontFamily>,
    /// The font size.
    pub font_size: f32,
    /// The line spacing (as a multiple of the font size).
    pub line_spacing: f32,
    /// The spacing for paragraphs (as a multiple of the line spacing).
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

/// The error type for layouting.
pub enum LayoutError {
    /// There was no suitable font.
    MissingFont,
    /// An error occured while gathering font data.
    Font(FontError),
}

/// The result type for layouting.
pub type LayoutResult<T> = Result<T, LayoutError>;

error_type! {
    err: LayoutError,
    show: f => match err {
        LayoutError::MissingFont => write!(f, "missing font"),
        LayoutError::Font(err) => write!(f, "font error: {}", err),
    },
    source: match err {
        LayoutError::Font(err) => Some(err),
        _ => None,
    },
    from: (std::io::Error, LayoutError::Font(FontError::Io(err))),
    from: (FontError, LayoutError::Font(err)),
}
