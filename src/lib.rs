//! Typeset is a library for compiling documents written in the
//! corresponding typesetting language into a typesetted document in an
//! output format like _PDF_.
//!
//! # Example
//! This is an example of compiling a really simple document into _PDF_.
//! ```
//! use typeset::Compiler;
//!
//! // Create an output file.
//! # /*
//! let mut file = std::fs::File::create("hello-typeset.pdf").unwrap();
//! # */
//! # let mut file = std::fs::File::create("../target/typeset-hello.pdf").unwrap();
//!
//! // Create a compiler and export a PDF.
//! let src = "Hello World from Typeset!";
//! let compiler = Compiler::new(src);
//!
//! // Write the document into a file as a PDF.
//! compiler.write_pdf(&mut file).unwrap();
//! ```

pub mod syntax;
pub mod doc;
pub mod font;
mod parsing;
mod engine;
mod pdf;
mod utility;

pub use crate::parsing::{Tokens, Parser, ParseError};
pub use crate::engine::{Engine, TypesetError};
pub use crate::pdf::{PdfCreator, PdfWritingError};

use std::error;
use std::fmt;
use std::io::Write;
use crate::syntax::SyntaxTree;
use crate::doc::Document;


/// Emits various compiled intermediates from source code.
pub struct Compiler<'s> {
    /// The source code of the document.
    source: &'s str,
}

impl<'s> Compiler<'s> {
    /// Create a new compiler from a document.
    #[inline]
    pub fn new(source: &'s str) -> Compiler<'s> {
        Compiler { source }
    }

    /// Return an iterator over the tokens of the document.
    #[inline]
    pub fn tokenize(&self) -> Tokens<'s> {
        Tokens::new(self.source)
    }

    /// Return the abstract syntax tree representation of the document.
    #[inline]
    pub fn parse(&self) -> Result<SyntaxTree<'s>, Error> {
        Parser::new(self.tokenize()).parse().map_err(Into::into)
    }

    /// Return the abstract typesetted representation of the document.
    #[inline]
    pub fn typeset(&self) -> Result<Document, Error> {
        Engine::new(self.parse()?).typeset().map_err(Into::into)
    }

    /// Write the document as a _PDF_, returning how many bytes were written.
    pub fn write_pdf<W: Write>(&self, target: &mut W) -> Result<usize, Error> {
        PdfCreator::new(target, &self.typeset()?)?.write().map_err(Into::into)
    }
}

/// The error type for compilation.
#[derive(Debug, Clone, Eq, PartialEq)]
pub enum Error {
    /// An error that occured while transforming source code into
    /// an abstract syntax tree.
    Parse(ParseError),
    /// An error that occured while typesetting into an abstract document.
    Typeset(TypesetError),
    /// An error that occured while writing the document as a _PDF_.
    PdfWrite(PdfWritingError)
}

impl error::Error for Error {
    #[inline]
    fn source(&self) -> Option<&(dyn error::Error + 'static)> {
        match self {
            Error::Parse(err) => Some(err),
            Error::Typeset(err) => Some(err),
            Error::PdfWrite(err) => Some(err),
        }
    }
}

impl fmt::Display for Error {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Error::Parse(err) => write!(f, "parse error: {}", err),
            Error::Typeset(err) => write!(f, "typeset error: {}", err),
            Error::PdfWrite(err) => write!(f, "typeset error: {}", err),
        }
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

impl From<PdfWritingError> for Error {
    #[inline]
    fn from(err: PdfWritingError) -> Error {
        Error::PdfWrite(err)
    }
}


#[cfg(test)]
mod test {
    use crate::Compiler;

    /// Create a pdf with a name from the source code.
    fn test(name: &str, src: &str) {
        let path = format!("../target/typeset-pdf-{}.pdf", name);
        let mut file = std::fs::File::create(path).unwrap();
        Compiler::new(src).write_pdf(&mut file).unwrap();
    }

    #[test]
    fn pdf() {
        test("unicode", "∑mbe∂∂ed font with Unicode!");
        test("parentheses", "Text with ) and ( or (enclosed) works.");
        test("composite-glyph", "Composite character‼");
        test("multiline","
             Lorem ipsum dolor sit amet, consetetur sadipscing elitr, sed
             diam nonumy eirmod tempor invidunt ut labore et dolore magna aliquyam erat, sed
             diam voluptua. At vero eos et accusam et justo duo dolores et ea rebum.
             Stet clita kasd gubergren, no sea takimata sanctus est.
        ");
    }
}
