//! Core typesetting engine.

use std::io;
use std::error;
use std::fmt;
use crate::syntax::{SyntaxTree, Node};
use crate::doc::{Document, Size, Page, Text, TextCommand};
use crate::font::{Font, FontConfig, FontError};
use crate::Context;


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
        let config = FontConfig::new(self.ctx.style.font_families.clone());
        for provider in &self.ctx.font_providers {
            if let Some(mut source) = provider.provide(&config) {
                let mut program = Vec::new();
                source.read_to_end(&mut program)?;
                font = Some(Font::new(program)?);
                break;
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

/// Result type used for typesetting.
type TypeResult<T> = std::result::Result<T, TypesetError>;

/// The error type for typesetting.
pub enum TypesetError {
    /// There was no suitable font.
    MissingFont,
    /// An error occured while gathering font data.
    Font(FontError),
    /// An I/O Error on occured while reading a font.
    Io(io::Error),
}

impl error::Error for TypesetError {
    #[inline]
    fn source(&self) -> Option<&(dyn error::Error + 'static)> {
        match self {
            TypesetError::Font(err) => Some(err),
            TypesetError::Io(err) => Some(err),
            _ => None,
        }
    }
}

impl fmt::Display for TypesetError {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            TypesetError::MissingFont => write!(f, "missing font"),
            TypesetError::Font(err) => write!(f, "font error: {}", err),
            TypesetError::Io(err) => write!(f, "io error: {}", err),
        }
    }
}

impl fmt::Debug for TypesetError {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        fmt::Display::fmt(self, f)
    }
}

impl From<io::Error> for TypesetError {
    #[inline]
    fn from(err: io::Error) -> TypesetError {
        TypesetError::Io(err)
    }
}

impl From<FontError> for TypesetError {
    #[inline]
    fn from(err: FontError) -> TypesetError {
        TypesetError::Font(err)
    }
}
