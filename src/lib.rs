//! Typeset is a library for compiling documents written in the
//! corresponding typesetting language into a typesetted document in an
//! output format like _PDF_.

mod pdf;
mod utility;
pub mod parsing;
pub mod doc;

/// Writing of documents into supported formats.
pub mod export {
    pub use crate::pdf::WritePdf;
}
