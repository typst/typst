//! File loading and management.

use std::fs;
use std::mem;
use std::path::{Path, PathBuf};
use std::str;
use std::str::Utf8Error;

use parking_lot::Mutex;
use rustc_hash::FxHashMap;
use typst_library::diag::{FileError, FileResult};
use typst_library::foundations::Bytes;
use typst_syntax::{FileId, Source, VirtualPath};

#[cfg(feature = "system-files")]
use {crate::packages::SystemPackages, typst_syntax::VirtualRoot};

/// Holds loaded files and sources.
///
/// This type is backed by a file loader of your choosing. Internally, it
/// handles caching of loaded files and creation of Typst [sources](Source).
/// This is the right level of abstraction if you're building a Typst
/// integration that's concerned with providing input bytes on-demand, but does
/// not require tighter integration with Typst [`Source`s](Source). It is
/// appropriate for most clients.
///
/// If you need more control, you can skip this and implement custom logic that
/// directly handles the [`World::source`](typst_library::World::source) and
/// [`World::file`](typst_library::World::file) requests. A language server is
/// an example of an integration that might want to go even deeper,  to create,
/// manage, and edit source files by itself. If you go the manual route, ensure
/// that those methods are cheap on repeated calls (either through caching or by
/// virtue of always being cheap).
#[derive(Default)]
pub struct FileStore<L> {
    loader: L,
    slots: Mutex<FxHashMap<FileId, FileSlot>>,
}

impl<L> FileStore<L>
where
    L: FileLoader,
{
    /// Creates a new file store that loads file data via the provided `loader`.
    pub fn new(loader: L) -> Self {
        Self { loader, slots: Mutex::new(FxHashMap::default()) }
    }

    /// Returns a reference to the underlying loader.
    pub fn loader(&self) -> &L {
        &self.loader
    }

    /// Returns a mutable reference to the underlying loader.
    pub fn loader_mut(&mut self) -> &mut L {
        &mut self.loader
    }

    /// Drops the store, extracting the underlying loader.
    pub fn into_loader(self) -> L {
        self.loader
    }

    /// Retrieves the given file id as a Typst source.
    ///
    /// Can directly be used to implement
    /// [`World::source`](typst_library::World::source).
    pub fn source(&self, id: FileId) -> FileResult<Source> {
        self.slot(id, |slot| slot.source(&self.loader, id))
    }

    /// Retrieves the given file id as a raw file.
    ///
    /// Can directly be used to implement
    /// [`World::file`](typst_library::World::file).
    pub fn file(&self, id: FileId) -> FileResult<Bytes> {
        self.slot(id, |slot| slot.file(&self.loader, id))
    }

    /// Returns all files that were referenced since the last
    /// [`reset()`](Self::reset).
    ///
    /// Also returns a reference to the loader so that the IDs can be resolved
    /// with it. It couldn't be accessed through [`.loader()`](Self::loader)
    /// while iterating because of overlapping borrows.
    ///
    /// The dependencies are returned in arbitrary order! If you want to get a
    /// consistent result, you should sort them by a suitable criterion after
    /// the fact.
    pub fn dependencies(&mut self) -> (&L, impl Iterator<Item = FileId> + '_) {
        let iter = self
            .slots
            .get_mut()
            .iter()
            .filter(|(_, slot)| slot.accessed())
            .map(|(&id, _)| id);
        (&self.loader, iter)
    }

    /// Resets the store.
    ///
    /// This marks all loaded file as stale. On subsequent accesses, they will
    /// be loaded once more through the underlying loader. Moreover, calls to
    /// [`dependencies()`](Self::dependencies) will not yield files accessed
    /// before the call to `reset()`.
    ///
    /// Unlike when creating an entirely new store, source files will be edited
    /// in place with updated data, leading to improved incremental compilation
    /// performance.
    pub fn reset(&mut self) {
        #[allow(clippy::iter_over_hash_type, reason = "order does not matter")]
        for slot in self.slots.get_mut().values_mut() {
            slot.reset();
        }
    }

    /// Access the canonical slot for the given file id.
    fn slot<F, T>(&self, id: FileId, f: F) -> FileResult<T>
    where
        F: FnOnce(&mut FileSlot) -> FileResult<T>,
    {
        let mut map = self.slots.lock();
        f(map.entry(id).or_default())
    }
}

/// Holds the state for a file.
enum FileSlot {
    /// Nothing is loaded that, but we may have a stale source from before a
    /// reset (i.e. from an earlier compilation) that we can reuse and edit in
    /// place.
    ///
    /// Transitions to
    /// - loaded when a file is requested
    /// - to parsed if a source is requested and the data could be loaded
    ///   (otherwise to loaded).
    Empty(Stale<Source>),
    /// The slot has been requested as a `file()` but not as a `source()` (at
    /// least since the last reset). We can still have a stale, reusable source.
    ///
    /// Transitions to
    /// - parsed when a source is requested
    Loaded(FileResult<Bytes>, Stale<Source>),
    /// The slot has been requested as a `source()` and potentially as a
    /// `file()`.
    ///
    /// If possible, the bytes are backed by the source (via
    /// `Bytes::from_string(source)`) so that we can serve `file()` and
    /// `source()` requests from the same underlying data. Note that this is not
    /// possible if the data has a UTF8-BOM as it is stripped for the source,
    /// but should be retained in the file.
    Parsed(Result<Source, Utf8Error>, Bytes),
}

/// Holds a source that is not up to date, but may be updated to the newest
/// state for better incremental performance than parsing and numbering it from
/// scratch.
type Stale<T> = Option<T>;

impl FileSlot {
    /// Whether the slot has been accessed in any way since the last reset.
    fn accessed(&self) -> bool {
        !matches!(self, Self::Empty(_))
    }

    /// Resets the slot to its empty state.
    fn reset(&mut self) {
        let stale = match mem::take(self) {
            Self::Parsed(Ok(source), _) => Some(source),
            _ => None,
        };
        *self = Self::Empty(stale);
    }

    /// Retrieves the slot's bytes.
    fn file(&mut self, loader: &impl FileLoader, id: FileId) -> FileResult<Bytes> {
        match self {
            Self::Empty(stale) => {
                let result = loader.load(id);
                *self = Self::Loaded(result.clone(), mem::take(stale));
                result
            }
            Self::Loaded(result, _) => result.clone(),
            Self::Parsed(_, bytes) => Ok(bytes.clone()),
        }
    }

    /// Retrieves the source for this slot.
    fn source(&mut self, loader: &impl FileLoader, id: FileId) -> FileResult<Source> {
        // When we already have a source or error, this returns. Otherwise, it
        // loads or extracts the bytes and a potential stale source file.
        let (bytes, stale) = match self {
            Self::Empty(stale) => match loader.load(id) {
                Ok(bytes) => (bytes, mem::take(stale)),
                Err(err) => {
                    *self = Self::Loaded(Err(err.clone()), mem::take(stale));
                    return Err(err);
                }
            },
            Self::Loaded(Ok(_), _) => match mem::take(self) {
                Self::Loaded(Ok(bytes), stale) => (bytes, stale),
                _ => unreachable!(),
            },
            Self::Loaded(Err(err), _) => return Err(err.clone()),
            Self::Parsed(source, _) => return Ok(source.clone()?),
        };

        const UTF8_BOM: &[u8] = b"\xef\xbb\xbf";
        let without_bom = bytes.strip_prefix(UTF8_BOM);

        // Create a source file, with various attempts to reuse things.
        let (result, bytes) = if let Some(mut source) = stale {
            let result = str::from_utf8(without_bom.unwrap_or(&bytes)).map(|new| {
                // If we have a stale source file, reuse it.
                source.replace(new);
                source
            });
            (result, bytes)
        } else if let Some(rest) = without_bom {
            // If we had a BOM, we can't reuse the bytes for a string, so we
            // just create a source with a cloned string.
            (str::from_utf8(rest).map(|text| Source::new(id, text.into())), bytes)
        } else {
            // If we had no BOM, we attempt to reuse an existing `String` or
            // `Vec<u8>` within the `Bytes`, backing the `Bytes` with the
            // resulting `Source` instead. This way, we can transition from
            // a vector-backed file to a source without reallocating.
            match bytes.into_string().map(|text| Source::new(id, text)) {
                Ok(source) => (Ok(source.clone()), Bytes::from_string(source)),
                Err(err) => (Err(err.error), err.bytes),
            }
        };

        *self = Self::Parsed(result.clone(), bytes);
        Ok(result?)
    }
}

impl Default for FileSlot {
    fn default() -> Self {
        Self::Empty(None)
    }
}

/// Provides data for files, backing a [`FileStore`].
///
/// If you want to load files in a different way, the first step would be to
/// create your own type that implements [`FileLoader`]. For an example, you can
/// take a look at how `typst-cli` implements it.
///
/// If you need even more control, you can also skip the [`FileStore`] and
/// implement fully custom logic that directly handles the
/// [`World::source`](typst_library::World::source) and
/// [`World::file`](typst_library::World::file) requests.
pub trait FileLoader {
    /// Load the data for the given file ID.
    ///
    /// Generally, here you'll want to match on the
    /// [`root()`](typst_syntax::RootedPath::root) of the `id` to check whether
    /// the file should be loaded from the project or a package. Then, you'll
    /// load the data at the path
    /// [`id.vpath()`](typst_syntax::RootedPath::vpath) in the project /
    /// package.
    fn load(&self, id: FileId) -> FileResult<Bytes>;
}

/// Serves project files from a directory and package files from standard
/// locations.
///
/// With this implementation,
/// - project files are loaded from a project root directory through an
///   [`FsRoot`].
/// - package files are loaded from configured directories and/or the official
///   Typst Universe package registry via [`SystemPackages`].
#[cfg(feature = "system-files")]
pub struct SystemFiles {
    project: FsRoot,
    packages: SystemPackages,
}

#[cfg(feature = "system-files")]
impl SystemFiles {
    /// Creates a new instance with a given file system root for project files
    /// and the given configuration for system packages.
    pub fn new(project: FsRoot, packages: SystemPackages) -> Self {
        Self { project, packages }
    }

    /// Resolves the path of the given file `id` in the file system.
    pub fn resolve(&self, id: FileId) -> FileResult<PathBuf> {
        Ok(self.root(id)?.resolve(id.vpath()))
    }

    /// Resolves the root in which the given file ID resides.
    pub fn root(&self, id: FileId) -> FileResult<FsRoot> {
        Ok(match id.root() {
            VirtualRoot::Project => self.project.clone(),
            VirtualRoot::Package(spec) => self.packages.obtain(spec)?,
        })
    }
}

#[cfg(feature = "system-files")]
impl FileLoader for SystemFiles {
    fn load(&self, id: FileId) -> FileResult<Bytes> {
        self.root(id)?.load(id.vpath())
    }
}

/// A [root](typst_syntax::VirtualRoot) that is backed by a file system directory.
///
/// A Typst project forms a root. Similarly, each package has its own root.
/// Through this mechanism, projects and packages are isolated from each other.
#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub struct FsRoot(PathBuf);

impl FsRoot {
    /// Creates a new instance with the given root path.
    pub fn new(root: PathBuf) -> Self {
        Self(root)
    }

    /// The path at which the root resides in the file system.
    pub fn path(&self) -> &Path {
        &self.0
    }

    /// Resolves the real file system path for the given virtual path in this
    /// root.
    pub fn resolve(&self, path: &VirtualPath) -> PathBuf {
        path.realize(&self.0)
    }

    /// Loads file data from the given virtual path in this root.
    pub fn load(&self, path: &VirtualPath) -> FileResult<Bytes> {
        // Join the path to the root. If it tries to escape, deny access. Note:
        // It can still escape via symlinks.
        let path = self.resolve(path);
        let f = |e| FileError::from_io(e, &path);
        if fs::metadata(&path).map_err(f)?.is_dir() {
            Err(FileError::IsDirectory)
        } else {
            fs::read(&path).map(Bytes::new).map_err(f)
        }
    }
}

#[cfg(test)]
mod tests {
    use typst_syntax::{RootedPath, VirtualRoot};

    use super::*;

    /// Test that a file that's first been loaded as raw bytes correctly
    /// transitions into the source state.
    #[test]
    fn test_file_store_source_via_file() {
        let store = FileStore::new(TestLoader(1));
        store.file(id("a.typ")).must_be(A_TEXT);
        store.source(id("a.typ")).must_be(A_TEXT);
    }

    /// With BOM, the storage cannot be reused and the data differs.
    #[test]
    fn test_file_store_bom() {
        let store = FileStore::new(TestLoader(1));
        store.file(id("b.typ")).must_be(B_DATA);
        store.source(id("b.typ")).must_be(B_TEXT);
    }

    /// Here that a file request that's already been served as a source reuses
    /// the same underlying buffer.
    #[test]
    fn test_file_store_storage_reuse() {
        let store = FileStore::new(TestLoader(1));
        let a_source = store.source(id("a.typ")).unwrap();
        let a_file = store.file(id("a.typ")).unwrap();
        a_file.must_be(A_TEXT);
        a_source.must_be(A_TEXT);
        assert!(std::ptr::eq(a_file.as_slice().as_ptr(), a_source.text().as_ptr()));
    }

    /// Check that resetting reloads files.
    #[test]
    fn test_file_store_cycles() {
        let mut store = FileStore::new(TestLoader(1));
        let deps = |store: &mut FileStore<TestLoader>| {
            let (_, iter) = store.dependencies();
            let mut vec = iter
                .map(|id| id.get().vpath().get_without_slash())
                .collect::<Vec<_>>();
            vec.sort();
            vec
        };
        store.source(id("a.typ")).must_be(A_TEXT);
        store.source(id("d.typ")).must_be("1");
        assert_eq!(store.file(id("e.bin")), Err(FileError::NotFound("e.bin".into())));
        assert_eq!(deps(&mut store), ["a.typ", "d.typ", "e.bin"]);
        store.loader_mut().0 = 5;
        store.reset();
        store.source(id("d.typ")).must_be("5");
        store.file(id("e.bin")).must_be(E_TEXT);
        assert_eq!(deps(&mut store), ["d.typ", "e.bin"]);
    }

    const A_TEXT: &str = "Hello from A";
    const B_DATA: &[u8] = b"\xef\xbb\xbfHello from B";
    const B_TEXT: &str = "Hello from B";
    const C_DATA: &[u8] = b"a\xFF\xFF\xFFb";
    const E_TEXT: &str = "A secret";

    struct TestLoader(usize);

    impl FileLoader for TestLoader {
        fn load(&self, id: FileId) -> FileResult<Bytes> {
            Ok(match id.vpath().get_without_slash() {
                "a.typ" => Bytes::new(Vec::from(A_TEXT)),
                "b.typ" => Bytes::new(B_DATA),
                "c.bin" => Bytes::new(C_DATA),
                "d.typ" => Bytes::from_string(format!("{}", self.0)),
                "e.bin" if self.0 > 3 => Bytes::from_string(E_TEXT),
                path => return Err(FileError::NotFound(path.into())),
            })
        }
    }

    fn id(path: &str) -> FileId {
        RootedPath::new(VirtualRoot::Project, VirtualPath::new(path).unwrap()).intern()
    }

    trait OutputExt {
        fn must_be(&self, data: impl AsRef<[u8]>);
    }

    impl OutputExt for Source {
        #[track_caller]
        fn must_be(&self, data: impl AsRef<[u8]>) {
            assert_eq!(self.text().as_bytes(), data.as_ref());
        }
    }

    impl OutputExt for Bytes {
        #[track_caller]
        fn must_be(&self, data: impl AsRef<[u8]>) {
            assert_eq!(self.as_slice(), data.as_ref());
        }
    }

    impl<T: OutputExt> OutputExt for FileResult<T> {
        #[track_caller]
        fn must_be(&self, data: impl AsRef<[u8]>) {
            self.as_ref().unwrap().must_be(data);
        }
    }
}
