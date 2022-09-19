//! Resource loading.

#[cfg(feature = "fs")]
mod fs;
mod mem;

#[cfg(feature = "fs")]
pub use fs::*;
pub use mem::*;

use std::io;
use std::path::Path;

use crate::font::FontInfo;

/// A hash that identifies a file.
///
/// Such a hash can be [resolved](Loader::resolve) from a path.
#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
pub struct FileHash(pub u64);

/// Loads resources from a local or remote source.
pub trait Loader {
    /// Descriptions of all fonts this loader serves.
    fn fonts(&self) -> &[FontInfo];

    /// Resolve a hash that is the same for this and all other paths pointing to
    /// the same file.
    fn resolve(&self, path: &Path) -> io::Result<FileHash>;

    /// Load a file from a path.
    fn load(&self, path: &Path) -> io::Result<Vec<u8>>;
}

/// A loader which serves nothing.
pub struct BlankLoader;

impl Loader for BlankLoader {
    fn fonts(&self) -> &[FontInfo] {
        &[]
    }

    fn resolve(&self, _: &Path) -> io::Result<FileHash> {
        Err(io::ErrorKind::NotFound.into())
    }

    fn load(&self, _: &Path) -> io::Result<Vec<u8>> {
        Err(io::ErrorKind::NotFound.into())
    }
}
