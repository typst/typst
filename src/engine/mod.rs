//! Core typesetting engine.

use std::cell::{RefCell, Ref};
use std::collections::HashMap;
use std::mem::swap;
use crate::syntax::{SyntaxTree, Node};
use crate::doc::{Document, Page, Text, TextCommand};
use crate::font::{Font, FontFamily, FontInfo, FontError};
use crate::Context;

mod size;
pub use size::Size;


/// The core typesetting engine, transforming an abstract syntax tree into a document.
pub struct Engine<'t> {
    // Input
    tree: &'t SyntaxTree<'t>,
    ctx: &'t Context<'t>,

    // Internal
    font_loader: FontLoader<'t>,

    // Output
    text_commands: Vec<TextCommand>,

    // Intermediates
    active_font: usize,
    current_text: String,
    current_line_width: Size,
    current_max_vertical_move: Size,
}

impl<'t> Engine<'t> {
    /// Create a new generator from a syntax tree.
    pub(crate) fn new(tree: &'t SyntaxTree<'t>, context: &'t Context<'t>) -> Engine<'t> {
        Engine {
            tree,
            ctx: context,
            font_loader: FontLoader::new(context),
            text_commands: vec![],
            active_font: std::usize::MAX,
            current_text: String::new(),
            current_line_width: Size::zero(),
            current_max_vertical_move: Size::zero(),
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
                Node::Newline => (),
                Node::ToggleItalics | Node::ToggleBold | Node::ToggleMath => unimplemented!(),
                Node::Func(_) => unimplemented!(),
            }
        }

        // Flush the text buffer.
        self.write_buffered_text();

        let fonts =  self.font_loader.into_fonts();

        println!("fonts: {:?}", fonts.len());

        // Create a document with one page from the contents.
        Ok(Document {
            pages: vec![Page {
                width: self.ctx.style.width,
                height: self.ctx.style.height,
                text: vec![Text {
                    commands: self.text_commands,
                }],
            }],
            fonts,
        })
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
    fn move_newline(&mut self) {
        let vertical_move = - if self.current_max_vertical_move == Size::zero() {
            // If max vertical move is still zero, the line is empty and we take the
            // font size from the previous line.
            self.ctx.style.font_size
                * self.ctx.style.line_spacing
                * self.font_loader.get_at(self.active_font).metrics.ascender
        } else {
            self.current_max_vertical_move
        };

        self.text_commands.push(TextCommand::Move(Size::zero(), vertical_move));
        self.current_max_vertical_move = Size::zero();
        self.current_line_width = Size::zero();
    }

    /// Set the current font.
    fn set_font(&mut self, index: usize) {
        self.text_commands.push(TextCommand::SetFont(index, self.ctx.style.font_size));
        self.active_font = index;
    }

    /// Write a word.
    fn write_word(&mut self, word: &str) -> TypeResult<()> {
        let width = self.width(word)?;

        // If this would overflow, we move to a new line and finally write the previous one.
        if self.would_overflow(width) {
            self.write_buffered_text();
            self.move_newline();
        }

        for c in word.chars() {
            let (index, _) = self.get_font_for(c)?;
            if index != self.active_font {
                self.write_buffered_text();
                self.set_font(index);
            }
            self.current_text.push(c);
            let char_width = self.char_width(c).unwrap();
            self.current_line_width += char_width;
        }

        Ok(())
    }

    /// Write the space character: `' '`.
    fn write_space(&mut self) -> TypeResult<()> {
        let space_width = self.char_width(' ')?;

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

    /// Whether the current line plus the extra `width` would overflow the line.
    fn would_overflow(&self, width: Size) -> bool {
        let max_width = self.ctx.style.width
            - self.ctx.style.margin_left - self.ctx.style.margin_right;
        self.current_line_width + width > max_width
    }

    /// The width of a word when printed out.
    fn width(&self, word: &str) -> TypeResult<Size> {
        let mut width = Size::zero();
        for c in word.chars() {
            width += self.char_width(c)?;
        }
        Ok(width)
    }

    /// The width of a char when printed out.
    fn char_width(&self, character: char) -> TypeResult<Size> {
        let font = self.get_font_for(character)?.1;
        Ok(font.widths[font.map(character) as usize] * self.ctx.style.font_size)
    }

    /// Load a font that has the character we need.
    fn get_font_for(&self, character: char) -> TypeResult<(usize, Ref<Font>)> {
        let res = self.font_loader.get(FontQuery {
            families: &self.ctx.style.font_families,
            italic: false,
            bold: false,
            character,
        }).ok_or_else(|| TypesetError::MissingFont)?;
        Ok(res)
    }
}

/// Serves matching fonts given a query.
struct FontLoader<'t> {
    /// The context containing the used font providers.
    context: &'t Context<'t>,
    /// All available fonts indexed by provider.
    availables: Vec<&'t [FontInfo]>,
    /// Allows to lookup fonts by their infos.
    indices: RefCell<HashMap<FontInfo, usize>>,
    /// Allows to retrieve cached results for queries.
    matches: RefCell<HashMap<FontQuery<'t>, usize>>,
    /// All loaded fonts.
    loaded: RefCell<Vec<Font>>,
    /// Indexed by outside and indices maps to internal indices.
    external: RefCell<Vec<usize>>,
}

impl<'t> FontLoader<'t> {
    /// Create a new font loader.
    pub fn new(context: &'t Context<'t>) -> FontLoader {
        let availables = context.font_providers.iter()
            .map(|prov| prov.available()).collect();

        FontLoader {
            context,
            availables,
            indices: RefCell::new(HashMap::new()),
            matches: RefCell::new(HashMap::new()),
            loaded: RefCell::new(vec![]),
            external: RefCell::new(vec![]),
        }
    }

    /// Return the list of fonts.
    pub fn into_fonts(self) -> Vec<Font> {
        // FIXME: Don't clone here.
        let fonts = self.loaded.into_inner();
        self.external.into_inner().into_iter().map(|index| fonts[index].clone()).collect()
    }

    /// Return the best matching font and it's index (if there is any) given the query.
    pub fn get(&self, query: FontQuery<'t>) -> Option<(usize, Ref<Font>)> {
        if let Some(index) = self.matches.borrow().get(&query) {
            let external = self.external.borrow().iter().position(|i| i == index).unwrap();
            return Some((external, self.get_at_internal(*index)));
        }

        // Go through all available fonts and try to find one.
        for family in query.families {
            for (p, available) in self.availables.iter().enumerate() {
                for info in available.iter() {
                    if Self::matches(query, &family, info) {
                        if let Some((index, font)) = self.try_load(info, p) {
                            if font.mapping.contains_key(&query.character) {
                                self.matches.borrow_mut().insert(query, index);

                                let pos = self.external.borrow().iter().position(|&i| i == index);
                                let external = pos.unwrap_or_else(|| {
                                    let external = self.external.borrow().len();
                                    self.external.borrow_mut().push(index);
                                    external
                                });

                                return Some((external, font));
                            }
                        }
                    }
                }
            }
        }

        None
    }

    /// Return a loaded font at an index. Panics if the index is out of bounds.
    pub fn get_at(&self, index: usize) -> Ref<Font> {
        let internal = self.external.borrow()[index];
        self.get_at_internal(internal)
    }

    /// Try to load the font with the given info from the provider.
    fn try_load(&self, info: &FontInfo,  provider: usize) -> Option<(usize, Ref<Font>)> {
        if let Some(index) = self.indices.borrow().get(info) {
            return Some((*index, self.get_at_internal(*index)));
        }

        if let Some(mut source) = self.context.font_providers[provider].get(info) {
            let mut program = Vec::new();
            source.read_to_end(&mut program).ok()?;

            let font = Font::new(program).ok()?;

            let index = self.loaded.borrow().len();
            println!("loading at interal index: {}", index);
            self.loaded.borrow_mut().push(font);
            self.indices.borrow_mut().insert(info.clone(), index);

            Some((index, self.get_at_internal(index)))
        } else {
            None
        }
    }

    /// Return a loaded font at an internal index. Panics if the index is out of bounds.
    fn get_at_internal(&self, index: usize) -> Ref<Font> {
        Ref::map(self.loaded.borrow(), |loaded| &loaded[index])
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
            font_size: 11.0,
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
