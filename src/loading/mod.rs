//! Resource loading.

#[cfg(feature = "fs")]
mod fs;

#[cfg(feature = "fs")]
pub use fs::*;

use std::path::Path;
use std::rc::Rc;

use crate::font::FaceInfo;

/// A shared byte buffer.
pub type Buffer = Rc<Vec<u8>>;

/// Loads resources from a local or remote source.
pub trait Loader {
    /// Descriptions of all font faces this loader serves.
    fn faces(&self) -> &[FaceInfo];

    /// Resolve a hash that is the same for all paths pointing to the same file.
    ///
    /// Should return `None` if the file does not exist.
    fn resolve(&self, path: &Path) -> Option<FileHash>;

    /// Load the font face with the given index in [`faces()`](Self::faces).
    fn load_face(&mut self, idx: usize) -> Option<Buffer>;

    /// Load a file from a path.
    fn load_file(&mut self, path: &Path) -> Option<Buffer>;
}

/// A hash that must be the same for all paths pointing to the same file.
#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
pub struct FileHash(pub u64);

/// A loader which serves nothing.
pub struct BlankLoader;

impl Loader for BlankLoader {
    fn faces(&self) -> &[FaceInfo] {
        &[]
    }

    fn resolve(&self, _: &Path) -> Option<FileHash> {
        None
    }

    fn load_face(&mut self, _: usize) -> Option<Buffer> {
        None
    }

    fn load_file(&mut self, _: &Path) -> Option<Buffer> {
        None
    }
}
