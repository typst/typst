use std::cell::{Cell, OnceCell, RefCell, RefMut};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

use chrono::{DateTime, Datelike, Local};
use comemo::Prehashed;
use ecow::eco_format;
use typst::diag::{FileError, FileResult, StrResult};
use typst::foundations::{Bytes, Datetime};
use typst::layout::Frame;
use typst::syntax::{FileId, Source, VirtualPath};
use typst::text::{Font, FontBook};
use typst::util::hash128;
use typst::{Library, World};

use crate::args::SharedArgs;
use crate::fonts::{FontSearcher, FontSlot};
use crate::package::prepare_package;

/// A world that provides access to the operating system.
pub struct SystemWorld {
    /// The working directory.
    workdir: Option<PathBuf>,
    /// The canonical path to the input file.
    input: PathBuf,
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
    /// Maps file ids to source files and buffers.
    slots: RefCell<HashMap<FileId, FileSlot>>,
    /// The current datetime if requested. This is stored here to ensure it is
    /// always the same within one compilation. Reset between compilations.
    now: OnceCell<DateTime<Local>>,
    /// The export cache, used for caching output files in `typst watch`
    /// sessions.
    export_cache: ExportCache,
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
            input,
            root,
            main: FileId::new(None, main_path),
            library: Prehashed::new(Library::build()),
            book: Prehashed::new(searcher.book),
            fonts: searcher.fonts,
            slots: RefCell::default(),
            now: OnceCell::new(),
            export_cache: ExportCache::new(),
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
    pub fn dependencies(&mut self) -> impl Iterator<Item = PathBuf> + '_ {
        self.slots
            .get_mut()
            .values()
            .filter(|slot| slot.accessed())
            .filter_map(|slot| slot.system_path(&self.root).ok())
    }

    /// Reset the compilation state in preparation of a new compilation.
    pub fn reset(&mut self) {
        for slot in self.slots.get_mut().values_mut() {
            slot.reset();
        }
        self.now.take();
    }

    /// Return the canonical path to the input file.
    pub fn input(&self) -> &PathBuf {
        &self.input
    }

    /// Lookup a source file by id.
    #[track_caller]
    pub fn lookup(&self, id: FileId) -> Source {
        self.source(id).expect("file id does not point to any source file")
    }

    /// Gets access to the export cache.
    pub fn export_cache(&mut self) -> &mut ExportCache {
        &mut self.export_cache
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
        self.slot(id)?.source(&self.root)
    }

    fn file(&self, id: FileId) -> FileResult<Bytes> {
        self.slot(id)?.file(&self.root)
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
    fn slot(&self, id: FileId) -> FileResult<RefMut<FileSlot>> {
        Ok(RefMut::map(self.slots.borrow_mut(), |slots| {
            slots.entry(id).or_insert_with(|| FileSlot::new(id))
        }))
    }
}

/// Holds the processed data for a file ID.
///
/// Both fields can be populated if the file is both imported and read().
struct FileSlot {
    /// The slot's file id.
    id: FileId,
    /// The lazily loaded and incrementally updated source file.
    source: SlotCell<Source>,
    /// The lazily loaded raw byte buffer.
    file: SlotCell<Bytes>,
}

impl FileSlot {
    /// Create a new path slot.
    fn new(id: FileId) -> Self {
        Self { id, file: SlotCell::new(), source: SlotCell::new() }
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
    fn source(&self, root: &Path) -> FileResult<Source> {
        self.source.get_or_init(
            || self.system_path(root),
            |data, prev| {
                let text = decode_utf8(&data)?;
                if let Some(mut prev) = prev {
                    prev.replace(text);
                    Ok(prev)
                } else {
                    Ok(Source::new(self.id, text.into()))
                }
            },
        )
    }

    /// Retrieve the file's bytes.
    fn file(&self, root: &Path) -> FileResult<Bytes> {
        self.file
            .get_or_init(|| self.system_path(root), |data, _| Ok(data.into()))
    }

    /// The path of the slot on the system.
    fn system_path(&self, root: &Path) -> FileResult<PathBuf> {
        // Determine the root path relative to which the file path
        // will be resolved.
        let buf;
        let mut root = root;
        if let Some(spec) = self.id.package() {
            buf = prepare_package(spec)?;
            root = &buf;
        }

        // Join the path to the root. If it tries to escape, deny
        // access. Note: It can still escape via symlinks.
        self.id.vpath().resolve(root).ok_or(FileError::AccessDenied)
    }
}

/// Lazily processes data for a file.
struct SlotCell<T> {
    /// The processed data.
    data: RefCell<Option<FileResult<T>>>,
    /// A hash of the raw file contents / access error.
    fingerprint: Cell<u128>,
    /// Whether the slot has been accessed in the current compilation.
    accessed: Cell<bool>,
}

impl<T: Clone> SlotCell<T> {
    /// Creates a new, empty cell.
    fn new() -> Self {
        Self {
            data: RefCell::new(None),
            fingerprint: Cell::new(0),
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
        path: impl FnOnce() -> FileResult<PathBuf>,
        f: impl FnOnce(Vec<u8>, Option<T>) -> FileResult<T>,
    ) -> FileResult<T> {
        let mut borrow = self.data.borrow_mut();

        // If we accessed the file already in this compilation, retrieve it.
        if self.accessed.replace(true) {
            if let Some(data) = &*borrow {
                return data.clone();
            }
        }

        // Read and hash the file.
        let result = path().and_then(|p| read(&p));
        let fingerprint = typst::util::hash128(&result);

        // If the file contents didn't change, yield the old processed data.
        if self.fingerprint.replace(fingerprint) == fingerprint {
            if let Some(data) = &*borrow {
                return data.clone();
            }
        }

        let prev = borrow.take().and_then(Result::ok);
        let value = result.and_then(|data| f(data, prev));
        *borrow = Some(value.clone());

        value
    }
}

/// Caches exported files so that we can avoid re-exporting them if they haven't
/// changed.
///
/// This is done by having a list of size `files.len()` that contains the hashes
/// of the last rendered frame in each file. If a new frame is inserted, this
/// will invalidate the rest of the cache, this is deliberate as to decrease the
/// complexity and memory usage of such a cache.
pub struct ExportCache {
    /// The hashes of last compilation's frames.
    pub cache: Vec<u128>,
}

impl ExportCache {
    /// Creates a new export cache.
    pub fn new() -> Self {
        Self { cache: Vec::with_capacity(32) }
    }

    /// Returns true if the entry is cached and appends the new hash to the
    /// cache (for the next compilation).
    pub fn is_cached(&mut self, i: usize, frame: &Frame) -> bool {
        let hash = hash128(frame);

        if i >= self.cache.len() {
            self.cache.push(hash);
            return false;
        }

        std::mem::replace(&mut self.cache[i], hash) == hash
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
