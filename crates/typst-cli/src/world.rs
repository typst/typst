use std::collections::HashMap;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::sync::{LazyLock, OnceLock};
use std::{fmt, fs, io, mem};

use chrono::{DateTime, Datelike, FixedOffset, Local, Utc};
use ecow::{eco_format, EcoString};
use parking_lot::Mutex;
use typst::diag::{FileError, FileResult};
use typst::foundations::{Bytes, Datetime, Dict, IntoValue};
use typst::syntax::{FileId, Lines, Source, VirtualPath};
use typst::text::{Font, FontBook};
use typst::utils::LazyHash;
use typst::{Library, World};
use typst_kit::fonts::{FontSlot, Fonts};
use typst_kit::package::PackageStorage;
use typst_timing::timed;

use crate::args::{Feature, Input, ProcessArgs, WorldArgs};
use crate::download::PrintDownload;
use crate::package;

/// Static `FileId` allocated for stdin.
/// This is to ensure that a file is read in the correct way.
static STDIN_ID: LazyLock<FileId> =
    LazyLock::new(|| FileId::new_fake(VirtualPath::new("<stdin>")));

/// A world that provides access to the operating system.
pub struct SystemWorld {
    /// The working directory.
    workdir: Option<PathBuf>,
    /// The root relative to which absolute paths are resolved.
    root: PathBuf,
    /// The input path.
    main: FileId,
    /// Typst's standard library.
    library: LazyHash<Library>,
    /// Metadata about discovered fonts.
    book: LazyHash<FontBook>,
    /// Locations of and storage for lazily loaded fonts.
    fonts: Vec<FontSlot>,
    /// Maps file ids to source files and buffers.
    slots: Mutex<HashMap<FileId, FileSlot>>,
    /// Holds information about where packages are stored.
    package_storage: PackageStorage,
    /// The current datetime if requested. This is stored here to ensure it is
    /// always the same within one compilation.
    /// Reset between compilations if not [`Now::Fixed`].
    now: Now,
}

impl SystemWorld {
    /// Create a new system world.
    pub fn new(
        input: &Input,
        world_args: &WorldArgs,
        process_args: &ProcessArgs,
    ) -> Result<Self, WorldCreationError> {
        // Set up the thread pool.
        if let Some(jobs) = process_args.jobs {
            rayon::ThreadPoolBuilder::new()
                .num_threads(jobs)
                .use_current_thread()
                .build_global()
                .ok();
        }

        // Resolve the system-global input path.
        let input = match input {
            Input::Stdin => None,
            Input::Path(path) => {
                Some(path.canonicalize().map_err(|err| match err.kind() {
                    io::ErrorKind::NotFound => {
                        WorldCreationError::InputNotFound(path.clone())
                    }
                    _ => WorldCreationError::Io(err),
                })?)
            }
        };

        // Resolve the system-global root directory.
        let root = {
            let path = world_args
                .root
                .as_deref()
                .or_else(|| input.as_deref().and_then(|i| i.parent()))
                .unwrap_or(Path::new("."));
            path.canonicalize().map_err(|err| match err.kind() {
                io::ErrorKind::NotFound => {
                    WorldCreationError::RootNotFound(path.to_path_buf())
                }
                _ => WorldCreationError::Io(err),
            })?
        };

        let main = if let Some(path) = &input {
            // Resolve the virtual path of the main file within the project root.
            let main_path = VirtualPath::within_root(path, &root)
                .ok_or(WorldCreationError::InputOutsideRoot)?;
            FileId::new(None, main_path)
        } else {
            // Return the special id of STDIN otherwise
            *STDIN_ID
        };

        let library = {
            // Convert the input pairs to a dictionary.
            let inputs: Dict = world_args
                .inputs
                .iter()
                .map(|(k, v)| (k.as_str().into(), v.as_str().into_value()))
                .collect();

            let features = process_args
                .features
                .iter()
                .map(|&feature| match feature {
                    Feature::Html => typst::Feature::Html,
                })
                .collect();

            Library::builder().with_inputs(inputs).with_features(features).build()
        };

        let fonts = Fonts::searcher()
            .include_system_fonts(!world_args.font.ignore_system_fonts)
            .search_with(&world_args.font.font_paths);

        let now = match world_args.creation_timestamp {
            Some(time) => Now::Fixed(time),
            None => Now::System(OnceLock::new()),
        };

        Ok(Self {
            workdir: std::env::current_dir().ok(),
            root,
            main,
            library: LazyHash::new(library),
            book: LazyHash::new(fonts.book),
            fonts: fonts.fonts,
            slots: Mutex::new(HashMap::new()),
            package_storage: package::storage(&world_args.package),
            now,
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
            .filter_map(|slot| {
                system_path(&self.root, slot.id, &self.package_storage).ok()
            })
    }

    /// Reset the compilation state in preparation of a new compilation.
    pub fn reset(&mut self) {
        for slot in self.slots.get_mut().values_mut() {
            slot.reset();
        }
        if let Now::System(time_lock) = &mut self.now {
            time_lock.take();
        }
    }

    /// Lookup a source file by id.
    #[track_caller]
    pub fn lookup(&self, id: FileId) -> Lines<String> {
        self.slot(id, |slot| {
            if let Some(source) = slot.source.get() {
                let source = source.as_ref().expect("file is not valid");
                source.lines()
            } else if let Some(bytes) = slot.file.get() {
                let bytes = bytes.as_ref().expect("file is not valid");
                Lines::from_bytes(bytes.as_slice()).expect("file is not valid utf-8")
            } else {
                panic!("file id does not point to any source file");
            }
        })
    }
}

impl World for SystemWorld {
    fn library(&self) -> &LazyHash<Library> {
        &self.library
    }

    fn book(&self) -> &LazyHash<FontBook> {
        &self.book
    }

    fn main(&self) -> FileId {
        self.main
    }

    fn source(&self, id: FileId) -> FileResult<Source> {
        self.slot(id, |slot| slot.source(&self.root, &self.package_storage))
    }

    fn file(&self, id: FileId) -> FileResult<Bytes> {
        self.slot(id, |slot| slot.file(&self.root, &self.package_storage))
    }

    fn font(&self, index: usize) -> Option<Font> {
        // comemo's validation may invoke this function with an invalid index. This is
        // impossible in typst-cli but possible if a custom tool mutates the fonts.
        self.fonts.get(index)?.get()
    }

    fn today(&self, offset: Option<i64>) -> Option<Datetime> {
        let now = match &self.now {
            Now::Fixed(time) => time,
            Now::System(time) => time.get_or_init(Utc::now),
        };

        // The time with the specified UTC offset, or within the local time zone.
        let with_offset = match offset {
            None => now.with_timezone(&Local).fixed_offset(),
            Some(hours) => {
                let seconds = i32::try_from(hours).ok()?.checked_mul(3600)?;
                now.with_timezone(&FixedOffset::east_opt(seconds)?)
            }
        };

        Datetime::from_ymd(
            with_offset.year(),
            with_offset.month().try_into().ok()?,
            with_offset.day().try_into().ok()?,
        )
    }
}

impl SystemWorld {
    /// Access the canonical slot for the given file id.
    fn slot<F, T>(&self, id: FileId, f: F) -> T
    where
        F: FnOnce(&mut FileSlot) -> T,
    {
        let mut map = self.slots.lock();
        f(map.entry(id).or_insert_with(|| FileSlot::new(id)))
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
    /// Create a new file slot.
    fn new(id: FileId) -> Self {
        Self { id, file: SlotCell::new(), source: SlotCell::new() }
    }

    /// Whether the file was accessed in the ongoing compilation.
    fn accessed(&self) -> bool {
        self.source.accessed() || self.file.accessed()
    }

    /// Marks the file as not yet accessed in preparation of the next
    /// compilation.
    fn reset(&mut self) {
        self.source.reset();
        self.file.reset();
    }

    /// Retrieve the source for this file.
    fn source(
        &mut self,
        project_root: &Path,
        package_storage: &PackageStorage,
    ) -> FileResult<Source> {
        self.source.get_or_init(
            || read(self.id, project_root, package_storage),
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
    fn file(
        &mut self,
        project_root: &Path,
        package_storage: &PackageStorage,
    ) -> FileResult<Bytes> {
        self.file.get_or_init(
            || read(self.id, project_root, package_storage),
            |data, _| Ok(Bytes::new(data)),
        )
    }
}

/// Lazily processes data for a file.
struct SlotCell<T> {
    /// The processed data.
    data: Option<FileResult<T>>,
    /// A hash of the raw file contents / access error.
    fingerprint: u128,
    /// Whether the slot has been accessed in the current compilation.
    accessed: bool,
}

impl<T: Clone> SlotCell<T> {
    /// Creates a new, empty cell.
    fn new() -> Self {
        Self { data: None, fingerprint: 0, accessed: false }
    }

    /// Whether the cell was accessed in the ongoing compilation.
    fn accessed(&self) -> bool {
        self.accessed
    }

    /// Marks the cell as not yet accessed in preparation of the next
    /// compilation.
    fn reset(&mut self) {
        self.accessed = false;
    }

    /// Gets the contents of the cell.
    fn get(&self) -> Option<&FileResult<T>> {
        self.data.as_ref()
    }

    /// Gets the contents of the cell or initialize them.
    fn get_or_init(
        &mut self,
        load: impl FnOnce() -> FileResult<Vec<u8>>,
        f: impl FnOnce(Vec<u8>, Option<T>) -> FileResult<T>,
    ) -> FileResult<T> {
        // If we accessed the file already in this compilation, retrieve it.
        if mem::replace(&mut self.accessed, true) {
            if let Some(data) = &self.data {
                return data.clone();
            }
        }

        // Read and hash the file.
        let result = timed!("loading file", load());
        let fingerprint = timed!("hashing file", typst::utils::hash128(&result));

        // If the file contents didn't change, yield the old processed data.
        if mem::replace(&mut self.fingerprint, fingerprint) == fingerprint {
            if let Some(data) = &self.data {
                return data.clone();
            }
        }

        let prev = self.data.take().and_then(Result::ok);
        let value = result.and_then(|data| f(data, prev));
        self.data = Some(value.clone());

        value
    }
}

/// Resolves the path of a file id on the system, downloading a package if
/// necessary.
fn system_path(
    project_root: &Path,
    id: FileId,
    package_storage: &PackageStorage,
) -> FileResult<PathBuf> {
    // Determine the root path relative to which the file path
    // will be resolved.
    let buf;
    let mut root = project_root;
    if let Some(spec) = id.package() {
        buf = package_storage.prepare_package(spec, &mut PrintDownload(&spec))?;
        root = &buf;
    }

    // Join the path to the root. If it tries to escape, deny
    // access. Note: It can still escape via symlinks.
    id.vpath().resolve(root).ok_or(FileError::AccessDenied)
}

/// Reads a file from a `FileId`.
///
/// If the ID represents stdin it will read from standard input,
/// otherwise it gets the file path of the ID and reads the file from disk.
fn read(
    id: FileId,
    project_root: &Path,
    package_storage: &PackageStorage,
) -> FileResult<Vec<u8>> {
    if id == *STDIN_ID {
        read_from_stdin()
    } else {
        read_from_disk(&system_path(project_root, id, package_storage)?)
    }
}

/// Read a file from disk.
fn read_from_disk(path: &Path) -> FileResult<Vec<u8>> {
    let f = |e| FileError::from_io(e, path);
    if fs::metadata(path).map_err(f)?.is_dir() {
        Err(FileError::IsDirectory)
    } else {
        fs::read(path).map_err(f)
    }
}

/// Read from stdin.
fn read_from_stdin() -> FileResult<Vec<u8>> {
    let mut buf = Vec::new();
    let result = io::stdin().read_to_end(&mut buf);
    match result {
        Ok(_) => (),
        Err(err) if err.kind() == io::ErrorKind::BrokenPipe => (),
        Err(err) => return Err(FileError::from_io(err, Path::new("<stdin>"))),
    }
    Ok(buf)
}

/// Decode UTF-8 with an optional BOM.
fn decode_utf8(buf: &[u8]) -> FileResult<&str> {
    // Remove UTF-8 BOM.
    Ok(std::str::from_utf8(buf.strip_prefix(b"\xef\xbb\xbf").unwrap_or(buf))?)
}

/// The current date and time.
enum Now {
    /// The date and time if the environment `SOURCE_DATE_EPOCH` is set.
    /// Used for reproducible builds.
    Fixed(DateTime<Utc>),
    /// The current date and time if the time is not externally fixed.
    System(OnceLock<DateTime<Utc>>),
}

/// An error that occurs during world construction.
#[derive(Debug)]
pub enum WorldCreationError {
    /// The input file does not appear to exist.
    InputNotFound(PathBuf),
    /// The input file is not contained within the root folder.
    InputOutsideRoot,
    /// The root directory does not appear to exist.
    RootNotFound(PathBuf),
    /// Another type of I/O error.
    Io(io::Error),
}

impl fmt::Display for WorldCreationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            WorldCreationError::InputNotFound(path) => {
                write!(f, "input file not found (searched at {})", path.display())
            }
            WorldCreationError::InputOutsideRoot => {
                write!(f, "source file must be contained in project root")
            }
            WorldCreationError::RootNotFound(path) => {
                write!(f, "root directory not found (searched at {})", path.display())
            }
            WorldCreationError::Io(err) => write!(f, "{err}"),
        }
    }
}

impl From<WorldCreationError> for EcoString {
    fn from(err: WorldCreationError) -> Self {
        eco_format!("{err}")
    }
}
