//! Resource loading.

#[cfg(feature = "fs")]
mod fs;

#[cfg(feature = "fs")]
pub use fs::*;

use std::rc::Rc;

use crate::font::FaceInfo;

/// A shared byte buffer.
pub type Buffer = Rc<Vec<u8>>;

/// Loads resources from a local or remote source.
pub trait Loader {
    /// Descriptions of all font faces this loader serves.
    fn faces(&self) -> &[FaceInfo];

    /// Load the font face with the given index in [`faces()`](Self::faces).
    fn load_face(&mut self, idx: usize) -> Option<Buffer>;

    /// Load a file from a path.
    fn load_file(&mut self, path: &str) -> Option<Buffer>;
}

/// A loader which serves nothing.
pub struct BlankLoader;

impl Loader for BlankLoader {
    fn faces(&self) -> &[FaceInfo] {
        &[]
    }

    fn load_face(&mut self, _: usize) -> Option<Buffer> {
        None
    }

    fn load_file(&mut self, _: &str) -> Option<Buffer> {
        None
    }
}
