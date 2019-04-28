//! Core typesetting engine.

use std::cell::{RefCell, Ref};
use std::collections::HashMap;
use std::mem::swap;
use smallvec::SmallVec;
use crate::syntax::{SyntaxTree, Node};
use crate::doc::{Document, Page, Text, TextCommand};
use crate::font::{Font, FontFamily, FontInfo, FontError};
use crate::Context;

mod size;
pub use size::Size;


/// The core typesetting engine, transforming an abstract syntax tree into a document.
pub struct Engine<'a> {
    // Input
    tree: &'a SyntaxTree,
    ctx: &'a Context<'a>,

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
    pub(crate) fn new(tree: &'a SyntaxTree, context: &'a Context<'a>) -> Engine<'a> {
        Engine {
            tree,
            ctx: context,
            font_loader: FontLoader::new(context),
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
    pub(crate) fn typeset(mut self) -> TypeResult<Document> {
        // Start by moving to a suitable position.
        self.move_start();

        // Iterate through the documents nodes.
        for node in &self.tree.nodes {
            match node {
                Node::Word(word) => self.write_word(word)?,
                Node::Space => self.write_space()?,
                Node::Newline => {
                    self.write_buffered_text();
                    self.move_newline(self.ctx.style.paragraph_spacing);
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
                width: self.ctx.style.width,
                height: self.ctx.style.height,
                text: vec![Text {
                    commands: self.text_commands,
                }],
            }],
            fonts: self.font_loader.into_fonts(),
        })
    }

    /// Write a word.
    fn write_word(&mut self, word: &str) -> TypeResult<()> {
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
    fn write_space(&mut self) -> TypeResult<()> {
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
            self.ctx.style.margin_left,
            self.ctx.style.height - self.ctx.style.margin_top
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
            self.ctx.style.font_size
                * self.ctx.style.line_spacing
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
        self.text_commands.push(TextCommand::SetFont(index, self.ctx.style.font_size));
        self.active_font = index;
    }

    /// Whether the current line plus the extra `width` would overflow the line.
    fn would_overflow(&self, width: Size) -> bool {
        let max_width = self.ctx.style.width
            - self.ctx.style.margin_left - self.ctx.style.margin_right;
        self.current_line_width + width > max_width
    }

    /// Load a font that has the character we need.
    fn get_font_for(&self, character: char) -> TypeResult<(usize, Ref<Font>)> {
        self.font_loader.get(FontQuery {
            families: &self.ctx.style.font_families,
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
        font.widths[font.map(character) as usize] * self.ctx.style.font_size
    }
}

/// Serves matching fonts given a query.
struct FontLoader<'a> {
    /// The context containing the used font providers.
    context: &'a Context<'a>,
    /// All available fonts indexed by provider.
    provider_fonts: Vec<&'a [FontInfo]>,
    /// The internal state.
    state: RefCell<FontLoaderState<'a>>,
}

/// Internal state of the font loader (wrapped in a RefCell).
struct FontLoaderState<'a> {
    /// The loaded fonts along with their external indices.
    fonts: Vec<(Option<usize>, Font)>,
    /// Allows to retrieve cached results for queries.
    query_cache: HashMap<FontQuery<'a>, usize>,
    /// Allows to lookup fonts by their infos.
    info_cache: HashMap<&'a FontInfo, usize>,
    /// Indexed by outside and indices maps to internal indices.
    inner_index: Vec<usize>,
}

impl<'a> FontLoader<'a> {
    /// Create a new font loader.
    pub fn new(context: &'a Context<'a>) -> FontLoader {
        let provider_fonts = context.font_providers.iter()
            .map(|prov| prov.available()).collect();

        FontLoader {
            context,
            provider_fonts,
            state: RefCell::new(FontLoaderState {
                query_cache: HashMap::new(),
                info_cache: HashMap::new(),
                inner_index: vec![],
                fonts: vec![],
            }),
        }
    }

    /// Return the best matching font and it's index (if there is any) given the query.
    pub fn get(&self, query: FontQuery<'a>) -> Option<(usize, Ref<Font>)> {
        // Check if we had the exact same query before.
        let state = self.state.borrow();
        if let Some(&index) = state.query_cache.get(&query) {
            // That this is the query cache means it must has an index as we've served it before.
            let extern_index = state.fonts[index].0.unwrap();
            let font = Ref::map(state, |s| &s.fonts[index].1);

            return Some((extern_index, font));
        }
        drop(state);

        // Go over all font infos from all font providers that match the query.
        for family in query.families {
            for (provider, infos) in self.context.font_providers.iter().zip(&self.provider_fonts) {
                for info in infos.iter() {
                    // Check whether this info matches the query.
                    if Self::matches(query, family, info) {
                        let mut state = self.state.borrow_mut();

                        // Check if we have already loaded this font before.
                        // Otherwise we'll fetch the font from the provider.
                        let index = if let Some(&index) = state.info_cache.get(info) {
                            index
                        } else if let Some(mut source) = provider.get(info) {
                            // Read the font program into a vec.
                            let mut program = Vec::new();
                            source.read_to_end(&mut program).ok()?;

                            // Create a font from it.
                            let font = Font::new(program).ok()?;

                            // Insert it into the storage.
                            let index = state.fonts.len();
                            state.info_cache.insert(info, index);
                            state.fonts.push((None, font));

                            index
                        } else {
                            continue;
                        };

                        // Check whether this font has the character we need.
                        let has_char = state.fonts[index].1.mapping.contains_key(&query.character);
                        if has_char {
                            // We can take this font, so we store the query.
                            state.query_cache.insert(query, index);

                            // Now we have to find out the external index of it, or assign a new
                            // one if it has not already one.
                            let maybe_extern_index = state.fonts[index].0;
                            let extern_index = maybe_extern_index.unwrap_or_else(|| {
                                // We have to assign an external index before serving.
                                let extern_index = state.inner_index.len();
                                state.inner_index.push(index);
                                state.fonts[index].0 =  Some(extern_index);
                                extern_index
                            });

                            // Release the mutable borrow and borrow immutably.
                            drop(state);
                            let font = Ref::map(self.state.borrow(), |s| &s.fonts[index].1);

                            // Finally we can return it.
                            return Some((extern_index, font));
                        }
                    }
                }
            }
        }

        None
    }

    /// Return a loaded font at an index. Panics if the index is out of bounds.
    pub fn get_with_index(&self, index: usize) -> Ref<Font> {
        let state = self.state.borrow();
        let internal = state.inner_index[index];
        Ref::map(state, |s| &s.fonts[internal].1)
    }

    /// Return the list of fonts.
    pub fn into_fonts(self) -> Vec<Font> {
        // Sort the fonts by external key so that they are in the correct order.
        let mut fonts = self.state.into_inner().fonts;
        fonts.sort_by_key(|&(maybe_index, _)| match maybe_index {
            Some(index) => index as isize,
            None => -1,
        });

        // Remove the fonts that are not used from the outside
        fonts.into_iter().filter_map(|(maybe_index, font)| {
            maybe_index.map(|_| font)
        }).collect()
    }

    /// Check whether the query and the current family match the info.
    fn matches(query: FontQuery, family: &FontFamily, info: &FontInfo) -> bool {
        info.families.contains(family)
          && info.italic == query.italic && info.bold == query.bold
    }
}

/// A query for a font with specific properties.
#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
struct FontQuery<'a> {
    /// A fallback list of font families to accept. The first family in this list, that also
    /// satisfies the other conditions, shall be returned.
    families: &'a [FontFamily],
    /// Whether the font shall be in italics.
    italic: bool,
    /// Whether the font shall be in boldface.
    bold: bool,
    /// Which character we need.
    character: char,
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
