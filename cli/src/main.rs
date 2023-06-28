mod args;
mod trace;

use std::cell::{Cell, RefCell, RefMut};
use std::collections::{HashMap, HashSet};
use std::env;
use std::fs::{self, File};
use std::hash::Hash;
use std::io::{self, IsTerminal, Write};
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use chrono::Datelike;
use clap::Parser;
use codespan_reporting::diagnostic::{Diagnostic, Label};
use codespan_reporting::term::{self, termcolor};
use comemo::Prehashed;
use memmap2::Mmap;
use notify::{RecommendedWatcher, RecursiveMode, Watcher};
use same_file::{is_same_file, Handle};
use siphasher::sip128::{Hasher128, SipHasher13};
use std::cell::OnceCell;
use termcolor::{ColorChoice, StandardStream, WriteColor};
use typst::diag::{
    bail, FileError, FileResult, PackageError, PackageResult, SourceError, StrResult,
};
use typst::doc::Document;
use typst::eval::{eco_format, Datetime, Library};
use typst::file::{FileId, PackageSpec};
use typst::font::{Font, FontBook, FontInfo, FontVariant};
use typst::geom::Color;
use typst::syntax::Source;
use typst::util::{Bytes, PathExt};
use typst::World;
use walkdir::WalkDir;

use crate::args::{CliArguments, Command, CompileCommand, DiagnosticFormat};

type CodespanResult<T> = Result<T, CodespanError>;
type CodespanError = codespan_reporting::files::Error;

thread_local! {
    static EXIT: Cell<ExitCode> = Cell::new(ExitCode::SUCCESS);
}

/// Entry point.
fn main() -> ExitCode {
    let arguments = CliArguments::parse();
    let _guard = match crate::trace::init_tracing(&arguments) {
        Ok(guard) => guard,
        Err(err) => {
            eprintln!("failed to initialize tracing {}", err);
            None
        }
    };

    let res = match &arguments.command {
        Command::Compile(_) | Command::Watch(_) => {
            compile(CompileSettings::with_arguments(arguments))
        }
        Command::Fonts(_) => fonts(FontsSettings::with_arguments(arguments)),
    };

    if let Err(msg) = res {
        set_failed();
        print_error(&msg).expect("failed to print error");
    }

    EXIT.with(|cell| cell.get())
}

/// Ensure a failure exit code.
fn set_failed() {
    EXIT.with(|cell| cell.set(ExitCode::FAILURE));
}

/// Print an application-level error (independent from a source file).
fn print_error(msg: &str) -> io::Result<()> {
    let mut w = color_stream();
    let styles = term::Styles::default();

    w.set_color(&styles.header_error)?;
    write!(w, "error")?;

    w.reset()?;
    writeln!(w, ": {msg}.")
}

/// Used by `args.rs`.
fn typst_version() -> &'static str {
    env!("TYPST_VERSION")
}

/// A summary of the input arguments relevant to compilation.
struct CompileSettings {
    /// The project's root directory.
    root: Option<PathBuf>,
    /// The path to the input file.
    input: PathBuf,
    /// The path to the output file.
    output: PathBuf,
    /// Whether to watch the input files for changes.
    watch: bool,
    /// The paths to search for fonts.
    font_paths: Vec<PathBuf>,
    /// The open command to use.
    open: Option<Option<String>>,
    /// The PPI to use for PNG export.
    ppi: Option<f32>,
    /// In which format to emit diagnostics.
    diagnostic_format: DiagnosticFormat,
}

impl CompileSettings {
    /// Create a new compile settings from the field values.
    #[allow(clippy::too_many_arguments)]
    fn new(
        input: PathBuf,
        output: Option<PathBuf>,
        root: Option<PathBuf>,
        font_paths: Vec<PathBuf>,
        watch: bool,
        open: Option<Option<String>>,
        ppi: Option<f32>,
        diagnostic_format: DiagnosticFormat,
    ) -> Self {
        let output = match output {
            Some(path) => path,
            None => input.with_extension("pdf"),
        };
        Self {
            root,
            input,
            output,
            watch,
            font_paths,
            open,
            diagnostic_format,
            ppi,
        }
    }

    /// Create a new compile settings from the CLI arguments and a compile command.
    ///
    /// # Panics
    /// Panics if the command is not a compile or watch command.
    fn with_arguments(args: CliArguments) -> Self {
        let watch = matches!(args.command, Command::Watch(_));
        let CompileCommand { input, output, open, ppi, diagnostic_format, .. } =
            match args.command {
                Command::Compile(command) => command,
                Command::Watch(command) => command,
                _ => unreachable!(),
            };

        Self::new(
            input,
            output,
            args.root,
            args.font_paths,
            watch,
            open,
            ppi,
            diagnostic_format,
        )
    }
}

struct FontsSettings {
    /// The font paths
    font_paths: Vec<PathBuf>,
    /// Whether to include font variants
    variants: bool,
}

impl FontsSettings {
    /// Create font settings from the field values.
    fn new(font_paths: Vec<PathBuf>, variants: bool) -> Self {
        Self { font_paths, variants }
    }

    /// Create a new font settings from the CLI arguments.
    ///
    /// # Panics
    /// Panics if the command is not a fonts command.
    fn with_arguments(args: CliArguments) -> Self {
        match args.command {
            Command::Fonts(command) => Self::new(args.font_paths, command.variants),
            _ => unreachable!(),
        }
    }
}

/// Execute a compilation command.
fn compile(mut settings: CompileSettings) -> StrResult<()> {
    // Create the world that serves sources, files, and fonts.
    let mut world = SystemWorld::new(&settings)?;

    // Perform initial compilation.
    let ok = compile_once(&mut world, &settings)?;

    // Open the file if requested, this must be done on the first **successful**
    // compilation.
    if ok {
        if let Some(open) = settings.open.take() {
            open_file(open.as_deref(), &settings.output)?;
        }
    }

    if !settings.watch {
        return Ok(());
    }

    // Setup file watching.
    let (tx, rx) = std::sync::mpsc::channel();
    let mut watcher = RecommendedWatcher::new(tx, notify::Config::default())
        .map_err(|_| "failed to setup file watching")?;

    // Watch all the files that are used by the input file and its dependencies.
    world.watch(&mut watcher, HashSet::new())?;

    // Handle events.
    let timeout = std::time::Duration::from_millis(100);
    loop {
        let mut recompile = false;
        for event in rx
            .recv()
            .into_iter()
            .chain(std::iter::from_fn(|| rx.recv_timeout(timeout).ok()))
        {
            let event = event.map_err(|_| "failed to watch directory")?;
            if event
                .paths
                .iter()
                .all(|path| is_same_file(path, &settings.output).unwrap_or(false))
            {
                continue;
            }

            recompile |= is_event_relevant(&event);
        }

        if recompile {
            // Retrieve the dependencies of the last compilation.
            let dependencies = world.dependencies();

            // Recompile.
            let ok = compile_once(&mut world, &settings)?;
            comemo::evict(10);

            // Adjust the watching.
            world.watch(&mut watcher, dependencies)?;

            // Open the file if requested, this must be done on the first
            // **successful** compilation
            if ok {
                if let Some(open) = settings.open.take() {
                    open_file(open.as_deref(), &settings.output)?;
                }
            }
        }
    }
}

/// Compile a single time.
///
/// Returns whether it compiled without errors.
#[tracing::instrument(skip_all)]
fn compile_once(world: &mut SystemWorld, settings: &CompileSettings) -> StrResult<bool> {
    tracing::info!("Starting compilation");

    let start = std::time::Instant::now();
    status(settings, Status::Compiling).unwrap();

    // Reset everything and ensure that the main file is still present.
    world.reset();
    world.source(world.main).map_err(|err| err.to_string())?;

    let result = typst::compile(world);
    let duration = start.elapsed();

    match result {
        // Export the PDF / PNG.
        Ok(document) => {
            export(&document, settings)?;
            status(settings, Status::Success(duration)).unwrap();
            tracing::info!("Compilation succeeded in {duration:?}");
            Ok(true)
        }

        // Print diagnostics.
        Err(errors) => {
            set_failed();
            status(settings, Status::Error).unwrap();
            print_diagnostics(world, *errors, settings.diagnostic_format)
                .map_err(|_| "failed to print diagnostics")?;
            tracing::info!("Compilation failed after {duration:?}");
            Ok(false)
        }
    }
}

/// Export into the target format.
fn export(document: &Document, settings: &CompileSettings) -> StrResult<()> {
    match settings.output.extension() {
        Some(ext) if ext.eq_ignore_ascii_case("png") => {
            // Determine whether we have a `{n}` numbering.
            let string = settings.output.to_str().unwrap_or_default();
            let numbered = string.contains("{n}");
            if !numbered && document.pages.len() > 1 {
                bail!("cannot export multiple PNGs without `{{n}}` in output path");
            }

            // Find a number width that accommodates all pages. For instance, the
            // first page should be numbered "001" if there are between 100 and
            // 999 pages.
            let width = 1 + document.pages.len().checked_ilog10().unwrap_or(0) as usize;
            let ppi = settings.ppi.unwrap_or(2.0);
            let mut storage;

            for (i, frame) in document.pages.iter().enumerate() {
                let pixmap = typst::export::render(frame, ppi, Color::WHITE);
                let path = if numbered {
                    storage = string.replace("{n}", &format!("{:0width$}", i + 1));
                    Path::new(&storage)
                } else {
                    settings.output.as_path()
                };
                pixmap.save_png(path).map_err(|_| "failed to write PNG file")?;
            }
        }
        _ => {
            let buffer = typst::export::pdf(document);
            fs::write(&settings.output, buffer)
                .map_err(|_| "failed to write PDF file")?;
        }
    }
    Ok(())
}

/// Clear the terminal and render the status message.
#[tracing::instrument(skip_all)]
fn status(settings: &CompileSettings, status: Status) -> io::Result<()> {
    if !settings.watch {
        return Ok(());
    }

    let esc = 27 as char;
    let input = settings.input.display();
    let output = settings.output.display();
    let time = chrono::offset::Local::now();
    let timestamp = time.format("%H:%M:%S");
    let message = status.message();
    let color = status.color();

    let mut w = color_stream();
    if std::io::stderr().is_terminal() {
        // Clear the terminal.
        write!(w, "{esc}c{esc}[1;1H")?;
    }

    w.set_color(&color)?;
    write!(w, "watching")?;
    w.reset()?;
    writeln!(w, " {input}")?;

    w.set_color(&color)?;
    write!(w, "writing to")?;
    w.reset()?;
    writeln!(w, " {output}")?;

    writeln!(w)?;
    writeln!(w, "[{timestamp}] {message}")?;
    writeln!(w)?;

    w.flush()
}

/// Get stderr with color support if desirable.
fn color_stream() -> termcolor::StandardStream {
    termcolor::StandardStream::stderr(if std::io::stderr().is_terminal() {
        ColorChoice::Auto
    } else {
        ColorChoice::Never
    })
}

/// The status in which the watcher can be.
enum Status {
    Compiling,
    Success(std::time::Duration),
    Error,
}

impl Status {
    fn message(&self) -> String {
        match self {
            Self::Compiling => "compiling ...".into(),
            Self::Success(duration) => format!("compiled successfully in {duration:.2?}"),
            Self::Error => "compiled with errors".into(),
        }
    }

    fn color(&self) -> termcolor::ColorSpec {
        let styles = term::Styles::default();
        match self {
            Self::Error => styles.header_error,
            _ => styles.header_note,
        }
    }
}

/// Print diagnostic messages to the terminal.
fn print_diagnostics(
    world: &SystemWorld,
    errors: Vec<SourceError>,
    diagnostic_format: DiagnosticFormat,
) -> Result<(), codespan_reporting::files::Error> {
    let mut w = match diagnostic_format {
        DiagnosticFormat::Human => color_stream(),
        DiagnosticFormat::Short => StandardStream::stderr(ColorChoice::Never),
    };

    let mut config = term::Config { tab_width: 2, ..Default::default() };
    if diagnostic_format == DiagnosticFormat::Short {
        config.display_style = term::DisplayStyle::Short;
    }

    for error in errors {
        // The main diagnostic.
        let diag = Diagnostic::error()
            .with_message(error.message)
            .with_notes(
                error
                    .hints
                    .iter()
                    .map(|e| (eco_format!("hint: {e}")).into())
                    .collect(),
            )
            .with_labels(vec![Label::primary(error.span.id(), error.span.range(world))]);

        term::emit(&mut w, &config, world, &diag)?;

        // Stacktrace-like helper diagnostics.
        for point in error.trace {
            let message = point.v.to_string();
            let help = Diagnostic::help().with_message(message).with_labels(vec![
                Label::primary(point.span.id(), point.span.range(world)),
            ]);

            term::emit(&mut w, &config, world, &help)?;
        }
    }

    Ok(())
}

/// Execute a font listing command.
fn fonts(command: FontsSettings) -> StrResult<()> {
    let mut searcher = FontSearcher::new();
    searcher.search(&command.font_paths);

    for (name, infos) in searcher.book.families() {
        println!("{name}");
        if command.variants {
            for info in infos {
                let FontVariant { style, weight, stretch } = info.variant;
                println!("- Style: {style:?}, Weight: {weight:?}, Stretch: {stretch:?}");
            }
        }
    }

    Ok(())
}

/// A world that provides access to the operating system.
struct SystemWorld {
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
    /// The current date if requested. This is stored here to ensure it is
    /// always the same within one compilation. Reset between compilations.
    today: OnceCell<Option<Datetime>>,
}

/// Holds details about the location of a font and lazily the font itself.
struct FontSlot {
    /// The path at which the font can be found on the system.
    path: PathBuf,
    /// The index of the font in its collection. Zero if the path does not point
    /// to a collection.
    index: u32,
    /// The lazily loaded font.
    font: OnceCell<Option<Font>>,
}

/// Holds canonical data for all paths pointing to the same entity.
///
/// Both fields can be populated if the file is both imported and read().
struct PathSlot {
    /// The slot's path on the system.
    system_path: PathBuf,
    /// The lazily loaded source file for a path hash.
    source: OnceCell<FileResult<Source>>,
    /// The lazily loaded buffer for a path hash.
    buffer: OnceCell<FileResult<Bytes>>,
}

impl SystemWorld {
    fn new(settings: &CompileSettings) -> StrResult<Self> {
        let mut searcher = FontSearcher::new();
        searcher.search(&settings.font_paths);

        // Resolve the system-global input path.
        let system_input = settings.input.canonicalize().map_err(|_| {
            eco_format!("input file not found (searched at {})", settings.input.display())
        })?;

        // Resolve the system-global root directory.
        let root = {
            let path = settings
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
            today: OnceCell::new(),
        })
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
        let slot = self.slot(id)?;
        slot.source
            .get_or_init(|| {
                let buf = read(&slot.system_path)?;
                let text = decode_utf8(buf)?;
                Ok(Source::new(id, text))
            })
            .clone()
    }

    fn file(&self, id: FileId) -> FileResult<Bytes> {
        let slot = self.slot(id)?;
        slot.buffer
            .get_or_init(|| read(&slot.system_path).map(Bytes::from))
            .clone()
    }

    fn font(&self, id: usize) -> Option<Font> {
        let slot = &self.fonts[id];
        slot.font
            .get_or_init(|| {
                let data = read(&slot.path).ok()?.into();
                Font::new(data, slot.index)
            })
            .clone()
    }

    fn today(&self, offset: Option<i64>) -> Option<Datetime> {
        *self.today.get_or_init(|| {
            let naive = match offset {
                None => chrono::Local::now().naive_local(),
                Some(o) => (chrono::Utc::now() + chrono::Duration::hours(o)).naive_utc(),
            };

            Datetime::from_ymd(
                naive.year(),
                naive.month().try_into().ok()?,
                naive.day().try_into().ok()?,
            )
        })
    }
}

impl SystemWorld {
    /// Access the canonical slot for the given path.
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
                // This will only trigger if the `or_insert_with` above also
                // triggered.
                system_path,
                source: OnceCell::new(),
                buffer: OnceCell::new(),
            })
        }))
    }

    /// Collect all paths the last compilation depended on.
    #[tracing::instrument(skip_all)]
    fn dependencies(&self) -> HashSet<PathBuf> {
        self.paths
            .borrow()
            .values()
            .map(|slot| slot.system_path.clone())
            .collect()
    }

    /// Adjust the file watching. Watches all new dependencies and unwatches
    /// all `previous` dependencies that are not relevant anymore.
    #[tracing::instrument(skip_all)]
    fn watch(
        &self,
        watcher: &mut dyn Watcher,
        mut previous: HashSet<PathBuf>,
    ) -> StrResult<()> {
        // Watch new paths that weren't watched yet.
        for slot in self.paths.borrow().values() {
            let path = &slot.system_path;
            let watched = previous.remove(path);
            if path.exists() && !watched {
                tracing::info!("Watching {}", path.display());
                watcher
                    .watch(path, RecursiveMode::NonRecursive)
                    .map_err(|_| eco_format!("failed to watch {path:?}"))?;
            }
        }

        // Unwatch old paths that don't need to be watched anymore.
        for path in previous {
            tracing::info!("Unwatching {}", path.display());
            watcher.unwatch(&path).ok();
        }

        Ok(())
    }

    /// Reset th compilation state in preparation of a new compilation.
    #[tracing::instrument(skip_all)]
    fn reset(&mut self) {
        self.hashes.borrow_mut().clear();
        self.paths.borrow_mut().clear();
        self.today.take();
    }

    /// Lookup a source file by id.
    #[track_caller]
    fn lookup(&self, id: FileId) -> Source {
        self.source(id).expect("file id does not point to any source file")
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
#[tracing::instrument(skip_all)]
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

/// Make a package available in the on-disk cache.
fn prepare_package(spec: &PackageSpec) -> PackageResult<PathBuf> {
    let subdir =
        format!("typst/packages/{}/{}-{}", spec.namespace, spec.name, spec.version);

    if let Some(data_dir) = dirs::data_dir() {
        let dir = data_dir.join(&subdir);
        if dir.exists() {
            return Ok(dir);
        }
    }

    if let Some(cache_dir) = dirs::cache_dir() {
        let dir = cache_dir.join(&subdir);

        // Download from network if it doesn't exist yet.
        if spec.namespace == "preview" && !dir.exists() {
            download_package(spec, &dir)?;
        }

        if dir.exists() {
            return Ok(dir);
        }
    }

    Err(PackageError::NotFound(spec.clone()))
}

/// Download a package over the network.
fn download_package(spec: &PackageSpec, package_dir: &Path) -> PackageResult<()> {
    // The `@preview` namespace is the only namespace that supports on-demand
    // fetching.
    assert_eq!(spec.namespace, "preview");

    let url = format!(
        "https://packages.typst.org/preview/{}-{}.tar.gz",
        spec.name, spec.version
    );

    print_downloading(spec).unwrap();
    let reader = match ureq::get(&url).call() {
        Ok(response) => response.into_reader(),
        Err(ureq::Error::Status(404, _)) => {
            return Err(PackageError::NotFound(spec.clone()))
        }
        Err(_) => return Err(PackageError::NetworkFailed),
    };

    let decompressed = flate2::read::GzDecoder::new(reader);
    tar::Archive::new(decompressed).unpack(package_dir).map_err(|_| {
        fs::remove_dir_all(package_dir).ok();
        PackageError::MalformedArchive
    })
}

/// Print that a package downloading is happening.
fn print_downloading(spec: &PackageSpec) -> io::Result<()> {
    let mut w = color_stream();
    let styles = term::Styles::default();

    w.set_color(&styles.header_help)?;
    write!(w, "downloading")?;

    w.reset()?;
    writeln!(w, " {spec}")
}

/// Opens the given file using:
/// - The default file viewer if `open` is `None`.
/// - The given viewer provided by `open` if it is `Some`.
fn open_file(open: Option<&str>, path: &Path) -> StrResult<()> {
    if let Some(app) = open {
        open::with_in_background(path, app);
    } else {
        open::that_in_background(path);
    }

    Ok(())
}

/// Whether a watch event is relevant for compilation.
fn is_event_relevant(event: &notify::Event) -> bool {
    match &event.kind {
        notify::EventKind::Any => true,
        notify::EventKind::Access(_) => false,
        notify::EventKind::Create(_) => true,
        notify::EventKind::Modify(kind) => match kind {
            notify::event::ModifyKind::Any => true,
            notify::event::ModifyKind::Data(_) => true,
            notify::event::ModifyKind::Metadata(_) => false,
            notify::event::ModifyKind::Name(_) => true,
            notify::event::ModifyKind::Other => false,
        },
        notify::EventKind::Remove(_) => true,
        notify::EventKind::Other => false,
    }
}

impl<'a> codespan_reporting::files::Files<'a> for SystemWorld {
    type FileId = FileId;
    type Name = FileId;
    type Source = Source;

    fn name(&'a self, id: FileId) -> CodespanResult<Self::Name> {
        Ok(id)
    }

    fn source(&'a self, id: FileId) -> CodespanResult<Self::Source> {
        Ok(self.lookup(id))
    }

    fn line_index(&'a self, id: FileId, given: usize) -> CodespanResult<usize> {
        let source = self.lookup(id);
        source
            .byte_to_line(given)
            .ok_or_else(|| CodespanError::IndexTooLarge {
                given,
                max: source.len_bytes(),
            })
    }

    fn line_range(
        &'a self,
        id: FileId,
        given: usize,
    ) -> CodespanResult<std::ops::Range<usize>> {
        let source = self.lookup(id);
        source
            .line_to_range(given)
            .ok_or_else(|| CodespanError::LineTooLarge { given, max: source.len_lines() })
    }

    fn column_number(
        &'a self,
        id: FileId,
        _: usize,
        given: usize,
    ) -> CodespanResult<usize> {
        let source = self.lookup(id);
        source.byte_to_column(given).ok_or_else(|| {
            let max = source.len_bytes();
            if given <= max {
                CodespanError::InvalidCharBoundary { given }
            } else {
                CodespanError::IndexTooLarge { given, max }
            }
        })
    }
}

/// Searches for fonts.
struct FontSearcher {
    book: FontBook,
    fonts: Vec<FontSlot>,
}

impl FontSearcher {
    /// Create a new, empty system searcher.
    fn new() -> Self {
        Self { book: FontBook::new(), fonts: vec![] }
    }

    /// Search everything that is available.
    fn search(&mut self, font_paths: &[PathBuf]) {
        self.search_system();

        #[cfg(feature = "embed-fonts")]
        self.search_embedded();

        for path in font_paths {
            self.search_dir(path)
        }
    }

    /// Add fonts that are embedded in the binary.
    #[cfg(feature = "embed-fonts")]
    fn search_embedded(&mut self) {
        let mut search = |bytes: &'static [u8]| {
            let buffer = Bytes::from_static(bytes);
            for (i, font) in Font::iter(buffer).enumerate() {
                self.book.push(font.info().clone());
                self.fonts.push(FontSlot {
                    path: PathBuf::new(),
                    index: i as u32,
                    font: OnceCell::from(Some(font)),
                });
            }
        };

        // Embed default fonts.
        search(include_bytes!("../../assets/fonts/LinLibertine_R.ttf"));
        search(include_bytes!("../../assets/fonts/LinLibertine_RB.ttf"));
        search(include_bytes!("../../assets/fonts/LinLibertine_RBI.ttf"));
        search(include_bytes!("../../assets/fonts/LinLibertine_RI.ttf"));
        search(include_bytes!("../../assets/fonts/NewCMMath-Book.otf"));
        search(include_bytes!("../../assets/fonts/NewCMMath-Regular.otf"));
        search(include_bytes!("../../assets/fonts/NewCM10-Regular.otf"));
        search(include_bytes!("../../assets/fonts/NewCM10-Bold.otf"));
        search(include_bytes!("../../assets/fonts/NewCM10-Italic.otf"));
        search(include_bytes!("../../assets/fonts/NewCM10-BoldItalic.otf"));
        search(include_bytes!("../../assets/fonts/DejaVuSansMono.ttf"));
        search(include_bytes!("../../assets/fonts/DejaVuSansMono-Bold.ttf"));
        search(include_bytes!("../../assets/fonts/DejaVuSansMono-Oblique.ttf"));
        search(include_bytes!("../../assets/fonts/DejaVuSansMono-BoldOblique.ttf"));
    }

    /// Search for fonts in the linux system font directories.
    fn search_system(&mut self) {
        if cfg!(target_os = "macos") {
            self.search_dir("/Library/Fonts");
            self.search_dir("/Network/Library/Fonts");
            self.search_dir("/System/Library/Fonts");
        } else if cfg!(unix) {
            self.search_dir("/usr/share/fonts");
            self.search_dir("/usr/local/share/fonts");
        } else if cfg!(windows) {
            self.search_dir(
                env::var_os("WINDIR")
                    .map(PathBuf::from)
                    .unwrap_or_else(|| "C:\\Windows".into())
                    .join("Fonts"),
            );

            if let Some(roaming) = dirs::config_dir() {
                self.search_dir(roaming.join("Microsoft\\Windows\\Fonts"));
            }

            if let Some(local) = dirs::cache_dir() {
                self.search_dir(local.join("Microsoft\\Windows\\Fonts"));
            }
        }

        if let Some(dir) = dirs::font_dir() {
            self.search_dir(dir);
        }
    }

    /// Search for all fonts in a directory recursively.
    fn search_dir(&mut self, path: impl AsRef<Path>) {
        for entry in WalkDir::new(path)
            .follow_links(true)
            .sort_by(|a, b| a.file_name().cmp(b.file_name()))
            .into_iter()
            .filter_map(|e| e.ok())
        {
            let path = entry.path();
            if matches!(
                path.extension().and_then(|s| s.to_str()),
                Some("ttf" | "otf" | "TTF" | "OTF" | "ttc" | "otc" | "TTC" | "OTC"),
            ) {
                self.search_file(path);
            }
        }
    }

    /// Index the fonts in the file at the given path.
    fn search_file(&mut self, path: impl AsRef<Path>) {
        let path = path.as_ref();
        if let Ok(file) = File::open(path) {
            if let Ok(mmap) = unsafe { Mmap::map(&file) } {
                for (i, info) in FontInfo::iter(&mmap).enumerate() {
                    self.book.push(info);
                    self.fonts.push(FontSlot {
                        path: path.into(),
                        index: i as u32,
                        font: OnceCell::new(),
                    });
                }
            }
        }
    }
}
