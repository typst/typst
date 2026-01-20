use std::fmt;
use std::io::{self, Read};
use std::path::{Path, PathBuf};
use std::sync::{LazyLock, OnceLock};

use chrono::{DateTime, Datelike, FixedOffset, Local, Utc};
use ecow::{EcoString, eco_format};
use typst::diag::{FileError, FileResult};
use typst::foundations::{Bytes, Datetime, Dict, IntoValue, Repr};
use typst::syntax::{
    FileId, PathError, RootedPath, Source, VirtualPath, VirtualRoot, VirtualizeError,
};
use typst::text::{Font, FontBook};
use typst::utils::LazyHash;
use typst::{Library, LibraryExt, World};
use typst_kit::diagnostics::DiagnosticWorld;
use typst_kit::files::{FileLoader, FileStore, FsRoot};
use typst_kit::fonts::FontStore;
use typst_kit::packages::SystemPackages;

use crate::args::{Feature, Input, ProcessArgs, WorldArgs};

/// A world that provides access to the operating system.
pub struct SystemWorld {
    /// The working directory.
    workdir: Option<PathBuf>,
    /// Typst's standard library.
    library: LazyHash<Library>,
    /// Metadata about discovered fonts and lazily loaded fonts.
    fonts: LazyLock<FontStore, Box<dyn Fn() -> FontStore + Send + Sync>>,
    /// Maps file ids to source files and buffers.
    files: FileStore<SystemFiles>,
    /// The current datetime if requested. This is stored here to ensure it is
    /// always the same within one compilation.
    /// Reset between compilations if not [`Now::Fixed`].
    now: Now,
}

impl SystemWorld {
    /// Creates a new system world.
    pub fn new(
        input: Option<&Input>,
        world_args: &'static WorldArgs,
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
                    Feature::A11yExtras => typst::Feature::A11yExtras,
                })
                .collect();

            Library::builder().with_inputs(inputs).with_features(features).build()
        };

        let now = match world_args.creation_timestamp {
            Some(time) => Now::Fixed(time),
            None => Now::System(OnceLock::new()),
        };

        Ok(Self {
            workdir: std::env::current_dir().ok(),
            library: LazyHash::new(library),
            fonts: LazyLock::new(Box::new(|| {
                crate::fonts::discover_fonts(&world_args.font)
            })),
            files: FileStore::new(SystemFiles::new(input, world_args)?),
            now,
        })
    }

    /// The project root relative to which absolute paths are resolved.
    pub fn root(&self) -> &Path {
        self.files.loader().project.path()
    }

    /// The current working directory.
    pub fn workdir(&self) -> &Path {
        self.workdir.as_deref().unwrap_or(Path::new("."))
    }

    /// Return all paths the last compilation depended on.
    pub fn dependencies(&mut self) -> impl Iterator<Item = PathBuf> + '_ {
        let (loader, deps) = self.files.dependencies();
        deps.filter_map(|id| loader.resolve(id).ok())
    }

    /// Reset the compilation state in preparation of a new compilation.
    pub fn reset(&mut self) {
        self.files.reset();
        if let Now::System(time_lock) = &mut self.now {
            time_lock.take();
        }
    }

    /// Forcibly scan fonts instead of doing it lazily upon the first access.
    ///
    /// Does nothing if the fonts were already scanned.
    pub fn scan_fonts(&mut self) {
        LazyLock::force(&self.fonts);
    }
}

impl World for SystemWorld {
    fn library(&self) -> &LazyHash<Library> {
        &self.library
    }

    fn book(&self) -> &LazyHash<FontBook> {
        self.fonts.book()
    }

    fn main(&self) -> FileId {
        self.files.loader().main
    }

    fn source(&self, id: FileId) -> FileResult<Source> {
        self.files.source(id)
    }

    fn file(&self, id: FileId) -> FileResult<Bytes> {
        self.files.file(id)
    }

    fn font(&self, index: usize) -> Option<Font> {
        self.fonts.font(index)
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

impl DiagnosticWorld for SystemWorld {
    fn name(&self, id: FileId) -> String {
        let vpath = id.vpath();
        match id.root() {
            VirtualRoot::Project => {
                // Try to express the path relative to the working directory.
                let rooted = vpath.realize(self.root());
                pathdiff::diff_paths(rooted, self.workdir())
                    .map(|path| path.to_string_lossy().into_owned())
                    .unwrap_or_else(|| vpath.get_without_slash().into())
            }
            VirtualRoot::Package(package) => {
                format!("{package}{}", vpath.get_with_slash())
            }
        }
    }
}

/// Static `FileId` allocated for stdin. This is to ensure that stdin can live
/// in the project root without colliding with any real on-disk file.
static STDIN_ID: LazyLock<FileId> = LazyLock::new(|| {
    FileId::unique(RootedPath::new(
        VirtualRoot::Project,
        VirtualPath::new("<stdin>").unwrap(),
    ))
});

/// Static `FileId` allocated for empty/no input at all. This is to ensure that
/// we can create a [`SystemWorld`] based on no main file or stdin at all.
static EMPTY_ID: LazyLock<FileId> = LazyLock::new(|| {
    FileId::unique(RootedPath::new(
        VirtualRoot::Project,
        VirtualPath::new("<empty>").unwrap(),
    ))
});

/// Provides project files from a configured directory and package files from
/// standard locations.
struct SystemFiles {
    main: FileId,
    project: FsRoot,
    packages: SystemPackages,
}

impl SystemFiles {
    /// Creates a new loader given the configuration.
    pub fn new(
        input: Option<&Input>,
        world_args: &'static WorldArgs,
    ) -> Result<Self, WorldCreationError> {
        // Resolve the system-global input path.
        let input_path = match input {
            Some(Input::Path(path)) => {
                Some(path.canonicalize().map_err(|err| match err.kind() {
                    io::ErrorKind::NotFound => {
                        WorldCreationError::InputNotFound(path.clone())
                    }
                    _ => WorldCreationError::Io(err),
                })?)
            }
            _ => None,
        };

        // Resolve the system-global root directory.
        let root = {
            let path = world_args
                .root
                .as_deref()
                .or_else(|| input_path.as_deref().and_then(|i| i.parent()))
                .unwrap_or(Path::new("."));
            path.canonicalize().map_err(|err| match err.kind() {
                io::ErrorKind::NotFound => {
                    WorldCreationError::RootNotFound(path.to_path_buf())
                }
                _ => WorldCreationError::Io(err),
            })?
        };

        let main = if let Some(path) = &input_path {
            // Resolve the virtual path of the main file within the project root.
            RootedPath::new(VirtualRoot::Project, VirtualPath::virtualize(&root, path)?)
                .intern()
        } else if matches!(input, Some(Input::Stdin)) {
            // Return the special id of STDIN.
            *STDIN_ID
        } else {
            // Return the special id of EMPTY/no input at all otherwise.
            *EMPTY_ID
        };

        Ok(Self {
            main,
            project: FsRoot::new(root),
            packages: crate::packages::system(&world_args.package),
        })
    }

    /// Resolves the file system path for the given `id`.
    pub fn resolve(&self, id: FileId) -> FileResult<PathBuf> {
        Ok(self.root(id)?.resolve(id.vpath()))
    }

    /// Resolves the root in which the given file ID resides.
    fn root(&self, id: FileId) -> FileResult<FsRoot> {
        Ok(match id.root() {
            VirtualRoot::Project => self.project.clone(),
            VirtualRoot::Package(spec) => self.packages.obtain(spec)?,
        })
    }
}

impl FileLoader for SystemFiles {
    fn load(&self, id: FileId) -> FileResult<Bytes> {
        if id == *EMPTY_ID {
            Ok(Bytes::new([]))
        } else if id == *STDIN_ID {
            read_from_stdin().map(Bytes::new)
        } else {
            self.root(id)?.load(id.vpath())
        }
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
    /// The input file path was malformed.
    InputMalformed(VirtualizeError),
    /// The root directory does not appear to exist.
    RootNotFound(PathBuf),
    /// Another type of I/O error.
    Io(io::Error),
}

impl fmt::Display for WorldCreationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            WorldCreationError::InputMalformed(err) => match err {
                VirtualizeError::Path(PathError::Escapes) => {
                    write!(f, "source file must be contained in project root")
                }
                VirtualizeError::Path(PathError::Backslash) => {
                    write!(f, "source path must not contain a backslash")
                }
                VirtualizeError::Invalid(s) => {
                    write!(f, "source path contains invalid sequence `{}`", s.repr())
                }
                VirtualizeError::Utf8 => write!(f, "source path must be valid UTF-8"),
            },
            WorldCreationError::InputNotFound(path) => {
                write!(f, "input file not found (searched at {})", path.display())
            }
            WorldCreationError::RootNotFound(path) => {
                write!(f, "root directory not found (searched at {})", path.display())
            }
            WorldCreationError::Io(err) => write!(f, "{err}"),
        }
    }
}

impl From<VirtualizeError> for WorldCreationError {
    fn from(err: VirtualizeError) -> Self {
        Self::InputMalformed(err)
    }
}

impl From<WorldCreationError> for EcoString {
    fn from(err: WorldCreationError) -> Self {
        eco_format!("{err}")
    }
}
