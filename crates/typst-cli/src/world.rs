use std::cell::{Cell, OnceCell, RefCell, RefMut};
use std::collections::HashMap;
use std::fs;
use std::hash::Hash;
use std::path::{Path, PathBuf};

use chrono::{DateTime, Datelike, Local};
use comemo::Prehashed;
use filetime::FileTime;
use same_file::Handle;
use siphasher::sip128::{Hasher128, SipHasher13};
use typst::diag::{FileError, FileResult, StrResult};
use typst::eval::{eco_format, Bytes, Datetime, Library};
use typst::font::{Font, FontBook};
use typst::syntax::{FileId, Source, VirtualPath};
use typst::World;

use crate::args::SharedArgs;
use crate::fonts::{FontSearcher, FontSlot};
use crate::package::prepare_package;

/// A world that provides access to the operating system.
pub struct SystemWorld {
    /// The working directory.
    workdir: Option<PathBuf>,
    /// The root relative to which absolute paths are resolved.
    root: PathBuf,
    /// The input path.
    main: FileId,
    /// Typst's standard library.
    library: Prehashed<Library>,
    /// Metadata about discovered fonts.
    book: Prehashed<FontBook>,
    /// Locations of and storage for lazily loaded fonts.
    fonts: Vec<FontSlot>,
    /// Maps package-path combinations to canonical hashes. All package-path
    /// combinations that point to the same file are mapped to the same hash. To
    /// be used in conjunction with `paths`.
    hashes: RefCell<HashMap<FileId, FileResult<PathHash>>>,
    /// Maps canonical path hashes to source files and buffers.
    slots: RefCell<HashMap<PathHash, PathSlot>>,
    /// The current datetime if requested. This is stored here to ensure it is
    /// always the same within one compilation. Reset between compilations.
    now: OnceCell<DateTime<Local>>,
}

impl SystemWorld {
    /// Create a new system world.
    pub fn new(command: &SharedArgs) -> StrResult<Self> {
        let mut searcher = FontSearcher::new();
        searcher.search(&command.font_paths);

        // Resolve the system-global input path.
        let input = command.input.canonicalize().map_err(|_| {
            eco_format!("input file not found (searched at {})", command.input.display())
        })?;

        // Resolve the system-global root directory.
        let root = {
            let path = command
                .root
                .as_deref()
                .or_else(|| input.parent())
                .unwrap_or(Path::new("."));
            path.canonicalize().map_err(|_| {
                eco_format!("root directory not found (searched at {})", path.display())
            })?
        };

        // Resolve the virtual path of the main file within the project root.
        let main_path = VirtualPath::within_root(&input, &root)
            .ok_or("input file must be contained in project root")?;

        Ok(Self {
            workdir: std::env::current_dir().ok(),
            root,
            main: FileId::new(None, main_path),
            library: Prehashed::new(typst_library::build()),
            book: Prehashed::new(searcher.book),
            fonts: searcher.fonts,
            hashes: RefCell::default(),
            slots: RefCell::default(),
            now: OnceCell::new(),
        })
    }

    /// The id of the main source file.
    pub fn main(&self) -> FileId {
        self.main
    }

    /// The root relative to which absolute paths are resolved.
    pub fn root(&self) -> &Path {
        &self.root
    }

    /// The current working directory.
    pub fn workdir(&self) -> &Path {
        self.workdir.as_deref().unwrap_or(Path::new("."))
    }

    /// Return all paths the last compilation depended on.
    pub fn dependencies(&mut self) -> impl Iterator<Item = &Path> {
        self.slots
            .get_mut()
            .values()
            .filter(|slot| slot.accessed())
            .map(|slot| slot.path.as_path())
    }

    /// Reset the compilation state in preparation of a new compilation.
    pub fn reset(&mut self) {
        self.hashes.borrow_mut().clear();
        for slot in self.slots.borrow_mut().values_mut() {
            slot.reset();
        }
        self.now.take();
    }

    /// Lookup a source file by id.
    #[track_caller]
    pub fn lookup(&self, id: FileId) -> Source {
        self.source(id).expect("file id does not point to any source file")
    }
}

impl World for SystemWorld {
    fn library(&self) -> &Prehashed<Library> {
        &self.library
    }

    fn book(&self) -> &Prehashed<FontBook> {
        &self.book
    }

    fn main(&self) -> Source {
        self.source(self.main).unwrap()
    }

    fn source(&self, id: FileId) -> FileResult<Source> {
        self.slot(id)?.source()
    }

    fn file(&self, id: FileId) -> FileResult<Bytes> {
        self.slot(id)?.file()
    }

    fn font(&self, index: usize) -> Option<Font> {
        self.fonts[index].get()
    }

    fn today(&self, offset: Option<i64>) -> Option<Datetime> {
        let now = self.now.get_or_init(chrono::Local::now);

        let naive = match offset {
            None => now.naive_local(),
            Some(o) => now.naive_utc() + chrono::Duration::hours(o),
        };

        Datetime::from_ymd(
            naive.year(),
            naive.month().try_into().ok()?,
            naive.day().try_into().ok()?,
        )
    }
}

impl SystemWorld {
    /// Access the canonical slot for the given file id.
    #[tracing::instrument(skip_all)]
    fn slot(&self, id: FileId) -> FileResult<RefMut<PathSlot>> {
        let mut system_path = PathBuf::new();
        let hash = self
            .hashes
            .borrow_mut()
            .entry(id)
            .or_insert_with(|| {
                // Determine the root path relative to which the file path
                // will be resolved.
                let buf;
                let mut root = &self.root;
                if let Some(spec) = id.package() {
                    buf = prepare_package(spec)?;
                    root = &buf;
                }

                // Join the path to the root. If it tries to escape, deny
                // access. Note: It can still escape via symlinks.
                system_path = id.vpath().resolve(root).ok_or(FileError::AccessDenied)?;

                PathHash::new(&system_path)
            })
            .clone()?;

        Ok(RefMut::map(self.slots.borrow_mut(), |paths| {
            paths.entry(hash).or_insert_with(|| PathSlot::new(id, system_path))
        }))
    }
}

/// Holds canonical data for all paths pointing to the same entity.
///
/// Both fields can be populated if the file is both imported and read().
struct PathSlot {
    /// The slot's canonical file id.
    id: FileId,
    /// The slot's path on the system.
    path: PathBuf,
    /// The lazily loaded and incrementally updated source file.
    source: SlotCell<Source>,
    /// The lazily loaded raw byte buffer.
    file: SlotCell<Bytes>,
}

impl PathSlot {
    /// Create a new path slot.
    fn new(id: FileId, path: PathBuf) -> Self {
        Self {
            id,
            path,
            file: SlotCell::new(),
            source: SlotCell::new(),
        }
    }

    /// Whether the file was accessed in the ongoing compilation.
    fn accessed(&self) -> bool {
        self.source.accessed() || self.file.accessed()
    }

    /// Marks the file as not yet accessed in preparation of the next
    /// compilation.
    fn reset(&self) {
        self.source.reset();
        self.file.reset();
    }

    /// Retrieve the source for this file.
    fn source(&self) -> FileResult<Source> {
        self.source.get_or_init(&self.path, |data, prev| {
            let text = decode_utf8(&data)?;
            if let Some(mut prev) = prev {
                prev.replace(text);
                Ok(prev)
            } else {
                Ok(Source::new(self.id, text.into()))
            }
        })
    }

    /// Retrieve the file's bytes.
    fn file(&self) -> FileResult<Bytes> {
        self.file.get_or_init(&self.path, |data, _| Ok(data.into()))
    }
}

/// Lazily processes data for a file.
struct SlotCell<T> {
    data: RefCell<Option<FileResult<T>>>,
    refreshed: Cell<FileTime>,
    accessed: Cell<bool>,
}

impl<T: Clone> SlotCell<T> {
    /// Creates a new, empty cell.
    fn new() -> Self {
        Self {
            data: RefCell::new(None),
            refreshed: Cell::new(FileTime::zero()),
            accessed: Cell::new(false),
        }
    }

    /// Whether the cell was accessed in the ongoing compilation.
    fn accessed(&self) -> bool {
        self.accessed.get()
    }

    /// Marks the cell as not yet accessed in preparation of the next
    /// compilation.
    fn reset(&self) {
        self.accessed.set(false);
    }

    /// Gets the contents of the cell or initialize them.
    fn get_or_init(
        &self,
        path: &Path,
        f: impl FnOnce(Vec<u8>, Option<T>) -> FileResult<T>,
    ) -> FileResult<T> {
        let mut borrow = self.data.borrow_mut();
        if let Some(data) = &*borrow {
            if self.accessed.replace(true) || self.current(path) {
                return data.clone();
            }
        }

        self.accessed.set(true);
        self.refreshed.set(FileTime::now());
        let prev = borrow.take().and_then(Result::ok);
        let value = read(path).and_then(|data| f(data, prev));
        *borrow = Some(value.clone());
        value
    }

    /// Whether the cell contents are still up to date with the file system.
    fn current(&self, path: &Path) -> bool {
        fs::metadata(path).map_or(false, |meta| {
            let modified = FileTime::from_last_modification_time(&meta);
            modified < self.refreshed.get()
        })
    }
}

/// A hash that is the same for all paths pointing to the same entity.
#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
struct PathHash(u128);

impl PathHash {
    fn new(path: &Path) -> FileResult<Self> {
        let f = |e| FileError::from_io(e, path);
        let handle = Handle::from_path(path).map_err(f)?;
        let mut state = SipHasher13::new();
        handle.hash(&mut state);
        Ok(Self(state.finish128().as_u128()))
    }
}

/// Read a file.
fn read(path: &Path) -> FileResult<Vec<u8>> {
    let f = |e| FileError::from_io(e, path);
    if fs::metadata(path).map_err(f)?.is_dir() {
        Err(FileError::IsDirectory)
    } else {
        fs::read(path).map_err(f)
    }
}

/// Decode UTF-8 with an optional BOM.
fn decode_utf8(buf: &[u8]) -> FileResult<&str> {
    // Remove UTF-8 BOM.
    Ok(std::str::from_utf8(buf.strip_prefix(b"\xef\xbb\xbf").unwrap_or(buf))?)
}
