use std::cell::RefCell;
use std::collections::{hash_map::Entry, HashMap};
use std::fs::{self, File};
use std::hash::Hash;
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::process;

use codespan_reporting::diagnostic::{Diagnostic, Label};
use codespan_reporting::term::{self, termcolor};
use elsa::FrozenVec;
use memmap2::Mmap;
use once_cell::unsync::OnceCell;
use pico_args::Arguments;
use same_file::{is_same_file, Handle};
use siphasher::sip128::{Hasher128, SipHasher};
use termcolor::{ColorChoice, StandardStream, WriteColor};
use walkdir::WalkDir;

use typst::diag::{failed_to_load, Error, StrResult};
use typst::font::{Font, FontBook, FontInfo, FontVariant};
use typst::library::text::THEME;
use typst::parse::TokenMode;
use typst::source::{Source, SourceId};
use typst::util::Buffer;
use typst::{Config, World};

type CodespanResult<T> = Result<T, CodespanError>;
type CodespanError = codespan_reporting::files::Error;

/// What to do.
enum Command {
    Typeset(TypesetCommand),
    Highlight(HighlightCommand),
    Fonts(FontsCommand),
}

/// Typeset a .typ file into a PDF file.
struct TypesetCommand {
    input: PathBuf,
    output: PathBuf,
    root: Option<PathBuf>,
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
  -h, --help     Print this help
  --root <dir>   Configure the root for absolute paths

SUBCOMMANDS:
  --highlight    Highlight .typ files to HTML
  --fonts        List all discovered system fonts
";

/// Highlight a .typ file into an HTML file.
struct HighlightCommand {
    input: PathBuf,
    output: PathBuf,
}

const HELP_HIGHLIGHT: &'static str = "\
typst --highlight creates highlighted HTML from .typ files

USAGE:
  typst --highlight [OPTIONS] <input.typ> [output.html]

ARGS:
  <input.typ>    Path to input Typst file
  [output.html]  Path to output HTML file

OPTIONS:
  -h, --help     Print this help
";

/// List discovered system fonts.
struct FontsCommand {
    variants: bool,
}

const HELP_FONTS: &'static str = "\
typst --fonts lists all discovered system fonts

USAGE:
  typst --fonts [OPTIONS]

OPTIONS:
  -h, --help     Print this help
  --variants     Also list style variants of each font family
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
    let help = args.contains(["-h", "--help"]);

    let command = if args.contains("--highlight") {
        if help {
            print_help(HELP_HIGHLIGHT);
        }

        let (input, output) = parse_input_output(&mut args, "html")?;
        Command::Highlight(HighlightCommand { input, output })
    } else if args.contains("--fonts") {
        if help {
            print_help(HELP_FONTS);
        }

        Command::Fonts(FontsCommand { variants: args.contains("--variants") })
    } else {
        if help {
            print_help(HELP);
        }

        let root = args.opt_value_from_str("--root").map_err(|_| "missing root path")?;
        let (input, output) = parse_input_output(&mut args, "pdf")?;
        Command::Typeset(TypesetCommand { input, output, root })
    };

    // Don't allow excess arguments.
    let rest = args.finish();
    if !rest.is_empty() {
        Err(format!(
            "unexpected argument{}",
            if rest.len() > 1 { "s" } else { "" }
        ))?;
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
fn print_help(help: &'static str) {
    print!("{help}");
    std::process::exit(0);
}

/// Print an application-level error (independent from a source file).
fn print_error(msg: &str) -> io::Result<()> {
    let mut w = StandardStream::stderr(ColorChoice::Always);
    let styles = term::Styles::default();

    w.set_color(&styles.header_error)?;
    write!(w, "error")?;

    w.reset()?;
    writeln!(w, ": {msg}.")
}

/// Dispatch a command.
fn dispatch(command: Command) -> StrResult<()> {
    match command {
        Command::Typeset(command) => typeset(command),
        Command::Highlight(command) => highlight(command),
        Command::Fonts(command) => fonts(command),
    }
}

/// Execute a typesetting command.
fn typeset(command: TypesetCommand) -> StrResult<()> {
    let mut world = SystemWorld::new();
    if let Some(root) = &command.root {
        world.config.root = root.clone();
    } else if let Some(dir) = command.input.parent() {
        world.config.root = dir.into();
    }

    // Create the world that serves sources, fonts and files.
    let id = world
        .resolve(&command.input)
        .map_err(|err| failed_to_load("source file", &command.input, err))?;

    // Typeset.
    match typst::typeset(&world, id) {
        // Export the PDF.
        Ok(frames) => {
            let buffer = typst::export::pdf(&frames);
            fs::write(&command.output, buffer).map_err(|_| "failed to write PDF file")?;
        }

        // Print diagnostics.
        Err(errors) => {
            print_diagnostics(&world, *errors)
                .map_err(|_| "failed to print diagnostics")?;
        }
    }

    Ok(())
}

/// Print diagnostic messages to the terminal.
fn print_diagnostics(
    world: &SystemWorld,
    errors: Vec<Error>,
) -> Result<(), codespan_reporting::files::Error> {
    let mut w = StandardStream::stderr(ColorChoice::Always);
    let config = term::Config { tab_width: 2, ..Default::default() };

    for error in errors {
        // The main diagnostic.
        let range = world.source(error.span.source()).range(error.span);
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

/// Execute a highlighting command.
fn highlight(command: HighlightCommand) -> StrResult<()> {
    let input =
        fs::read_to_string(&command.input).map_err(|_| "failed to load source file")?;

    let html = typst::syntax::highlight_html(&input, TokenMode::Markup, &THEME);
    fs::write(&command.output, html).map_err(|_| "failed to write HTML file")?;

    Ok(())
}

/// Execute a font listing command.
fn fonts(command: FontsCommand) -> StrResult<()> {
    let world = SystemWorld::new();
    for (name, infos) in world.book().families() {
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
    config: Config,
    sources: FrozenVec<Box<Source>>,
    nav: RefCell<HashMap<PathHash, SourceId>>,
    book: FontBook,
    fonts: Vec<FontSlot>,
    files: RefCell<HashMap<PathHash, Buffer>>,
}

struct FontSlot {
    path: PathBuf,
    index: u32,
    font: OnceCell<Option<Font>>,
}

impl SystemWorld {
    fn new() -> Self {
        let mut world = Self {
            config: Config::default(),
            book: FontBook::new(),
            sources: FrozenVec::new(),
            nav: RefCell::new(HashMap::new()),
            fonts: vec![],
            files: RefCell::new(HashMap::new()),
        };
        world.search_system();
        world
    }
}

impl World for SystemWorld {
    fn config(&self) -> &Config {
        &self.config
    }

    fn resolve(&self, path: &Path) -> io::Result<SourceId> {
        let hash = PathHash::new(path)?;
        if let Some(&id) = self.nav.borrow().get(&hash) {
            return Ok(id);
        }

        let data = fs::read(path)?;
        let text = String::from_utf8(data).map_err(|_| {
            io::Error::new(io::ErrorKind::InvalidData, "file is not valid utf-8")
        })?;

        let id = SourceId::from_raw(self.sources.len() as u16);
        let source = Source::new(id, path, text);
        self.sources.push(Box::new(source));
        self.nav.borrow_mut().insert(hash, id);

        Ok(id)
    }

    fn source(&self, id: SourceId) -> &Source {
        &self.sources[id.into_raw() as usize]
    }

    fn book(&self) -> &FontBook {
        &self.book
    }

    fn font(&self, id: usize) -> io::Result<Font> {
        let slot = &self.fonts[id];
        slot.font
            .get_or_init(|| {
                let data = self.file(&slot.path).ok()?;
                Font::new(data, slot.index)
            })
            .clone()
            .ok_or_else(|| io::ErrorKind::InvalidData.into())
    }

    fn file(&self, path: &Path) -> io::Result<Buffer> {
        let hash = PathHash::new(path)?;
        Ok(match self.files.borrow_mut().entry(hash) {
            Entry::Occupied(entry) => entry.get().clone(),
            Entry::Vacant(entry) => entry.insert(fs::read(path)?.into()).clone(),
        })
    }
}

/// A hash that is the same for all paths pointing to the same file.
#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
struct PathHash(u128);

impl PathHash {
    fn new(path: &Path) -> io::Result<Self> {
        let file = File::open(path)?;
        if file.metadata()?.is_file() {
            let handle = Handle::from_file(file)?;
            let mut state = SipHasher::new();
            handle.hash(&mut state);
            Ok(Self(state.finish128().as_u128()))
        } else {
            Err(io::ErrorKind::NotFound.into())
        }
    }
}

impl SystemWorld {
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

    /// Search for all fonts in a directory.
    /// recursively.
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
                for (i, info) in FontInfo::from_data(&mmap).enumerate() {
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
