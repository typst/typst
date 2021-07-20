//! Resource loading.

#[cfg(feature = "fs")]
mod fs;

#[cfg(feature = "fs")]
pub use fs::*;

use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::font::FaceInfo;

/// Loads resources from a local or remote source.
pub trait Loader {
    /// Descriptions of all font faces this loader serves.
    fn faces(&self) -> &[FaceInfo];

    /// Resolve a `path` relative to a `base` file.
    ///
    /// This should return the same id for all paths pointing to the same file
    /// and `None` if the file does not exist.
    fn resolve_from(&self, base: FileId, path: &Path) -> Option<FileId>;

    /// Load a file by id.
    fn load_file(&self, id: FileId) -> Option<Vec<u8>>;
}

/// A file id that can be [resolved](Loader::resolve_from) from a path.
///
/// Should be the same for all paths pointing to the same file.
#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash, Serialize, Deserialize)]
pub struct FileId(u64);

impl FileId {
    /// Create a file id from a raw value.
    pub fn from_raw(v: u64) -> Self {
        Self(v)
    }

    /// Convert into the raw underlying value.
    pub fn into_raw(self) -> u64 {
        self.0
    }
}

/// A loader which serves nothing.
pub struct BlankLoader;

impl Loader for BlankLoader {
    fn faces(&self) -> &[FaceInfo] {
        &[]
    }

    fn resolve_from(&self, _: FileId, _: &Path) -> Option<FileId> {
        None
    }

    fn load_file(&self, _: FileId) -> Option<Vec<u8>> {
        None
    }
}
