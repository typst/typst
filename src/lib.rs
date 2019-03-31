//! The compiler for the _Typeset_ typesetting language 📜.
//!
//! # Compilation
//! - **Parsing:** The parsing step first transforms a plain string into an
//!   [iterator of tokens](crate::parsing::Tokens). Then the [parser](crate::parsing::Parser)
//!   operates on that to construct a syntax tree. The structures describing the tree can be found
//!   in the [syntax] module.
//! - **Typesetting:** The next step is to transform the syntax tree into a portable representation
//!   of the typesetted document. Types for these can be found in the [doc] module. This
//!   representation contains already the finished layout.
//! - **Exporting:** The finished document can then be exported into supported formats. Submodules
//!   for the supported formats are located in the [export] module. Currently the only supported
//!   format is _PDF_.
//!
//! # Example
//! ```
//! use std::fs::File;
//! use typeset::Compiler;
//! use typeset::{font::FileSystemFontProvider, font_info};
//! use typeset::export::pdf::PdfExporter;
//!
//! // Simple example source code.
//! let src = "Hello World from Typeset!";
//!
//! // Create a compiler with a font provider that provides three fonts
//! // (the default sans-serif fonts and a fallback for the emoji).
//! let mut compiler = Compiler::new();
//! compiler.add_font_provider(FileSystemFontProvider::new("../fonts", vec![
//!     ("NotoSans-Regular.ttf", font_info!(["NotoSans", "Noto", SansSerif])),
//!     ("NotoSans-Italic.ttf", font_info!(["NotoSans", "Noto", SansSerif], italic)),
//!     ("NotoEmoji-Regular.ttf", font_info!(["NotoEmoji", "Noto", SansSerif, Serif, Monospace])),
//! ]));
//!
//! // Compile the source code with the compiler.
//! let document = compiler.typeset(src).unwrap();
//!
//! // Export the document into a PDF file.
//! # /*
//! let file = File::create("hello-typeset.pdf").unwrap();
//! # */
//! # let file = File::create("../target/typeset-hello.pdf").unwrap();
//! let exporter = PdfExporter::new();
//! exporter.export(&document, file).unwrap();
//! ```

use crate::syntax::SyntaxTree;
use crate::parsing::{Tokens, Parser, ParseError};
use crate::doc::Document;
use crate::font::FontProvider;
use crate::engine::{Engine, Style, TypesetError};

#[macro_use]
mod error;
mod utility;
pub mod doc;
pub mod engine;
pub mod export;
#[macro_use]
pub mod font;
pub mod parsing;
pub mod syntax;


/// Transforms source code into typesetted documents.
///
/// Holds the compilation context, which can be configured through various methods.
pub struct Compiler<'p> {
    context: Context<'p>,
}

struct Context<'p> {
    /// Style for typesetting.
    style: Style,
    /// Font providers.
    font_providers: Vec<Box<dyn FontProvider + 'p>>,
}

/// Functions to set up the compilation context.
impl<'p> Compiler<'p> {
    /// Create a new compiler.
    #[inline]
    pub fn new() -> Compiler<'p> {
        Compiler {
            context: Context {
                style: Style::default(),
                font_providers: vec![],
            }
        }
    }

    /// Set the default style for the document.
    #[inline]
    pub fn set_style(&mut self, style: Style) {
        self.context.style = style;
    }

    /// Add a font provider to the context of this compiler.
    #[inline]
    pub fn add_font_provider<P: 'p>(&mut self, provider: P) where P: FontProvider {
        self.context.font_providers.push(Box::new(provider));
    }
}

/// Compilation functions.
impl<'p> Compiler<'p> {
    /// Parse source code into a syntax tree.
    #[inline]
    pub fn parse<'s>(&self, src: &'s str) -> Result<SyntaxTree<'s>, ParseError> {
        Parser::new(Tokens::new(src)).parse()
    }

    /// Compile a portable typesetted document from source code.
    #[inline]
    pub fn typeset(&self, src: &str) -> Result<Document, Error> {
        let tree = self.parse(src)?;
        let engine = Engine::new(&tree, &self.context);
        engine.typeset().map_err(Into::into)
    }
}

/// The general error type for compilation.
pub enum Error {
    /// An error that occured while transforming source code into
    /// an abstract syntax tree.
    Parse(ParseError),
    /// An error that occured while typesetting into an abstract document.
    Typeset(TypesetError),
}

error_type! {
    err: Error,
    show: f => match err {
        Error::Parse(e) => write!(f, "parse error: {}", e),
        Error::Typeset(e) => write!(f, "typeset error: {}", e),
    },
    source: match err {
        Error::Parse(e) => Some(e),
        Error::Typeset(e) => Some(e),
    },
    from: (ParseError, Error::Parse(err)),
    from: (TypesetError, Error::Typeset(err)),
}


#[cfg(test)]
mod test {
    use std::fs::File;
    use crate::Compiler;
    use crate::export::pdf::PdfExporter;
    use crate::font::FileSystemFontProvider;

    /// Create a pdf with a name from the source code.
    fn test(name: &str, src: &str) {
        // Create compiler
        let mut compiler = Compiler::new();
        compiler.add_font_provider(FileSystemFontProvider::new("../fonts", vec![
            ("NotoSans-Regular.ttf",     font_info!(["NotoSans", "Noto", SansSerif])),
            ("NotoSans-Italic.ttf",      font_info!(["NotoSans", "Noto", SansSerif], italic)),
            ("NotoSans-Bold.ttf",        font_info!(["NotoSans", "Noto", SansSerif], bold)),
            ("NotoSans-BoldItalic.ttf",  font_info!(["NotoSans", "Noto", SansSerif], italic, bold)),
            ("NotoSansMath-Regular.ttf", font_info!(["NotoSansMath", "Noto", SansSerif])),
            ("NotoEmoji-Regular.ttf",    font_info!(["NotoEmoji", "Noto", SansSerif, Serif, Monospace])),
        ]));

        // Compile into document
        let document = compiler.typeset(src).unwrap();

        // Write to file
        let path = format!("../target/typeset-pdf-{}.pdf", name);
        let file = File::create(path).unwrap();
        let exporter = PdfExporter::new();
        exporter.export(&document, file).unwrap();
    }

    #[test]
    fn small() {
        test("parentheses", "Text with ) and ( or (enclosed) works.");
        test("multiline","
            Lorem ipsum dolor sit amet, consetetur sadipscing elitr, sed diam nonumy
            eirmod tempor invidunt ut labore et dolore magna aliquyam erat, sed diam
            voluptua. At vero eos et accusam et justo duo dolores et ea rebum. Stet
            clita kasd gubergren, no sea takimata sanctus est.
        ");
    }

    #[test]
    fn unicode() {
        test("unicode", "∑mbe∂∂ed font with Unicode!");
    }

    #[test]
    fn composite_glyph() {
        test("composite-glyph", "Composite character‼");
    }

    #[test]
    fn long_wikipedia() {
        test("wikipedia", r#"
            Typesetting is the composition of text by means of arranging physical types or the
            digital equivalents. Stored letters and other symbols (called sorts in mechanical
            systems and glyphs in digital systems) are retrieved and ordered according to a
            language's orthography for visual display. Typesetting requires one or more fonts
            (which are widely but erroneously confused with and substituted for typefaces). One
            significant effect of typesetting was that authorship of works could be spotted more
            easily, making it difficult for copiers who have not gained permission.

            During much of the letterpress era, movable type was composed by hand for each page.
            Cast metal sorts were composed into words, then lines, then paragraphs, then pages of
            text and tightly bound together to make up a form, with all letter faces exactly the
            same "height to paper", creating an even surface of type. The form was placed in a
            press, inked, and an impression made on paper.

            During typesetting, individual sorts are picked from a type case with the right hand,
            and set into a composing stick held in the left hand from left to right, and as viewed
            by the setter upside down. As seen in the photo of the composing stick, a lower case
            'q' looks like a 'd', a lower case 'b' looks like a 'p', a lower case 'p' looks like a
            'b' and a lower case 'd' looks like a 'q'. This is reputed to be the origin of the
            expression "mind your p's and q's". It might just as easily have been "mind your b's
            and d's".

            The diagram at right illustrates a cast metal sort: a face, b body or shank, c point
            size, 1 shoulder, 2 nick, 3 groove, 4 foot. Wooden printing sorts were in use for
            centuries in combination with metal type. Not shown, and more the concern of the
            casterman, is the “set”, or width of each sort. Set width, like body size, is measured
            in points.

            In order to extend the working life of type, and to account for the finite sorts in a
            case of type, copies of forms were cast when anticipating subsequent printings of a
            text, freeing the costly type for other work. This was particularly prevalent in book
            and newspaper work where rotary presses required type forms to wrap an impression
            cylinder rather than set in the bed of a press. In this process, called stereotyping,
            the entire form is pressed into a fine matrix such as plaster of Paris or papier mâché
            called a flong to create a positive, from which the stereotype form was electrotyped,
            cast of type metal.

            Advances such as the typewriter and computer would push the state of the art even
            farther ahead. Still, hand composition and letterpress printing have not fallen
            completely out of use, and since the introduction of digital typesetting, it has seen a
            revival as an artisanal pursuit. However, it is a very small niche within the larger
            typesetting market.
        "#);
    }
}
