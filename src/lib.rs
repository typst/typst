//! The compiler for the _Typeset_ typesetting language 📜.
//!
//! # Compilation
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
//!
//! # Example
//! ```
//! use std::fs::File;
//! use typeset::Typesetter;
//! use typeset::{font::FileSystemFontProvider, font_info};
//! use typeset::export::pdf::PdfExporter;
//!
//! // Simple example source code.
//! let src = "Hello World from __Typeset__! 🌍";
//!
//! // Create a typesetter with a font provider that provides three fonts
//! // (two sans-serif fonts and a fallback for the emoji).
//! let mut typesetter = Typesetter::new();
//! typesetter.add_font_provider(FileSystemFontProvider::new("../fonts", vec![
//!     ("CMU-SansSerif-Regular.ttf", font_info!(["Computer Modern", SansSerif])),
//!     ("CMU-SansSerif-Italic.ttf",  font_info!(["Computer Modern", SansSerif], italic)),
//!     ("NotoEmoji-Regular.ttf", font_info!(["NotoEmoji", "Noto", SansSerif, Serif, Monospace])),
//! ]));
//!
//! // Typeset the source code into a document.
//! let document = typesetter.typeset(src).unwrap();
//!
//! // Export the document into a PDF file.
//! # /*
//! let file = File::create("hello-typeset.pdf").unwrap();
//! # */
//! # let file = File::create("../target/typeset-doc-hello.pdf").unwrap();
//! let exporter = PdfExporter::new();
//! exporter.export(&document, file).unwrap();
//! ```

use std::fmt::{self, Debug, Formatter};

use crate::doc::Document;
use crate::font::{Font, FontLoader, FontProvider};
use crate::func::Scope;
use crate::parsing::{parse, ParseContext, ParseResult, ParseError};
use crate::layout::{layout, LayoutContext, LayoutSpace, LayoutError, LayoutResult, BoxLayout};
use crate::style::{PageStyle, TextStyle};
use crate::syntax::SyntaxTree;

#[macro_use]
mod error;
pub mod doc;
pub mod export;
#[macro_use]
pub mod font;
pub mod func;
pub mod layout;
pub mod parsing;
pub mod size;
pub mod style;
pub mod syntax;


/// Transforms source code into typesetted documents.
///
/// Can be configured through various methods.
pub struct Typesetter<'p> {
    /// Font providers.
    font_providers: Vec<Box<dyn FontProvider + 'p>>,
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
            text_style: TextStyle::default(),
            page_style: PageStyle::default(),
            font_providers: vec![],
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
        self.font_providers.push(Box::new(provider));
    }

    /// Parse source code into a syntax tree.
    #[inline]
    pub fn parse(&self, src: &str) -> ParseResult<SyntaxTree> {
        let scope = Scope::with_std();
        let ctx = ParseContext { scope: &scope };
        parse(src, &ctx)
    }

    /// Layout a syntax tree and return the layout and the referenced font list.
    pub fn layout(&self, tree: &SyntaxTree) -> LayoutResult<(BoxLayout, Vec<Font>)> {
        let loader = FontLoader::new(&self.font_providers);
        let ctx = LayoutContext {
            loader: &loader,
            style: self.text_style.clone(),
            space: LayoutSpace {
                dimensions: self.page_style.dimensions,
                padding: self.page_style.margins,
            },
        };

        let pages = layout(&tree, &ctx)?;
        Ok((pages, loader.into_fonts()))
    }

    /// Typeset a portable document from source code.
    #[inline]
    pub fn typeset(&self, src: &str) -> Result<Document, TypesetError> {
        let tree = self.parse(src)?;
        let (layout, fonts) = self.layout(&tree)?;
        let document = layout.into_doc(fonts);
        Ok(document)
    }
}

impl Debug for Typesetter<'_> {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        f.debug_struct("Typesetter")
            .field("page_style", &self.page_style)
            .field("text_style", &self.text_style)
            .field("font_providers", &self.font_providers.len())
            .finish()
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
    use crate::font::FileSystemFontProvider;

    /// Create a _PDF_ with a name from the source code.
    fn test(name: &str, src: &str) {
        let mut typesetter = Typesetter::new();
        typesetter.add_font_provider(FileSystemFontProvider::new("../fonts", vec![
            ("CMU-SansSerif-Regular.ttf", font_info!(["Computer Modern", SansSerif])),
            ("CMU-SansSerif-Italic.ttf", font_info!(["Computer Modern", SansSerif], italic)),
            ("CMU-SansSerif-Bold.ttf", font_info!(["Computer Modern", SansSerif], bold)),
            ("CMU-SansSerif-Bold-Italic.ttf", font_info!(["Computer Modern", SansSerif], bold, italic)),
            ("CMU-Serif-Regular.ttf", font_info!(["Computer Modern", Serif])),
            ("CMU-Serif-Italic.ttf", font_info!(["Computer Modern", Serif], italic)),
            ("CMU-Serif-Bold.ttf", font_info!(["Computer Modern", Serif], bold)),
            ("CMU-Serif-Bold-Italic.ttf", font_info!(["Computer Modern", Serif], bold, italic)),
            ("CMU-Typewriter-Regular.ttf", font_info!(["Computer Modern", Monospace])),
            ("CMU-Typewriter-Italic.ttf", font_info!(["Computer Modern", Monospace], italic)),
            ("CMU-Typewriter-Bold.ttf", font_info!(["Computer Modern", Monospace], bold)),
            ("CMU-Typewriter-Bold-Italic.ttf", font_info!(["Computer Modern", Monospace], bold, italic)),
            ("NotoEmoji-Regular.ttf", font_info!(["NotoEmoji", "Noto", SansSerif, Serif, Monospace])),
        ]));

        // Typeset into document.
        let document = typesetter.typeset(src).unwrap();

        // Write to file.
        let path = format!("../target/typeset-unit-{}.pdf", name);
        let file = BufWriter::new(File::create(path).unwrap());
        let exporter = PdfExporter::new();
        exporter.export(&document, file).unwrap();
    }

    #[test]
    fn features() {
        test("features", r"
            **FEATURES TEST PAGE**

            __Multiline:__
            Lorem ipsum dolor sit amet, consetetur sadipscing elitr, sed diam nonumy
            eirmod tempor invidunt ut labore et dolore magna aliquyam erat, sed diam
            voluptua. At vero eos et accusam et justo duo dolores et ea rebum. Stet
            clita kasd gubergren, no sea takimata sanctus est.

            __Emoji:__ Hello World! 🌍

            __Styles:__ This is **bold** and that is __italic__!
        ");
    }

    #[test]
    fn wikipedia() {
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
