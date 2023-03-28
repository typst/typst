use std::cell::{RefCell, RefMut};
use std::collections::HashMap;
use std::fs::{self, File};
use std::hash::Hash;
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::process;

use codespan_reporting::diagnostic::{Diagnostic, Label};
use codespan_reporting::term::{self, termcolor};
use comemo::Prehashed;
use elsa::FrozenVec;
use memmap2::Mmap;
use notify::{RecommendedWatcher, RecursiveMode, Watcher};
use once_cell::unsync::OnceCell;
use pico_args::Arguments;
use same_file::{is_same_file, Handle};
use siphasher::sip128::{Hasher128, SipHasher};
use termcolor::{ColorChoice, StandardStream, WriteColor};
use typst::diag::{FileError, FileResult, SourceError, StrResult};
use typst::eval::Library;
use typst::font::{Font, FontBook, FontInfo, FontVariant};
use typst::syntax::{Source, SourceId};
use typst::util::{Buffer, PathExt};
use typst::World;
use walkdir::WalkDir;

type CodespanResult<T> = Result<T, CodespanError>;
type CodespanError = codespan_reporting::files::Error;

/// What to do.
enum Command {
    Compile(CompileCommand),
    Fonts(FontsCommand),
}

/// Compile a .typ file into a PDF file.
struct CompileCommand {
    input: PathBuf,
    output: PathBuf,
    root: Option<PathBuf>,
    watch: bool,
    font_paths: Vec<PathBuf>,
}

const HELP: &'static str = "\
typst creates PDF files from .typ files

USAGE:
  typst [OPTIONS] <input.typ> [output.pdf]
  typst [SUBCOMMAND] ...

ARGS:
  <input.typ>    Path to input Typst file
  [output.pdf]   Path to output PDF file

OPTIONS:
  -h, --help        Print this help
  -V, --version     Print the CLI's version
  -w, --watch       Watch the inputs and recompile on changes
  --font-path <dir> Add additional directories to search for fonts
  --root <dir>      Configure the root for absolute paths

SUBCOMMANDS:
  --fonts           List all discovered fonts in system and custom font paths
";

/// List discovered system fonts.
struct FontsCommand {
    font_paths: Vec<PathBuf>,
    variants: bool,
}

const HELP_FONTS: &'static str = "\
typst --fonts lists all discovered fonts in system and custom font paths

USAGE:
  typst --fonts [OPTIONS]

OPTIONS:
  -h, --help        Print this help
  --font-path <dir> Add additional directories to search for fonts
  --variants        Also list style variants of each font family
";

/// Entry point.
fn main() {
    let command = parse_args();
    let ok = command.is_ok();
    if let Err(msg) = command.and_then(dispatch) {
        print_error(&msg).unwrap();
        if !ok {
            println!("\nfor more information, try --help");
        }
        process::exit(1);
    }
}

/// Parse command line arguments.
fn parse_args() -> StrResult<Command> {
    let mut args = Arguments::from_env();
    if args.contains(["-V", "--version"]) {
        print_version();
    }

    let help = args.contains(["-h", "--help"]);
    let font_paths = args.values_from_str("--font-path").unwrap();

    let command = if args.contains("--fonts") {
        if help {
            print_help(HELP_FONTS);
        }

        Command::Fonts(FontsCommand { font_paths, variants: args.contains("--variants") })
    } else {
        if help {
            print_help(HELP);
        }

        let root = args.opt_value_from_str("--root").map_err(|_| "missing root path")?;
        let watch = args.contains(["-w", "--watch"]);
        let (input, output) = parse_input_output(&mut args, "pdf")?;
        Command::Compile(CompileCommand { input, output, watch, root, font_paths })
    };

    // Don't allow excess arguments.
    let rest = args.finish();
    if !rest.is_empty() {
        Err(format!("unexpected argument{}", if rest.len() > 1 { "s" } else { "" }))?;
    }

    Ok(command)
}

/// Parse two freestanding path arguments, with the output path being optional.
/// If it is omitted, it is determined from the input path's file stem plus the
/// given extension.
fn parse_input_output(args: &mut Arguments, ext: &str) -> StrResult<(PathBuf, PathBuf)> {
    let input: PathBuf = args.free_from_str().map_err(|_| "missing input file")?;
    let output = match args.opt_free_from_str().ok().flatten() {
        Some(output) => output,
        None => {
            let name = input.file_name().ok_or("source path does not point to a file")?;
            Path::new(name).with_extension(ext)
        }
    };

    // Ensure that the source file is not overwritten.
    if is_same_file(&input, &output).unwrap_or(false) {
        Err("source and destination files are the same")?;
    }

    Ok((input, output))
}

/// Print a help string and quit.
fn print_help(help: &'static str) -> ! {
    print!("{help}");
    std::process::exit(0);
}

/// Print the version hash and quit.
fn print_version() -> ! {
    println!("typst {}", env!("TYPST_VERSION"));
    std::process::exit(0);
}

/// Print an application-level error (independent from a source file).
fn print_error(msg: &str) -> io::Result<()> {
    let mut w = StandardStream::stderr(ColorChoice::Auto);
    let styles = term::Styles::default();

    w.set_color(&styles.header_error)?;
    write!(w, "error")?;

    w.reset()?;
    writeln!(w, ": {msg}.")
}

/// Dispatch a command.
fn dispatch(command: Command) -> StrResult<()> {
    match command {
        Command::Compile(command) => compile(command),
        Command::Fonts(command) => fonts(command),
    }
}

/// Execute a compilation command.
fn compile(command: CompileCommand) -> StrResult<()> {
    let root = if let Some(root) = &command.root {
        root.clone()
    } else if let Some(dir) = command.input.parent() {
        dir.into()
    } else {
        PathBuf::new()
    };

    // Create the world that serves sources, fonts and files.
    let mut world = SystemWorld::new(root, &command.font_paths);

    // Perform initial compilation.
    let failed = compile_once(&mut world, &command)?;
    if !command.watch {
        // Return with non-zero exit code in case of error.
        if failed {
            process::exit(1);
        }

        return Ok(());
    }

    // Setup file watching.
    let (tx, rx) = std::sync::mpsc::channel();
    let mut watcher = RecommendedWatcher::new(tx, notify::Config::default())
        .map_err(|_| "failed to watch directory")?;

    // Watch this directory recursively.
    watcher
        .watch(Path::new("."), RecursiveMode::Recursive)
        .map_err(|_| "failed to watch directory")?;

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
                .all(|path| is_same_file(path, &command.output).unwrap_or(false))
            {
                continue;
            }

            recompile |= world.relevant(&event);
        }

        if recompile {
            compile_once(&mut world, &command)?;
            comemo::evict(30);
        }
    }
}

/// Compile a single time.
fn compile_once(world: &mut SystemWorld, command: &CompileCommand) -> StrResult<bool> {
    status(command, Status::Compiling).unwrap();

    world.reset();
    world.main = world.resolve(&command.input).map_err(|err| err.to_string())?;

    match typst::compile(world) {
        // Export the PDF.
        Ok(document) => {
            let buffer = typst::export::pdf(&document);
            fs::write(&command.output, buffer).map_err(|_| "failed to write PDF file")?;
            status(command, Status::Success).unwrap();
            Ok(false)
        }

        // Print diagnostics.
        Err(errors) => {
            status(command, Status::Error).unwrap();
            print_diagnostics(&world, *errors)
                .map_err(|_| "failed to print diagnostics")?;
            Ok(true)
        }
    }
}

/// Clear the terminal and render the status message.
fn status(command: &CompileCommand, status: Status) -> io::Result<()> {
    if !command.watch {
        return Ok(());
    }

    let esc = 27 as char;
    let input = command.input.display();
    let output = command.output.display();
    let time = chrono::offset::Local::now();
    let timestamp = time.format("%H:%M:%S");
    let message = status.message();
    let color = status.color();

    let mut w = StandardStream::stderr(ColorChoice::Auto);
    write!(w, "{esc}c{esc}[1;1H")?;

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

/// The status in which the watcher can be.
enum Status {
    Compiling,
    Success,
    Error,
}

impl Status {
    fn message(&self) -> &str {
        match self {
            Self::Compiling => "compiling ...",
            Self::Success => "compiled successfully",
            Self::Error => "compiled with errors",
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
) -> Result<(), codespan_reporting::files::Error> {
    let mut w = StandardStream::stderr(ColorChoice::Auto);
    let config = term::Config { tab_width: 2, ..Default::default() };

    for error in errors {
        // The main diagnostic.
        let range = error.range(world);
        let diag = Diagnostic::error()
            .with_message(error.message)
            .with_labels(vec![Label::primary(error.span.source(), range)]);

        term::emit(&mut w, &config, world, &diag)?;

        // Stacktrace-like helper diagnostics.
        for point in error.trace {
            let message = point.v.to_string();
            let help = Diagnostic::help().with_message(message).with_labels(vec![
                Label::primary(
                    point.span.source(),
                    world.source(point.span.source()).range(point.span),
                ),
            ]);

            term::emit(&mut w, &config, world, &help)?;
        }
    }

    Ok(())
}

/// Execute a font listing command.
fn fonts(command: FontsCommand) -> StrResult<()> {
    let mut searcher = FontSearcher::new();
    searcher.search_system();
    for path in &command.font_paths {
        searcher.search_dir(path)
    }
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
    root: PathBuf,
    library: Prehashed<Library>,
    book: Prehashed<FontBook>,
    fonts: Vec<FontSlot>,
    hashes: RefCell<HashMap<PathBuf, FileResult<PathHash>>>,
    paths: RefCell<HashMap<PathHash, PathSlot>>,
    sources: FrozenVec<Box<Source>>,
    main: SourceId,
}

/// Holds details about the location of a font and lazily the font itself.
struct FontSlot {
    path: PathBuf,
    index: u32,
    font: OnceCell<Option<Font>>,
}

/// Holds canonical data for all paths pointing to the same entity.
#[derive(Default)]
struct PathSlot {
    source: OnceCell<FileResult<SourceId>>,
    buffer: OnceCell<FileResult<Buffer>>,
}

impl SystemWorld {
    fn new(root: PathBuf, font_paths: &[PathBuf]) -> Self {
        let mut searcher = FontSearcher::new();
        searcher.search_system();

        #[cfg(feature = "embed-fonts")]
        searcher.add_embedded();

        for path in font_paths {
            searcher.search_dir(path)
        }

        Self {
            root,
            library: Prehashed::new(typst_library::build()),
            book: Prehashed::new(searcher.book),
            fonts: searcher.fonts,
            hashes: RefCell::default(),
            paths: RefCell::default(),
            sources: FrozenVec::new(),
            main: SourceId::detached(),
        }
    }
}

impl World for SystemWorld {
    fn root(&self) -> &Path {
        &self.root
    }

    fn library(&self) -> &Prehashed<Library> {
        &self.library
    }

    fn main(&self) -> &Source {
        self.source(self.main)
    }

    fn resolve(&self, path: &Path) -> FileResult<SourceId> {
        self.slot(path)?
            .source
            .get_or_init(|| {
                let buf = read(path)?;
                let text = String::from_utf8(buf)?;
                Ok(self.insert(path, text))
            })
            .clone()
    }

    fn source(&self, id: SourceId) -> &Source {
        &self.sources[id.into_u16() as usize]
    }

    fn book(&self) -> &Prehashed<FontBook> {
        &self.book
    }

    fn font(&self, id: usize) -> Option<Font> {
        let slot = &self.fonts[id];
        slot.font
            .get_or_init(|| {
                let data = self.file(&slot.path).ok()?;
                Font::new(data, slot.index)
            })
            .clone()
    }

    fn file(&self, path: &Path) -> FileResult<Buffer> {
        self.slot(path)?
            .buffer
            .get_or_init(|| read(path).map(Buffer::from))
            .clone()
    }
}

impl SystemWorld {
    fn slot(&self, path: &Path) -> FileResult<RefMut<PathSlot>> {
        let mut hashes = self.hashes.borrow_mut();
        let hash = match hashes.get(path).cloned() {
            Some(hash) => hash,
            None => {
                let hash = PathHash::new(path);
                if let Ok(canon) = path.canonicalize() {
                    hashes.insert(canon.normalize(), hash.clone());
                }
                hashes.insert(path.into(), hash.clone());
                hash
            }
        }?;

        Ok(std::cell::RefMut::map(self.paths.borrow_mut(), |paths| {
            paths.entry(hash).or_default()
        }))
    }

    fn insert(&self, path: &Path, text: String) -> SourceId {
        let id = SourceId::from_u16(self.sources.len() as u16);
        let source = Source::new(id, path, text);
        self.sources.push(Box::new(source));
        id
    }

    fn relevant(&mut self, event: &notify::Event) -> bool {
        match &event.kind {
            notify::EventKind::Any => {}
            notify::EventKind::Access(_) => return false,
            notify::EventKind::Create(_) => return true,
            notify::EventKind::Modify(kind) => match kind {
                notify::event::ModifyKind::Any => {}
                notify::event::ModifyKind::Data(_) => {}
                notify::event::ModifyKind::Metadata(_) => return false,
                notify::event::ModifyKind::Name(_) => return true,
                notify::event::ModifyKind::Other => return false,
            },
            notify::EventKind::Remove(_) => {}
            notify::EventKind::Other => return false,
        }

        event.paths.iter().any(|path| self.dependant(path))
    }

    fn dependant(&self, path: &Path) -> bool {
        self.hashes.borrow().contains_key(&path.normalize())
            || PathHash::new(path)
                .map_or(false, |hash| self.paths.borrow().contains_key(&hash))
    }

    fn reset(&mut self) {
        self.sources.as_mut().clear();
        self.hashes.borrow_mut().clear();
        self.paths.borrow_mut().clear();
    }
}

/// A hash that is the same for all paths pointing to the same entity.
#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
struct PathHash(u128);

impl PathHash {
    fn new(path: &Path) -> FileResult<Self> {
        let f = |e| FileError::from_io(e, path);
        let handle = Handle::from_path(path).map_err(f)?;
        let mut state = SipHasher::new();
        handle.hash(&mut state);
        Ok(Self(state.finish128().as_u128()))
    }
}

/// Read a file.
fn read(path: &Path) -> FileResult<Vec<u8>> {
    let f = |e| FileError::from_io(e, path);
    if fs::metadata(&path).map_err(f)?.is_file() {
        fs::read(&path).map_err(f)
    } else {
        Err(FileError::IsDirectory)
    }
}

impl<'a> codespan_reporting::files::Files<'a> for SystemWorld {
    type FileId = SourceId;
    type Name = std::path::Display<'a>;
    type Source = &'a str;

    fn name(&'a self, id: SourceId) -> CodespanResult<Self::Name> {
        Ok(World::source(self, id).path().display())
    }

    fn source(&'a self, id: SourceId) -> CodespanResult<Self::Source> {
        Ok(World::source(self, id).text())
    }

    fn line_index(&'a self, id: SourceId, given: usize) -> CodespanResult<usize> {
        let source = World::source(self, id);
        source
            .byte_to_line(given)
            .ok_or_else(|| CodespanError::IndexTooLarge {
                given,
                max: source.len_bytes(),
            })
    }

    fn line_range(
        &'a self,
        id: SourceId,
        given: usize,
    ) -> CodespanResult<std::ops::Range<usize>> {
        let source = World::source(self, id);
        source
            .line_to_range(given)
            .ok_or_else(|| CodespanError::LineTooLarge { given, max: source.len_lines() })
    }

    fn column_number(
        &'a self,
        id: SourceId,
        _: usize,
        given: usize,
    ) -> CodespanResult<usize> {
        let source = World::source(self, id);
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

    /// Add fonts that are embedded in the binary.
    #[cfg(feature = "embed-fonts")]
    fn add_embedded(&mut self) {
        let mut add = |bytes: &'static [u8]| {
            let buffer = Buffer::from_static(bytes);
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
        add(include_bytes!("../../assets/fonts/LinLibertine_R.ttf"));
        add(include_bytes!("../../assets/fonts/LinLibertine_RB.ttf"));
        add(include_bytes!("../../assets/fonts/LinLibertine_RBI.ttf"));
        add(include_bytes!("../../assets/fonts/LinLibertine_RI.ttf"));
        add(include_bytes!("../../assets/fonts/NewCMMath-Book.otf"));
        add(include_bytes!("../../assets/fonts/NewCMMath-Regular.otf"));
        add(include_bytes!("../../assets/fonts/DejaVuSansMono.ttf"));
        add(include_bytes!("../../assets/fonts/DejaVuSansMono-Bold.ttf"));
        add(include_bytes!("../../assets/fonts/DejaVuSansMono-Oblique.ttf"));
        add(include_bytes!("../../assets/fonts/DejaVuSansMono-BoldOblique.ttf"));
    }

    /// Search for fonts in the linux system font directories.
    #[cfg(all(unix, not(target_os = "macos")))]
    fn search_system(&mut self) {
        self.search_dir("/usr/share/fonts");
        self.search_dir("/usr/local/share/fonts");

        if let Some(dir) = dirs::font_dir() {
            self.search_dir(dir);
        }
    }

    /// Search for fonts in the macOS system font directories.
    #[cfg(target_os = "macos")]
    fn search_system(&mut self) {
        self.search_dir("/Library/Fonts");
        self.search_dir("/Network/Library/Fonts");
        self.search_dir("/System/Library/Fonts");

        if let Some(dir) = dirs::font_dir() {
            self.search_dir(dir);
        }
    }

    /// Search for fonts in the Windows system font directories.
    #[cfg(windows)]
    fn search_system(&mut self) {
        let windir =
            std::env::var("WINDIR").unwrap_or_else(|_| "C:\\Windows".to_string());

        self.search_dir(Path::new(&windir).join("Fonts"));

        if let Some(roaming) = dirs::config_dir() {
            self.search_dir(roaming.join("Microsoft\\Windows\\Fonts"));
        }

        if let Some(local) = dirs::cache_dir() {
            self.search_dir(local.join("Microsoft\\Windows\\Fonts"));
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
