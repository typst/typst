//! The layouting engine.

use crate::doc::{Document, Page, TextAction};
use crate::font::{Font, FontLoader, FontFamily, FontError};
use crate::syntax::{SyntaxTree, Node};

mod size;
mod text;

pub use size::Size;
pub use text::TextLayouter;


/// Layout a syntax tree in a given context.
pub fn layout(tree: &SyntaxTree, ctx: &LayoutContext) -> LayoutResult<Layout> {
    let mut layouter = TextLayouter::new(ctx);

    let mut italic = false;
    let mut bold = false;

    for node in &tree.nodes {
        match node {
            Node::Text(text) => layouter.add_text(text)?,
            Node::Space => layouter.add_space()?,
            Node::Newline => layouter.add_paragraph()?,

            Node::ToggleItalics => {
                italic = !italic;
                layouter.set_italic(italic);
            },
            Node::ToggleBold => {
                bold = !bold;
                layouter.set_bold(bold);
            }

            Node::Func(_) => unimplemented!(),
        }
    }

    layouter.finish()
}

/// A collection of layouted content.
#[derive(Debug, Clone)]
pub struct Layout {
    /// The extent of this layout into all directions.
    extent: LayoutDimensions,
    /// Actions composing this layout.
    actions: Vec<TextAction>,
}

impl Layout {
    /// Convert this layout into a document given the list of fonts referenced by it.
    pub fn into_document(self, fonts: Vec<Font>) -> Document {
        Document {
            pages: vec![Page {
                width: self.extent.width,
                height: self.extent.height,
                actions: self.actions,
            }],
            fonts,
        }
    }
}

/// Types supporting some kind of layouting.
pub trait Layouter {
    /// Finishing the current layouting process and return a layout.
    fn finish(self) -> LayoutResult<Layout>;
}

/// The context for layouting.
#[derive(Debug, Clone)]
pub struct LayoutContext<'a, 'p> {
    /// Loads fonts matching queries.
    pub loader: &'a FontLoader<'p>,
    /// The spacial constraints to layout in.
    pub max_extent: LayoutDimensions,
    /// Base style to set text with.
    pub text_style: TextStyle,
}

/// A space to layout in.
#[derive(Debug, Clone)]
pub struct LayoutDimensions {
    /// Horizontal extent.
    pub width: Size,
    /// Vertical extent.
    pub height: Size,
}

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
    /// The width of the page.
    pub width: Size,
    /// The height of the page.
    pub height: Size,

    /// The amount of white space on the left side.
    pub margin_left: Size,
    /// The amount of white space on the top side.
    pub margin_top: Size,
    /// The amount of white space on the right side.
    pub margin_right: Size,
    /// The amount of white space on the bottom side.
    pub margin_bottom: Size,
}

impl Default for PageStyle {
    fn default() -> PageStyle {
        PageStyle {
            // A4 paper.
            width: Size::from_mm(210.0),
            height: Size::from_mm(297.0),

            // All the same margins.
            margin_left: Size::from_cm(3.0),
            margin_top: Size::from_cm(3.0),
            margin_right: Size::from_cm(3.0),
            margin_bottom: Size::from_cm(3.0),
        }
    }
}

/// The error type for layouting.
pub enum LayoutError {
    /// There was no suitable font.
    NoSuitableFont,
    /// An error occured while gathering font data.
    Font(FontError),
}

/// The result type for layouting.
pub type LayoutResult<T> = Result<T, LayoutError>;

error_type! {
    err: LayoutError,
    show: f => match err {
        LayoutError::NoSuitableFont => write!(f, "no suitable font"),
        LayoutError::Font(err) => write!(f, "font error: {}", err),
    },
    source: match err {
        LayoutError::Font(err) => Some(err),
        _ => None,
    },
    from: (std::io::Error, LayoutError::Font(FontError::Io(err))),
    from: (FontError, LayoutError::Font(err)),
}
