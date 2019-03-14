//! The compiler for the _Typeset_ typesetting language 📜.
//!
//! # Compilation
//! - **Parsing:** The parsing step first transforms a plain string into an
//!   [iterator of tokens](Tokens). Then the parser operates on that to
//!   construct an abstract syntax tree. The structures describing the tree
//!   can be found in the [`syntax`](syntax) module.
//! - **Typesetting:** The next step is to transform the syntax tree into an
//!   abstract document representation. Types for these can be found in the
//!   [`doc`](doc) module. This representation contains already the finished
//!   layout, but is still portable.
//! - **Exporting:** The abstract document can then be exported into supported
//!   formats. Currently the only supported format is _PDF_. In this step
//!   the text is finally encoded into glyph indices and font data is
//!   subsetted.
//!
//! # Fonts
//! To do the typesetting, the compiler needs font data. To be highly portable
//! the compiler assumes nothing about the environment. To still work with fonts,
//! the consumer of this library has to add _font providers_ to their compiler
//! instance. These can be queried for font data given a flexible font configuration
//! specifying font families and styles. A font provider is a type implementing the
//! [`FontProvider`](crate::font::FontProvider) trait. For convenience there exists
//! the [`FileFontProvider`](crate::font::FileFontProvider) to serve fonts from a
//! local folder.
//!
//! # Example
//! ```
//! use std::fs::File;
//! use typeset::{Compiler, font::FileFontProvider, file_font};
//!
//! // Simple example source code.
//! let source = "Hello World from Typeset!";
//!
//! // Create a compiler with a font provider that provides one font.
//! let mut compiler = Compiler::new();
//! compiler.add_font_provider(FileFontProvider::new("../fonts", vec![
//!     // Font family name, generic families, file, bold, italic
//!     file_font!("NotoSans", [SansSerif], "NotoSans-Regular.ttf", false, false),
//! ]));
//!
//! // Open an output file, compile and write to the file.
//! # /*
//! let mut file = File::create("hello-typeset.pdf").unwrap();
//! # */
//! # let mut file = File::create("../target/typeset-hello.pdf").unwrap();
//! compiler.write_pdf(source, &mut file).unwrap();
//! ```

pub mod syntax;
pub mod doc;
pub mod font;
mod parsing;
mod engine;
mod pdf;
mod utility;

pub use crate::parsing::{Tokens, ParseError};
pub use crate::engine::TypesetError;
pub use crate::pdf::PdfError;

use std::error;
use std::fmt;
use std::io::Write;
use crate::syntax::SyntaxTree;
use crate::parsing::Parser;
use crate::doc::{Document, Style};
use crate::font::FontProvider;
use crate::engine::Engine;
use crate::pdf::PdfCreator;


/// Compiles source code into typesetted documents allowing to
/// retrieve results at various stages.
pub struct Compiler<'p> {
    context: Context<'p>,
}

struct Context<'p> {
    /// Style for typesetting.
    style: Style,
    /// Font providers.
    font_providers: Vec<Box<dyn FontProvider + 'p>>,
}

impl<'p> Compiler<'p> {
    /// Create a new compiler from a document.
    #[inline]
    pub fn new() -> Compiler<'p> {
        Compiler {
            context: Context {
                style: Style::default(),
                font_providers: Vec::new(),
            }
        }
    }

    /// Set the default style for typesetting.
    #[inline]
    pub fn style(&mut self, style: Style) -> &mut Self {
        self.context.style = style;
        self
    }

    /// Add a font provider.
    #[inline]
    pub fn add_font_provider<P: 'p>(&mut self, provider: P) -> &mut Self
    where P: FontProvider {
        self.context.font_providers.push(Box::new(provider));
        self
    }

    /// Return an iterator over the tokens of the document.
    #[inline]
    pub fn tokenize<'s>(&self, source: &'s str) -> Tokens<'s> {
        Tokens::new(source)
    }

    /// Return the abstract syntax tree representation of the document.
    #[inline]
    pub fn parse<'s>(&self, source: &'s str) -> Result<SyntaxTree<'s>, ParseError> {
        Parser::new(self.tokenize(source)).parse()
    }

    /// Return the abstract typesetted representation of the document.
    #[inline]
    pub fn typeset(&self, source: &str) -> Result<Document, Error> {
        let tree = self.parse(source)?;
        Engine::new(&tree, &self.context).typeset().map_err(Into::into)
    }

    /// Write the document as a _PDF_, returning how many bytes were written.
    pub fn write_pdf<W: Write>(&self, source: &str, target: &mut W) -> Result<usize, Error> {
        let document = self.typeset(source)?;
        PdfCreator::new(&document, target)?.write().map_err(Into::into)
    }
}

/// The general error type for compilation.
pub enum Error {
    /// An error that occured while transforming source code into
    /// an abstract syntax tree.
    Parse(ParseError),
    /// An error that occured while typesetting into an abstract document.
    Typeset(TypesetError),
    /// An error that occured while writing the document as a _PDF_.
    Pdf(PdfError),
}

impl error::Error for Error {
    #[inline]
    fn source(&self) -> Option<&(dyn error::Error + 'static)> {
        match self {
            Error::Parse(err) => Some(err),
            Error::Typeset(err) => Some(err),
            Error::Pdf(err) => Some(err),
        }
    }
}

impl fmt::Display for Error {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Error::Parse(err) => write!(f, "parse error: {}", err),
            Error::Typeset(err) => write!(f, "typeset error: {}", err),
            Error::Pdf(err) => write!(f, "pdf error: {}", err),
        }
    }
}

impl fmt::Debug for Error {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        fmt::Display::fmt(self, f)
    }
}

impl From<ParseError> for Error {
    #[inline]
    fn from(err: ParseError) -> Error {
        Error::Parse(err)
    }
}

impl From<TypesetError> for Error {
    #[inline]
    fn from(err: TypesetError) -> Error {
        Error::Typeset(err)
    }
}

impl From<PdfError> for Error {
    #[inline]
    fn from(err: PdfError) -> Error {
        Error::Pdf(err)
    }
}


#[cfg(test)]
mod test {
    use std::fs::File;
    use crate::Compiler;
    use crate::font::FileFontProvider;

    /// Create a pdf with a name from the source code.
    fn test(name: &str, src: &str) {
        // Create compiler
        let mut compiler = Compiler::new();
        let provider = FileFontProvider::new("../fonts", vec![
            // Font family name, generic families, file, bold, italic
            file_font!("NotoSans", [SansSerif], "NotoSans-Regular.ttf", false, false),
            file_font!("NotoSans", [SansSerif], "NotoSans-Bold.ttf", true, false),
            file_font!("NotoSans", [SansSerif], "NotoSans-Italic.ttf", false, true),
            file_font!("NotoSans", [SansSerif], "NotoSans-BoldItalic.ttf", true, true),
            file_font!("NotoSansMath", [SansSerif], "NotoSansMath-Regular.ttf", false, false),
            file_font!("NotoEmoji", [SansSerif, Serif, Monospace],
                       "NotoEmoji-Regular.ttf", false, false),
        ]);
        compiler.add_font_provider(provider);

        // Open output file;
        let path = format!("../target/typeset-pdf-{}.pdf", name);
        let mut file = File::create(path).unwrap();

        // Compile and output
        compiler.write_pdf(src, &mut file).unwrap();
    }

    #[test]
    fn small() {
        test("unicode", "∑mbe∂∂ed font with Unicode!");
        test("parentheses", "Text with ) and ( or (enclosed) works.");
        test("composite-glyph", "Composite character‼");
        test("multiline","
            Lorem ipsum dolor sit amet, consetetur sadipscing elitr, sed diam nonumy
            eirmod tempor invidunt ut labore et dolore magna aliquyam erat, sed diam
            voluptua. At vero eos et accusam et justo duo dolores et ea rebum. Stet
            clita kasd gubergren, no sea takimata sanctus est.
        ");
    }

    #[test]
    fn long_styled() {
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
