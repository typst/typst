//! Typeset is a library for compiling documents written in the
//! corresponding typesetting language into a typesetted document in an
//! output format like _PDF_.
//!
//! # Example
//! This is an example of compiling a really simple document into _PDF_.
//! ```
//! use typeset::{parsing::ParseTree, doc::Generate, write::WritePdf};
//!
//! // Create an output file.
//! # /*
//! let mut file = std::fs::File::create("hello-typeset.pdf").unwrap();
//! # */
//! # let mut file = std::fs::File::create("../target/typeset-hello.pdf").unwrap();
//!
//! // Parse the source and then generate the document.
//! let src = "Hello World from Typeset‼";
//! let doc = src.parse_tree().unwrap().generate().unwrap();
//!
//! // Write the document into file as PDF.
//! file.write_pdf(&doc).unwrap();
//! ```

mod pdf;
mod utility;
pub mod font;
pub mod parsing;
pub mod doc;

/// Writing of documents into supported formats.
pub mod write {
    pub use crate::pdf::{WritePdf, PdfWritingError};
}
