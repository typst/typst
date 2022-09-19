//! Resource loading.

#[cfg(feature = "fs")]
mod fs;
mod mem;

#[cfg(feature = "fs")]
pub use fs::*;
pub use mem::*;

use std::fmt::{self, Debug, Formatter};
use std::io;
use std::ops::Deref;
use std::path::Path;
use std::sync::Arc;

use crate::font::FontInfo;
use crate::util::Prehashed;

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
    fn load(&self, path: &Path) -> io::Result<Buffer>;
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

    fn load(&self, _: &Path) -> io::Result<Buffer> {
        Err(io::ErrorKind::NotFound.into())
    }
}

/// A shared buffer that is cheap to clone.
#[derive(Clone, Hash, Eq, PartialEq)]
pub struct Buffer(Prehashed<Arc<Vec<u8>>>);

impl Buffer {
    /// Return a view into the buffer.
    pub fn as_slice(&self) -> &[u8] {
        self
    }

    /// Return a copy of the buffer as a vector.
    pub fn to_vec(&self) -> Vec<u8> {
        self.0.to_vec()
    }
}

impl From<&[u8]> for Buffer {
    fn from(slice: &[u8]) -> Self {
        Self(Prehashed::new(Arc::new(slice.to_vec())))
    }
}

impl From<Vec<u8>> for Buffer {
    fn from(vec: Vec<u8>) -> Self {
        Self(Prehashed::new(Arc::new(vec)))
    }
}

impl From<Arc<Vec<u8>>> for Buffer {
    fn from(arc: Arc<Vec<u8>>) -> Self {
        Self(Prehashed::new(arc))
    }
}

impl Deref for Buffer {
    type Target = [u8];

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl AsRef<[u8]> for Buffer {
    fn as_ref(&self) -> &[u8] {
        self
    }
}

impl Debug for Buffer {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        f.pad("Buffer(..)")
    }
}
