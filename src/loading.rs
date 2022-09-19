//! Resource loading.

use std::fmt::{self, Debug, Formatter};
use std::io;
use std::ops::Deref;
use std::path::Path;
use std::sync::Arc;

use crate::font::{Font, FontBook};
use crate::util::Prehashed;

/// A hash that identifies a file.
///
/// Such a hash can be [resolved](Loader::resolve) from a path.
#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
pub struct FileHash(pub u64);

/// Loads resources from a local or remote source.
pub trait Loader {
    /// Metadata about all known fonts.
    fn book(&self) -> &FontBook;

    /// Access the font with the given id.
    fn font(&self, id: usize) -> io::Result<Font>;

    /// Resolve a hash that is the same for this and all other paths pointing to
    /// the same file.
    fn resolve(&self, path: &Path) -> io::Result<FileHash>;

    /// Load a file from a path.
    fn file(&self, path: &Path) -> io::Result<Buffer>;
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

#[cfg(feature = "fs")]
pub use fs::*;

#[cfg(feature = "fs")]
mod fs {
    use std::fs::{self, File};
    use std::io;
    use std::path::{Path, PathBuf};

    use memmap2::Mmap;
    use same_file::Handle;
    use walkdir::WalkDir;

    use super::{Buffer, FileHash, Loader};
    use crate::font::{Font, FontBook, FontInfo};

    /// Loads fonts and files from the local file system.
    ///
    /// _This is only available when the `system` feature is enabled._
    pub struct FsLoader {
        book: FontBook,
        paths: Vec<(PathBuf, u32)>,
    }

    impl FsLoader {
        /// Create a new system loader.
        pub fn new() -> Self {
            Self { book: FontBook::new(), paths: vec![] }
        }

        /// Builder-style variant of [`search_path`](Self::search_path).
        pub fn with_path(mut self, dir: impl AsRef<Path>) -> Self {
            self.search_path(dir);
            self
        }

        /// Search for all fonts at a path.
        ///
        /// If the path is a directory, all contained fonts will be searched for
        /// recursively.
        pub fn search_path(&mut self, path: impl AsRef<Path>) {
            let walk = WalkDir::new(path)
                .follow_links(true)
                .sort_by(|a, b| a.file_name().cmp(b.file_name()))
                .into_iter()
                .filter_map(|e| e.ok());

            for entry in walk {
                let path = entry.path();
                if let Some(ext) = path.extension().and_then(|s| s.to_str()) {
                    if matches!(
                        ext,
                        "ttf" | "otf" | "TTF" | "OTF" | "ttc" | "otc" | "TTC" | "OTC",
                    ) {
                        self.search_file(path);
                    }
                }
            }
        }

        /// Index the fonts in the file at the given path.
        ///
        /// The file may form a font collection and contain multiple fonts,
        /// which will then all be indexed.
        fn search_file(&mut self, path: impl AsRef<Path>) {
            let path = path.as_ref();
            let path = path.strip_prefix(".").unwrap_or(path);
            if let Ok(file) = File::open(path) {
                if let Ok(mmap) = unsafe { Mmap::map(&file) } {
                    for (i, info) in FontInfo::from_data(&mmap).enumerate() {
                        self.book.push(info);
                        self.paths.push((path.into(), i as u32));
                    }
                }
            }
        }

        /// Builder-style variant of [`search_system`](Self::search_system).
        pub fn with_system(mut self) -> Self {
            self.search_system();
            self
        }

        /// Search for fonts in the operating system's font directories.
        pub fn search_system(&mut self) {
            self.search_system_impl();
        }

        #[cfg(all(unix, not(target_os = "macos")))]
        fn search_system_impl(&mut self) {
            self.search_path("/usr/share/fonts");
            self.search_path("/usr/local/share/fonts");

            if let Some(dir) = dirs::font_dir() {
                self.search_path(dir);
            }
        }

        #[cfg(target_os = "macos")]
        fn search_system_impl(&mut self) {
            self.search_path("/Library/Fonts");
            self.search_path("/Network/Library/Fonts");
            self.search_path("/System/Library/Fonts");

            if let Some(dir) = dirs::font_dir() {
                self.search_path(dir);
            }
        }

        #[cfg(windows)]
        fn search_system_impl(&mut self) {
            let windir =
                std::env::var("WINDIR").unwrap_or_else(|_| "C:\\Windows".to_string());

            self.search_path(Path::new(&windir).join("Fonts"));

            if let Some(roaming) = dirs::config_dir() {
                self.search_path(roaming.join("Microsoft\\Windows\\Fonts"));
            }

            if let Some(local) = dirs::cache_dir() {
                self.search_path(local.join("Microsoft\\Windows\\Fonts"));
            }
        }
    }

    impl Loader for FsLoader {
        fn book(&self) -> &FontBook {
            &self.book
        }

        fn font(&self, id: usize) -> io::Result<Font> {
            let (path, index) = &self.paths[id];
            let data = self.file(path)?;
            Font::new(data, *index).ok_or_else(|| io::ErrorKind::InvalidData.into())
        }

        fn resolve(&self, path: &Path) -> io::Result<FileHash> {
            let meta = fs::metadata(path)?;
            if meta.is_file() {
                let handle = Handle::from_path(path)?;
                Ok(FileHash(fxhash::hash64(&handle)))
            } else {
                Err(io::ErrorKind::NotFound.into())
            }
        }

        fn file(&self, path: &Path) -> io::Result<Buffer> {
            Ok(fs::read(path)?.into())
        }
    }
}
