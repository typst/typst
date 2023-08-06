use std::cell::{OnceCell, RefCell, RefMut};
use std::collections::HashMap;
use std::fs;
use std::hash::Hash;
use std::path::{Path, PathBuf};

use chrono::{DateTime, Datelike, Local};
use comemo::Prehashed;
use same_file::Handle;
use siphasher::sip128::{Hasher128, SipHasher13};
use typst::diag::{FileError, FileResult, StrResult};
use typst::eval::{eco_format, Bytes, Datetime, Library};
use typst::font::{Font, FontBook};
use typst::syntax::{FileId, Source};
use typst::util::PathExt;
use typst::World;

use crate::args::SharedArgs;
use crate::fonts::{FontSearcher, FontSlot};
use crate::package::prepare_package;

/// A world that provides access to the operating system.
pub struct SystemWorld {
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
    paths: RefCell<HashMap<PathHash, PathSlot>>,
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
        let system_input = command.input.canonicalize().map_err(|_| {
            eco_format!("input file not found (searched at {})", command.input.display())
        })?;

        // Resolve the system-global root directory.
        let root = {
            let path = command
                .root
                .as_deref()
                .or_else(|| system_input.parent())
                .unwrap_or(Path::new("."));
            path.canonicalize().map_err(|_| {
                eco_format!("root directory not found (searched at {})", path.display())
            })?
        };

        // Resolve the input path within the project.
        let project_input = system_input
            .strip_prefix(&root)
            .map(|path| Path::new("/").join(path))
            .map_err(|_| "input file must be contained in project root")?;

        Ok(Self {
            root,
            main: FileId::new(None, &project_input),
            library: Prehashed::new(typst_library::build()),
            book: Prehashed::new(searcher.book),
            fonts: searcher.fonts,
            hashes: RefCell::default(),
            paths: RefCell::default(),
            now: OnceCell::new(),
        })
    }

    /// The id of the main source file.
    pub fn main(&self) -> FileId {
        self.main
    }

    /// Return all paths the last compilation depended on.
    pub fn dependencies(&mut self) -> impl Iterator<Item = &Path> {
        self.paths.get_mut().values().map(|slot| slot.system_path.as_path())
    }

    /// Reset the compilation state in preparation of a new compilation.
    pub fn reset(&mut self) {
        self.hashes.borrow_mut().clear();
        self.paths.borrow_mut().clear();
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
                let root = match id.package() {
                    Some(spec) => prepare_package(spec)?,
                    None => self.root.clone(),
                };

                // Join the path to the root. If it tries to escape, deny
                // access. Note: It can still escape via symlinks.
                system_path =
                    root.join_rooted(id.path()).ok_or(FileError::AccessDenied)?;

                PathHash::new(&system_path)
            })
            .clone()?;

        Ok(RefMut::map(self.paths.borrow_mut(), |paths| {
            paths.entry(hash).or_insert_with(|| PathSlot {
                id,
                // This will only trigger if the `or_insert_with` above also
                // triggered.
                system_path,
                source: OnceCell::new(),
                buffer: OnceCell::new(),
            })
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
    system_path: PathBuf,
    /// The lazily loaded source file for a path hash.
    source: OnceCell<FileResult<Source>>,
    /// The lazily loaded buffer for a path hash.
    buffer: OnceCell<FileResult<Bytes>>,
}

impl PathSlot {
    fn source(&self) -> FileResult<Source> {
        self.source
            .get_or_init(|| {
                let buf = read(&self.system_path)?;
                let text = decode_utf8(buf)?;
                Ok(Source::new(self.id, text))
            })
            .clone()
    }

    fn file(&self) -> FileResult<Bytes> {
        self.buffer
            .get_or_init(|| read(&self.system_path).map(Bytes::from))
            .clone()
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
fn decode_utf8(buf: Vec<u8>) -> FileResult<String> {
    Ok(if buf.starts_with(b"\xef\xbb\xbf") {
        // Remove UTF-8 BOM.
        std::str::from_utf8(&buf[3..])?.into()
    } else {
        // Assume UTF-8.
        String::from_utf8(buf)?
    })
}
