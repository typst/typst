//! Core typesetting engine.

use std::cell::Ref;
use std::mem::swap;

use smallvec::SmallVec;

use crate::doc::{Document, Page, Text, TextCommand};
use crate::font::{Font, FontFamily, FontProvider, FontError};
use crate::syntax::{SyntaxTree, Node};
use loader::{FontLoader, FontQuery};

mod size;
mod loader;
pub use size::Size;


/// Typeset a parsed syntax tree.
pub fn typeset<'p>(tree: &SyntaxTree, style: &Style, font_providers: &[Box<dyn FontProvider + 'p>])
    -> TypesetResult<Document> {
    Engine::new(tree, style, font_providers).typeset()
}


/// The core typesetting engine, transforming an abstract syntax tree into a document.
struct Engine<'a> {
    // Input
    tree: &'a SyntaxTree,
    style: &'a Style,

    // Internal
    font_loader: FontLoader<'a>,

    // Output
    text_commands: Vec<TextCommand>,

    // Intermediates
    active_font: usize,
    current_text: String,
    current_line_width: Size,
    current_max_vertical_move: Size,
    bold: bool,
    italic: bool,
}

impl<'a> Engine<'a> {
    /// Create a new generator from a syntax tree.
    fn new(
        tree: &'a SyntaxTree,
        style: &'a Style,
        font_providers: &'a [Box<dyn FontProvider + 'a>]
    ) -> Engine<'a> {
        Engine {
            tree,
            style,
            font_loader: FontLoader::new(font_providers),
            text_commands: vec![],
            active_font: std::usize::MAX,
            current_text: String::new(),
            current_line_width: Size::zero(),
            current_max_vertical_move: Size::zero(),
            italic: false,
            bold: false,
        }
    }

    /// Generate the abstract document.
    fn typeset(mut self) -> TypesetResult<Document> {
        // Start by moving to a suitable position.
        self.move_start();

        // Iterate through the documents nodes.
        for node in &self.tree.nodes {
            match node {
                Node::Text(text) => self.write_word(text)?,
                Node::Space => self.write_space()?,
                Node::Newline => {
                    self.write_buffered_text();
                    self.move_newline(self.style.paragraph_spacing);
                },

                Node::ToggleItalics => self.italic = !self.italic,
                Node::ToggleBold => self.bold = !self.bold,

                Node::ToggleMath => unimplemented!(),
                Node::Func(_) => unimplemented!(),
            }
        }

        // Flush the text buffer.
        self.write_buffered_text();

        // Create a document with one page from the contents.
        Ok(Document {
            pages: vec![Page {
                width: self.style.width,
                height: self.style.height,
                text: vec![Text {
                    commands: self.text_commands,
                }],
            }],
            fonts: self.font_loader.into_fonts(),
        })
    }

    /// Write a word.
    fn write_word(&mut self, word: &str) -> TypesetResult<()> {
        // Contains pairs of (characters, font_index, char_width).
        let mut chars_with_widths = SmallVec::<[(char, usize, Size); 12]>::new();

        // Find out which font to use for each character in the word and meanwhile
        // calculate the width of the word.
        let mut word_width = Size::zero();
        for c in word.chars() {
            let (index, font) = self.get_font_for(c)?;
            let width = self.char_width(c, &font);
            word_width += width;
            chars_with_widths.push((c, index, width));
        }

        // If this would overflow, we move to a new line and finally write the previous one.
        if self.would_overflow(word_width) {
            self.write_buffered_text();
            self.move_newline(1.0);
        }

        // Finally write the word.
        for (c, index, width) in chars_with_widths {
            if index != self.active_font {
                // If we will change the font, first write the remaining things.
                self.write_buffered_text();
                self.set_font(index);
            }

            self.current_text.push(c);
            self.current_line_width += width;
        }

        Ok(())
    }

    /// Write the space character: `' '`.
    fn write_space(&mut self) -> TypesetResult<()> {
        let space_width = self.char_width(' ', &self.get_font_for(' ')?.1);
        if !self.would_overflow(space_width) && self.current_line_width > Size::zero() {
            self.write_word(" ")?;
        }

        Ok(())
    }

    /// Write a text command with the buffered text.
    fn write_buffered_text(&mut self) {
        if !self.current_text.is_empty() {
            let mut current_text = String::new();
            swap(&mut self.current_text, &mut current_text);
            self.text_commands.push(TextCommand::Text(current_text));
        }
    }

    /// Move to the starting position defined by the style.
    fn move_start(&mut self) {
        // Move cursor to top-left position
        self.text_commands.push(TextCommand::Move(
            self.style.margin_left,
            self.style.height - self.style.margin_top
        ));
    }

    /// Move to a new line.
    fn move_newline(&mut self, factor: f32) {
        if self.active_font == std::usize::MAX {
            return;
        }

        let vertical_move = if self.current_max_vertical_move == Size::zero() {
            // If max vertical move is still zero, the line is empty and we take the
            // font size from the previous line.
            self.style.font_size
                * self.style.line_spacing
                * self.get_font_at(self.active_font).metrics.ascender
                * factor
        } else {
            self.current_max_vertical_move
        };

        self.text_commands.push(TextCommand::Move(Size::zero(), -vertical_move));
        self.current_max_vertical_move = Size::zero();
        self.current_line_width = Size::zero();
    }

    /// Set the current font.
    fn set_font(&mut self, index: usize) {
        self.text_commands.push(TextCommand::SetFont(index, self.style.font_size));
        self.active_font = index;
    }

    /// Whether the current line plus the extra `width` would overflow the line.
    fn would_overflow(&self, width: Size) -> bool {
        let max_width = self.style.width
            - self.style.margin_left - self.style.margin_right;
        self.current_line_width + width > max_width
    }

    /// Load a font that has the character we need.
    fn get_font_for(&self, character: char) -> TypesetResult<(usize, Ref<Font>)> {
        self.font_loader.get(FontQuery {
            families: &self.style.font_families,
            italic: self.italic,
            bold: self.bold,
            character,
        }).ok_or_else(|| TypesetError::MissingFont)
    }

    /// Load a font at an index.
    fn get_font_at(&self, index: usize) -> Ref<Font> {
        self.font_loader.get_with_index(index)
    }

    /// The width of a char in a specific font.
    fn char_width(&self, character: char, font: &Font) -> Size {
        font.widths[font.map(character) as usize] * self.style.font_size
    }
}

/// The context for typesetting a function.
#[derive(Debug)]
pub struct TypesetContext {}

/// Default styles for typesetting.
#[derive(Debug, Clone, PartialEq)]
pub struct Style {
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

    /// A fallback list of font families to use.
    pub font_families: Vec<FontFamily>,
    /// The font size.
    pub font_size: f32,
    /// The line spacing (as a multiple of the font size).
    pub line_spacing: f32,
    /// The spacing for paragraphs (as a multiple of the line spacing).
    pub paragraph_spacing: f32,
}

impl Default for Style {
    fn default() -> Style {
        use FontFamily::*;
        Style {
            // A4 paper.
            width: Size::from_mm(210.0),
            height: Size::from_mm(297.0),

            // Margins. A bit more on top and bottom.
            margin_left: Size::from_cm(3.0),
            margin_top: Size::from_cm(3.0),
            margin_right: Size::from_cm(3.0),
            margin_bottom: Size::from_cm(3.0),

            // Default font family, font size and line spacing.
            font_families: vec![SansSerif, Serif, Monospace],
            font_size: 11.0,
            line_spacing: 1.25,
            paragraph_spacing: 1.5,
        }
    }
}

/// The error type for typesetting.
pub enum TypesetError {
    /// There was no suitable font.
    MissingFont,
    /// An error occured while gathering font data.
    Font(FontError),
}

/// The result type for typesetting.
pub type TypesetResult<T> = Result<T, TypesetError>;

error_type! {
    err: TypesetError,
    show: f => match err {
        TypesetError::MissingFont => write!(f, "missing font"),
        TypesetError::Font(err) => write!(f, "font error: {}", err),
    },
    source: match err {
        TypesetError::Font(err) => Some(err),
        _ => None,
    },
    from: (std::io::Error, TypesetError::Font(FontError::Io(err))),
    from: (FontError, TypesetError::Font(err)),
}
