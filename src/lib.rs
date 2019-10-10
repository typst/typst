//! The compiler for the _Typst_ typesetting language.
//!
//! # Steps
//! - **Parsing:** The parsing step first transforms a plain string into an [iterator of
//!   tokens](crate::parsing::Tokens). Then the [parser](crate::parsing::Parser) operates on that to
//!   construct a syntax tree. The structures describing the tree can be found in the [syntax]
//!   module.
//! - **Layouting:** The next step is to transform the syntax tree into a portable representation of
//!   the typesetted document. Types for these can be found in the [doc] and [layout] modules. This
//!   representation contains already the finished layout.
//! - **Exporting:** The finished document can then be exported into supported formats. Submodules
//!   for the supported formats are located in the [export] module. Currently the only supported
//!   format is _PDF_.

pub extern crate toddle;

use std::cell::RefCell;
use toddle::query::{FontLoader, SharedFontLoader, FontProvider};

use crate::doc::Document;
use crate::func::Scope;
use crate::parsing::{parse, ParseContext, ParseResult, ParseError};
use crate::layout::{layout, LayoutContext, Alignment, LayoutSpace, LayoutError, LayoutResult};
use crate::layout::boxed::BoxLayout;
use crate::style::{PageStyle, TextStyle};
use crate::syntax::SyntaxTree;

#[macro_use]
mod error;
pub mod doc;
pub mod export;
pub mod func;
pub mod layout;
pub mod parsing;
pub mod size;
pub mod style;
pub mod syntax;
pub mod library;


/// Transforms source code into typesetted documents.
///
/// Can be configured through various methods.
pub struct Typesetter<'p> {
    /// The font loader shared by all typesetting processes.
    loader: SharedFontLoader<'p>,
    /// The default text style.
    text_style: TextStyle,
    /// The default page style.
    page_style: PageStyle,
}

impl<'p> Typesetter<'p> {
    /// Create a new typesetter.
    #[inline]
    pub fn new() -> Typesetter<'p> {
        Typesetter {
            loader: RefCell::new(FontLoader::new()),
            text_style: TextStyle::default(),
            page_style: PageStyle::default(),
        }
    }

    /// Set the default page style for the document.
    #[inline]
    pub fn set_page_style(&mut self, style: PageStyle) {
        self.page_style = style;
    }

    /// Set the default text style for the document.
    #[inline]
    pub fn set_text_style(&mut self, style: TextStyle) {
        self.text_style = style;
    }

    /// Add a font provider to the context of this typesetter.
    #[inline]
    pub fn add_font_provider<P: 'p>(&mut self, provider: P) where P: FontProvider {
        self.loader.get_mut().add_provider(provider);
    }

    /// Parse source code into a syntax tree.
    pub fn parse(&self, src: &str) -> ParseResult<SyntaxTree> {
        let scope = Scope::with_std();
        parse(src, ParseContext { scope: &scope })
    }

    /// Layout a syntax tree and return the layout and the referenced font list.
    pub fn layout(&self, tree: &SyntaxTree) -> LayoutResult<BoxLayout> {
        let pages = layout(&tree, LayoutContext {
            loader: &self.loader,
            style: &self.text_style,
            space: LayoutSpace {
                dimensions: self.page_style.dimensions,
                padding: self.page_style.margins,
                alignment: Alignment::Left,
                shrink_to_fit: false,
            },
        })?;
        Ok(pages)
    }

    /// Typeset a portable document from source code.
    pub fn typeset(&self, src: &str) -> Result<Document, TypesetError> {
        let tree = self.parse(src)?;
        let layout = self.layout(&tree)?;
        let document = layout.into_doc();
        Ok(document)
    }

    /// A reference to the backing font loader.
    pub fn loader(&self) -> &SharedFontLoader<'p> {
        &self.loader
    }
}


/// The general error type for typesetting.
pub enum TypesetError {
    /// An error that occured while parsing.
    Parse(ParseError),
    /// An error that occured while layouting.
    Layout(LayoutError),
}

error_type! {
    err: TypesetError,
    show: f => match err {
        TypesetError::Parse(e) => write!(f, "parse error: {}", e),
        TypesetError::Layout(e) => write!(f, "layout error: {}", e),
    },
    source: match err {
        TypesetError::Parse(e) => Some(e),
        TypesetError::Layout(e) => Some(e),
    },
    from: (ParseError, TypesetError::Parse(err)),
    from: (LayoutError, TypesetError::Layout(err)),
}


#[cfg(test)]
mod test {
    use std::fs::File;
    use std::io::BufWriter;
    use crate::Typesetter;
    use crate::export::pdf::PdfExporter;
    use toddle::query::FileSystemFontProvider;

    /// Create a _PDF_ with a name from the source code.
    fn test(name: &str, src: &str) {
        let mut typesetter = Typesetter::new();
        let provider = FileSystemFontProvider::from_listing("fonts/fonts.toml").unwrap();
        typesetter.add_font_provider(provider);

        // Typeset into document.
        let document = typesetter.typeset(src).unwrap();

        // Write to file.
        let path = format!("../target/typeset-unit-{}.pdf", name);
        let file = BufWriter::new(File::create(path).unwrap());
        let exporter = PdfExporter::new();
        exporter.export(&document, typesetter.loader(), file).unwrap();
    }

    #[test]
    fn features() {
        test("features", r"
            *Features Test Page*

            _Multiline:_
            Lorem ipsum dolor sit amet, consetetur sadipscing elitr, sed diam nonumy
            eirmod tempor invidunt ut labore et dolore magna aliquyam erat, sed diam
            voluptua. At vero eos et accusam et justo duo dolores et ea rebum. Stet
            clita kasd gubergren, no sea takimata sanctus est.

            _Emoji:_ Hello World! 🌍

            _Styles:_ This is made *bold*, that _italic_ and this one `monospace` using the
            built-in syntax!

            _Styles with functions:_ This [bold][word] is made bold and [italic][that] is italic
            using the standard library functions [mono][bold] and `italic`!
        ");
    }

    #[test]
    fn shakespeare() {
        test("shakespeare", include_str!("../test/shakespeare.tps"));
        test("shakespeare-right", &format!("[align:right][{}]", include_str!("../test/shakespeare.tps")));
    }
}
