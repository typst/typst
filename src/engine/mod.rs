//! Core typesetting engine.

use crate::syntax::{SyntaxTree, Node};
use crate::doc::{Document, Page, Text, TextCommand};
use crate::font::{Font, FontFamily, FontFilter, FontError};
use crate::Context;

mod size;
pub use size::Size;


/// The core typesetting engine, transforming an abstract syntax tree into a document.
pub(crate) struct Engine<'a> {
    // Immutable
    tree: &'a SyntaxTree<'a>,
    ctx: &'a Context<'a>,

    // Mutable
    fonts: Vec<Font>,
    active_font: usize,
    text_commands: Vec<TextCommand>,
    current_line: String,
    current_width: Size,
}

impl<'a> Engine<'a> {
    /// Create a new generator from a syntax tree.
    pub fn new(tree: &'a SyntaxTree<'a>, context: &'a Context<'a>) -> Engine<'a> {
        Engine {
            tree,
            ctx: context,
            fonts: Vec::new(),
            active_font: 0,
            text_commands: Vec::new(),
            current_line: String::new(),
            current_width: Size::zero(),
        }
    }

    /// Generate the abstract document.
    pub fn typeset(mut self) -> TypeResult<Document> {
        // Load font defined by style
        let mut font = None;
        let filter = FontFilter::new(&self.ctx.style.font_families);
        for provider in &self.ctx.font_providers {
            let available = provider.available();
            for info in available {
                if filter.matches(info) {
                    if let Some(mut source) = provider.get(info) {
                        let mut program = Vec::new();
                        source.read_to_end(&mut program)?;
                        font = Some(Font::new(program)?);
                        break;
                    }
                }
            }
        }

        let font = match font {
            Some(font) => font,
            None => return Err(TypesetError::MissingFont),
        };

        self.fonts.push(font);
        self.active_font = 0;

        // Move cursor to top-left position
        self.text_commands.push(TextCommand::Move(
            self.ctx.style.margin_left,
            self.ctx.style.height - self.ctx.style.margin_top
        ));

        // Set the current font
        self.text_commands.push(TextCommand::SetFont(0, self.ctx.style.font_size));

        // Iterate through the documents nodes.
        for node in &self.tree.nodes {
            match node {
                Node::Word(word) => self.write_word(word),

                Node::Space => self.write_space(),
                Node::Newline => (),

                Node::ToggleItalics | Node::ToggleBold | Node::ToggleMath => unimplemented!(),
                Node::Func(_) => unimplemented!(),
            }
        }

        // Create a page from the contents.
        let page = Page {
            width: self.ctx.style.width,
            height: self.ctx.style.height,
            text: vec![Text {
                commands: self.text_commands,
            }],
        };

        Ok(Document {
            pages: vec![page],
            fonts: self.fonts,
        })
    }

    fn write_word(&mut self, word: &str) {
        let font = &self.fonts[self.active_font];

        let width = self.width(word);
        if self.would_overflow(width) {
            let vertical_move = - self.ctx.style.font_size
                * self.ctx.style.line_spacing
                * font.metrics.ascender;
            self.text_commands.push(TextCommand::Move(Size::zero(), vertical_move));

            self.current_line.clear();
            self.current_width = Size::zero();
        }

        self.text_commands.push(TextCommand::Text(word.to_owned()));
        self.current_line.push_str(word);
        self.current_width += width;
    }

    fn write_space(&mut self) {
        let space_width = self.width(" ");

        if !self.would_overflow(space_width) && !self.current_line.is_empty() {
            self.text_commands.push(TextCommand::Text(" ".to_owned()));
            self.current_line.push_str(" ");
            self.current_width += space_width;
        }
    }

    fn width(&self, word: &str) -> Size {
        let font = &self.fonts[self.active_font];
        word.chars()
            .map(|c| font.widths[font.map(c) as usize] * self.ctx.style.font_size)
            .sum()
    }

    fn would_overflow(&self, width: Size) -> bool {
        let max_width = self.ctx.style.width
            - self.ctx.style.margin_left
            - self.ctx.style.margin_right;

        self.current_width + width > max_width
    }
}

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
}

impl Default for Style {
    fn default() -> Style {
        use FontFamily::*;
        Style {
            // A4 paper.
            width: Size::from_mm(210.0),
            height: Size::from_mm(297.0),

            // Margins. A bit more on top and bottom.
            margin_left: Size::from_cm(2.5),
            margin_top: Size::from_cm(3.0),
            margin_right: Size::from_cm(2.5),
            margin_bottom: Size::from_cm(3.0),

            // Default font family, font size and line spacing.
            font_families: vec![SansSerif, Serif, Monospace],
            font_size: 12.0,
            line_spacing: 1.25,
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

error_type! {
    err: TypesetError,
    res: TypeResult,
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
