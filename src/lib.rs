//! Typeset is a library for compiling documents written in the
//! corresponding typesetting language into a typesetted document in an
//! output format like _PDF_.
//!
//! # Example
//! This is an example of compiling a _really_ simple document into _PDF_.
//! ```
//! use typeset::{parsing::{Tokenize, Parse}, doc::Generate, export::WritePdf};
//!
//! let path = "hello-typeset.pdf";
//! # let path = "../target/hello-typeset.pdf";
//! let mut file = std::fs::File::create(path).unwrap();
//!
//! // Tokenize, parse and then generate the document.
//! let src = "Hello World from Typeset!";
//! let doc = src.tokenize()
//!     .parse().unwrap()
//!     .generate().unwrap();
//!
//! file.write_pdf(&doc).unwrap();
//! ```

mod pdf;
mod utility;
pub mod parsing;
pub mod doc;

/// Writing of documents into supported formats.
pub mod export {
    pub use crate::pdf::WritePdf;
}
