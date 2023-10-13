#![allow(clippy::comparison_chain)]

use std::cell::{RefCell, RefMut};
use std::collections::{HashMap, HashSet};
use std::env;
use std::ffi::OsStr;
use std::fmt::{self, Display, Formatter, Write as _};
use std::fs;
use std::io::{self, Write};
use std::ops::Range;
use std::path::{Path, PathBuf};

use clap::Parser;
use comemo::{Prehashed, Track};
use ecow::EcoString;
use oxipng::{InFile, Options, OutFile};
use rayon::iter::{ParallelBridge, ParallelIterator};
use std::cell::OnceCell;
use tiny_skia as sk;
use unscanny::Scanner;
use walkdir::WalkDir;

use typst::diag::{bail, FileError, FileResult, Severity, StrResult};
use typst::doc::{Document, Frame, FrameItem, Meta};
use typst::eval::{
    eco_format, func, Bytes, Datetime, Library, NoneValue, Repr, Tracer, Value,
};
use typst::font::{Font, FontBook};
use typst::geom::{Abs, Color, Smart};
use typst::syntax::{FileId, PackageVersion, Source, SyntaxNode, VirtualPath};
use typst::{World, WorldExt};
use typst_library::layout::{Margin, PageElem};
use typst_library::text::{TextElem, TextSize};

const TYP_DIR: &str = "typ";
const REF_DIR: &str = "ref";
const PNG_DIR: &str = "png";
const PDF_DIR: &str = "pdf";
const SVG_DIR: &str = "svg";
const FONT_DIR: &str = "../assets/fonts";
const ASSET_DIR: &str = "../assets";

#[derive(Debug, Clone, Parser)]
#[clap(name = "typst-test", author)]
struct Args {
    filter: Vec<String>,
    /// runs only the specified subtest
    #[arg(short, long)]
    #[arg(allow_hyphen_values = true)]
    subtest: Option<isize>,
    #[arg(long)]
    exact: bool,
    #[arg(long, default_value_t = env::var_os("UPDATE_EXPECT").is_some())]
    update: bool,
    #[arg(long)]
    pdf: bool,
    #[command(flatten)]
    print: PrintConfig,
    #[arg(long)]
    nocapture: bool, // simply ignores the argument
}

/// Which things to print out for debugging.
#[derive(Default, Debug, Copy, Clone, Eq, PartialEq, Parser)]
struct PrintConfig {
    #[arg(long)]
    syntax: bool,
    #[arg(long)]
    model: bool,
    #[arg(long)]
    frames: bool,
}

impl Args {
    fn matches(&self, path: &Path) -> bool {
        if self.exact {
            let name = path.file_name().unwrap().to_string_lossy();
            self.filter.iter().any(|v| v == &name)
        } else {
            let path = path.to_string_lossy();
            self.filter.is_empty() || self.filter.iter().any(|v| path.contains(v))
        }
    }
}

fn main() {
    let args = Args::parse();

    // Create loader and context.
    let world = TestWorld::new(args.print);

    println!("Running tests...");
    let results = WalkDir::new("typ")
        .into_iter()
        .par_bridge()
        .filter_map(|entry| {
            let entry = entry.unwrap();
            if entry.depth() == 0 {
                return None;
            }

            if entry.path().starts_with("typ/benches") {
                return None;
            }

            let src_path = entry.into_path();
            if src_path.extension() != Some(OsStr::new("typ")) {
                return None;
            }

            if args.matches(&src_path) {
                Some(src_path)
            } else {
                None
            }
        })
        .map_with(world, |world, src_path| {
            let path = src_path.strip_prefix(TYP_DIR).unwrap();
            let png_path = Path::new(PNG_DIR).join(path).with_extension("png");
            let ref_path = Path::new(REF_DIR).join(path).with_extension("png");
            let svg_path = Path::new(SVG_DIR).join(path).with_extension("svg");
            let pdf_path =
                args.pdf.then(|| Path::new(PDF_DIR).join(path).with_extension("pdf"));

            test(
                world,
                &src_path,
                &png_path,
                &ref_path,
                pdf_path.as_deref(),
                &svg_path,
                &args,
            ) as usize
        })
        .collect::<Vec<_>>();

    let len = results.len();
    let ok = results.iter().sum::<usize>();
    if len > 1 {
        println!("{ok} / {len} tests passed.");
    }

    if ok != len {
        println!(
            "Set the UPDATE_EXPECT environment variable or pass the \
             --update flag to update the reference image(s)."
        );
    }

    if ok < len {
        std::process::exit(1);
    }
}

fn library() -> Library {
    #[func]
    fn test(lhs: Value, rhs: Value) -> StrResult<NoneValue> {
        if lhs != rhs {
            bail!("Assertion failed: {} != {}", lhs.repr(), rhs.repr());
        }
        Ok(NoneValue)
    }

    #[func]
    fn test_repr(lhs: Value, rhs: Value) -> StrResult<NoneValue> {
        if lhs.repr() != rhs.repr() {
            bail!("Assertion failed: {} != {}", lhs.repr(), rhs.repr());
        }
        Ok(NoneValue)
    }

    #[func]
    fn print(#[variadic] values: Vec<Value>) -> NoneValue {
        let mut stdout = io::stdout().lock();
        write!(stdout, "> ").unwrap();
        for (i, value) in values.into_iter().enumerate() {
            if i > 0 {
                write!(stdout, ", ").unwrap();
            }
            write!(stdout, "{value:?}").unwrap();
        }
        writeln!(stdout).unwrap();
        NoneValue
    }

    let mut lib = typst_library::build();

    // Set page width to 120pt with 10pt margins, so that the inner page is
    // exactly 100pt wide. Page height is unbounded and font size is 10pt so
    // that it multiplies to nice round numbers.
    lib.styles
        .set(PageElem::set_width(Smart::Custom(Abs::pt(120.0).into())));
    lib.styles.set(PageElem::set_height(Smart::Auto));
    lib.styles.set(PageElem::set_margin(Margin::splat(Some(Smart::Custom(
        Abs::pt(10.0).into(),
    )))));
    lib.styles.set(TextElem::set_size(TextSize(Abs::pt(10.0).into())));

    // Hook up helpers into the global scope.
    lib.global.scope_mut().define_func::<test>();
    lib.global.scope_mut().define_func::<test_repr>();
    lib.global.scope_mut().define_func::<print>();
    lib.global
        .scope_mut()
        .define("conifer", Color::from_u8(0x9f, 0xEB, 0x52, 0xFF));
    lib.global
        .scope_mut()
        .define("forest", Color::from_u8(0x43, 0xA1, 0x27, 0xFF));

    lib
}

/// A world that provides access to the tests environment.
#[derive(Clone)]
struct TestWorld {
    print: PrintConfig,
    main: FileId,
    library: Prehashed<Library>,
    book: Prehashed<FontBook>,
    fonts: Vec<Font>,
    paths: RefCell<HashMap<PathBuf, PathSlot>>,
}

#[derive(Clone)]
struct PathSlot {
    system_path: PathBuf,
    source: OnceCell<FileResult<Source>>,
    buffer: OnceCell<FileResult<Bytes>>,
}

impl TestWorld {
    fn new(print: PrintConfig) -> Self {
        // Search for fonts.
        let mut fonts = vec![];
        for entry in WalkDir::new(FONT_DIR)
            .sort_by_file_name()
            .into_iter()
            .filter_map(|e| e.ok())
            .filter(|entry| entry.file_type().is_file())
        {
            let data = fs::read(entry.path()).unwrap();
            fonts.extend(Font::iter(data.into()));
        }

        Self {
            print,
            main: FileId::new(None, VirtualPath::new("main.typ")),
            library: Prehashed::new(library()),
            book: Prehashed::new(FontBook::from_fonts(&fonts)),
            fonts,
            paths: RefCell::default(),
        }
    }
}

impl World for TestWorld {
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
                let text = String::from_utf8(buf)?;
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
        Some(self.fonts[id].clone())
    }

    fn today(&self, _: Option<i64>) -> Option<Datetime> {
        Some(Datetime::from_ymd(1970, 1, 1).unwrap())
    }
}

impl TestWorld {
    fn set(&mut self, path: &Path, text: String) -> Source {
        self.main = FileId::new(None, VirtualPath::new(path));
        let mut slot = self.slot(self.main).unwrap();
        let source = Source::new(self.main, text);
        slot.source = OnceCell::from(Ok(source.clone()));
        source
    }

    fn slot(&self, id: FileId) -> FileResult<RefMut<PathSlot>> {
        let root: PathBuf = match id.package() {
            Some(spec) => format!("packages/{}-{}", spec.name, spec.version).into(),
            None => PathBuf::new(),
        };

        let system_path = id.vpath().resolve(&root).ok_or(FileError::AccessDenied)?;

        Ok(RefMut::map(self.paths.borrow_mut(), |paths| {
            paths.entry(system_path.clone()).or_insert_with(|| PathSlot {
                system_path,
                source: OnceCell::new(),
                buffer: OnceCell::new(),
            })
        }))
    }
}

/// Read as file.
fn read(path: &Path) -> FileResult<Vec<u8>> {
    // Basically symlinks `assets/files` to `tests/files` so that the assets
    // are within the test project root.
    let mut resolved = path.to_path_buf();
    if path.starts_with("files/") {
        resolved = Path::new(ASSET_DIR).join(path);
    }

    let f = |e| FileError::from_io(e, path);
    if fs::metadata(&resolved).map_err(f)?.is_dir() {
        Err(FileError::IsDirectory)
    } else {
        fs::read(&resolved).map_err(f)
    }
}

fn test(
    world: &mut TestWorld,
    src_path: &Path,
    png_path: &Path,
    ref_path: &Path,
    pdf_path: Option<&Path>,
    svg_path: &Path,
    args: &Args,
) -> bool {
    struct PanicGuard<'a>(&'a Path);
    impl Drop for PanicGuard<'_> {
        fn drop(&mut self) {
            if std::thread::panicking() {
                println!("Panicked in {}", self.0.display());
            }
        }
    }

    let name = src_path.strip_prefix(TYP_DIR).unwrap_or(src_path);
    let text = fs::read_to_string(src_path).unwrap();
    let _guard = PanicGuard(name);

    let mut output = String::new();
    let mut ok = true;
    let mut updated = false;
    let mut frames = vec![];
    let mut line = 0;
    let mut compare_ref = None;
    let mut validate_hints = None;
    let mut compare_ever = false;
    let mut rng = LinearShift::new();

    let parts: Vec<_> = text
        .split("\n---")
        .map(|s| s.strip_suffix('\r').unwrap_or(s))
        .collect();

    for (i, &part) in parts.iter().enumerate() {
        if let Some(x) = args.subtest {
            let x = usize::try_from(
                x.rem_euclid(isize::try_from(parts.len()).unwrap_or_default()),
            )
            .unwrap();
            if x != i {
                writeln!(output, "  Skipped subtest {i}.").unwrap();
                continue;
            }
        }
        let is_header = i == 0
            && parts.len() > 1
            && part
                .lines()
                .all(|s| s.starts_with("//") || s.chars().all(|c| c.is_whitespace()));

        if is_header {
            for line in part.lines() {
                compare_ref = get_flag_metadata(line, "Ref").or(compare_ref);
                validate_hints = get_flag_metadata(line, "Hints").or(validate_hints);
            }
        } else {
            let (part_ok, compare_here, part_frames) = test_part(
                &mut output,
                world,
                src_path,
                part.into(),
                i,
                compare_ref.unwrap_or(true),
                validate_hints.unwrap_or(true),
                line,
                &mut rng,
            );

            ok &= part_ok;
            compare_ever |= compare_here;
            frames.extend(part_frames);
        }

        line += part.lines().count() + 1;
    }

    let document = Document { pages: frames, ..Default::default() };
    if compare_ever {
        if let Some(pdf_path) = pdf_path {
            let pdf_data = typst::export::pdf(&document);
            fs::create_dir_all(pdf_path.parent().unwrap()).unwrap();
            fs::write(pdf_path, pdf_data).unwrap();
        }

        if world.print.frames {
            for frame in &document.pages {
                writeln!(output, "{:#?}\n", frame).unwrap();
            }
        }

        let canvas = render(&document.pages);
        fs::create_dir_all(png_path.parent().unwrap()).unwrap();
        canvas.save_png(png_path).unwrap();

        let svg = typst::export::svg_merged(&document.pages, Abs::pt(5.0));
        fs::create_dir_all(svg_path.parent().unwrap()).unwrap();
        std::fs::write(svg_path, svg).unwrap();

        if let Ok(ref_pixmap) = sk::Pixmap::load_png(ref_path) {
            if canvas.width() != ref_pixmap.width()
                || canvas.height() != ref_pixmap.height()
                || canvas
                    .data()
                    .iter()
                    .zip(ref_pixmap.data())
                    .any(|(&a, &b)| a.abs_diff(b) > 2)
            {
                if args.update {
                    update_image(png_path, ref_path);
                    updated = true;
                } else {
                    writeln!(output, "  Does not match reference image.").unwrap();
                    ok = false;
                }
            }
        } else if !document.pages.is_empty() {
            if args.update {
                update_image(png_path, ref_path);
                updated = true;
            } else {
                writeln!(output, "  Failed to open reference image.").unwrap();
                ok = false;
            }
        }
    }

    {
        let mut stdout = io::stdout().lock();
        stdout.write_all(name.to_string_lossy().as_bytes()).unwrap();
        if ok {
            writeln!(stdout, " ✔").unwrap();
        } else {
            writeln!(stdout, " ❌").unwrap();
        }
        if updated {
            writeln!(stdout, "  Updated reference image.").unwrap();
        }
        if !output.is_empty() {
            stdout.write_all(output.as_bytes()).unwrap();
        }
    }

    ok
}

fn get_metadata<'a>(line: &'a str, key: &str) -> Option<&'a str> {
    line.strip_prefix(eco_format!("// {key}: ").as_str())
}

fn get_flag_metadata(line: &str, key: &str) -> Option<bool> {
    get_metadata(line, key).map(|value| value == "true")
}

fn update_image(png_path: &Path, ref_path: &Path) {
    oxipng::optimize(
        &InFile::Path(png_path.to_owned()),
        &OutFile::Path(Some(ref_path.to_owned())),
        &Options::max_compression(),
    )
    .unwrap();
}

#[allow(clippy::too_many_arguments)]
fn test_part(
    output: &mut String,
    world: &mut TestWorld,
    src_path: &Path,
    text: String,
    i: usize,
    compare_ref: bool,
    validate_hints: bool,
    line: usize,
    rng: &mut LinearShift,
) -> (bool, bool, Vec<Frame>) {
    let mut ok = true;

    let source = world.set(src_path, text);
    if world.print.syntax {
        writeln!(output, "Syntax Tree:\n{:#?}\n", source.root()).unwrap();
    }

    let metadata = parse_part_metadata(&source);
    let compare_ref = metadata.part_configuration.compare_ref.unwrap_or(compare_ref);
    let validate_hints =
        metadata.part_configuration.validate_hints.unwrap_or(validate_hints);

    ok &= test_spans(output, source.root());
    ok &= test_reparse(output, source.text(), i, rng);

    if world.print.model {
        let world = (world as &dyn World).track();
        let route = typst::eval::Route::default();
        let mut tracer = typst::eval::Tracer::new();

        let module =
            typst::eval::eval(world, route.track(), tracer.track_mut(), &source).unwrap();
        writeln!(output, "Model:\n{:#?}\n", module.content()).unwrap();
    }

    let mut tracer = Tracer::new();
    let (mut frames, diagnostics) = match typst::compile(world, &mut tracer) {
        Ok(document) => (document.pages, tracer.warnings()),
        Err(errors) => {
            let mut warnings = tracer.warnings();
            warnings.extend(*errors);
            (vec![], warnings)
        }
    };

    // Don't retain frames if we don't want to compare with reference images.
    if !compare_ref {
        frames.clear();
    }

    // Map diagnostics to range and message format, discard traces and errors from
    // other files, collect hints.
    //
    // This has one caveat: due to the format of the expected hints, we can not
    // verify if a hint belongs to a diagnostic or not. That should be irrelevant
    // however, as the line of the hint is still verified.
    let mut actual_diagnostics = HashSet::new();
    for diagnostic in &diagnostics {
        // Ignore diagnostics from other files.
        if diagnostic.span.id().map_or(false, |id| id != source.id()) {
            continue;
        }

        let annotation = Annotation {
            kind: match diagnostic.severity {
                Severity::Error => AnnotationKind::Error,
                Severity::Warning => AnnotationKind::Warning,
            },
            range: world.range(diagnostic.span),
            message: diagnostic.message.replace("\\", "/"),
        };

        if validate_hints {
            for hint in &diagnostic.hints {
                actual_diagnostics.insert(Annotation {
                    kind: AnnotationKind::Hint,
                    message: hint.clone(),
                    range: annotation.range.clone(),
                });
            }
        }

        actual_diagnostics.insert(annotation);
    }

    // Basically symmetric_difference, but we need to know where an item is coming from.
    let mut unexpected_outputs = actual_diagnostics
        .difference(&metadata.annotations)
        .collect::<Vec<_>>();
    let mut missing_outputs = metadata
        .annotations
        .difference(&actual_diagnostics)
        .collect::<Vec<_>>();

    unexpected_outputs.sort_by_key(|&v| v.range.as_ref().map(|r| r.start));
    missing_outputs.sort_by_key(|&v| v.range.as_ref().map(|r| r.start));

    // This prints all unexpected emits first, then all missing emits.
    // Is this reasonable or subject to change?
    if !(unexpected_outputs.is_empty() && missing_outputs.is_empty()) {
        writeln!(output, "  Subtest {i} does not match expected errors.").unwrap();
        ok = false;

        for unexpected in unexpected_outputs {
            write!(output, "    Not annotated | ").unwrap();
            print_annotation(output, &source, line, unexpected)
        }

        for missing in missing_outputs {
            write!(output, "    Not emitted   | ").unwrap();
            print_annotation(output, &source, line, missing)
        }
    }

    (ok, compare_ref, frames)
}

fn print_annotation(
    output: &mut String,
    source: &Source,
    line: usize,
    annotation: &Annotation,
) {
    let Annotation { range, message, kind } = annotation;
    write!(output, "{kind}: ").unwrap();
    if let Some(range) = range {
        let start_line = 1 + line + source.byte_to_line(range.start).unwrap();
        let start_col = 1 + source.byte_to_column(range.start).unwrap();
        let end_line = 1 + line + source.byte_to_line(range.end).unwrap();
        let end_col = 1 + source.byte_to_column(range.end).unwrap();
        write!(output, "{start_line}:{start_col}-{end_line}:{end_col}: ").unwrap();
    }
    writeln!(output, "{message}").unwrap();
}

struct TestConfiguration {
    compare_ref: Option<bool>,
    validate_hints: Option<bool>,
}

struct TestPartMetadata {
    part_configuration: TestConfiguration,
    annotations: HashSet<Annotation>,
}

#[derive(Debug, Clone, Eq, PartialEq, Hash)]
struct Annotation {
    range: Option<Range<usize>>,
    message: EcoString,
    kind: AnnotationKind,
}

#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
enum AnnotationKind {
    Error,
    Warning,
    Hint,
}

impl AnnotationKind {
    fn iter() -> impl Iterator<Item = Self> {
        [AnnotationKind::Error, AnnotationKind::Warning, AnnotationKind::Hint].into_iter()
    }

    fn as_str(self) -> &'static str {
        match self {
            AnnotationKind::Error => "Error",
            AnnotationKind::Warning => "Warning",
            AnnotationKind::Hint => "Hint",
        }
    }
}

impl Display for AnnotationKind {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        f.pad(self.as_str())
    }
}

fn parse_part_metadata(source: &Source) -> TestPartMetadata {
    let mut compare_ref = None;
    let mut validate_hints = None;
    let mut annotations = HashSet::default();

    let lines: Vec<_> = source.text().lines().map(str::trim).collect();
    for (i, line) in lines.iter().enumerate() {
        compare_ref = get_flag_metadata(line, "Ref").or(compare_ref);
        validate_hints = get_flag_metadata(line, "Hints").or(validate_hints);

        fn num(s: &mut Scanner) -> Option<isize> {
            let mut first = true;
            let n = &s.eat_while(|c: char| {
                let valid = first && c == '-' || c.is_numeric();
                first = false;
                valid
            });
            n.parse().ok()
        }

        let comments_until_code =
            lines[i..].iter().take_while(|line| line.starts_with("//")).count();

        let pos = |s: &mut Scanner| -> Option<usize> {
            let first = num(s)? - 1;
            let (delta, column) =
                if s.eat_if(':') { (first, num(s)? - 1) } else { (0, first) };
            let line = (i + comments_until_code).checked_add_signed(delta)?;
            source.line_column_to_byte(line, usize::try_from(column).ok()?)
        };

        let range = |s: &mut Scanner| -> Option<Range<usize>> {
            let start = pos(s)?;
            let end = if s.eat_if('-') { pos(s)? } else { start };
            Some(start..end)
        };

        for kind in AnnotationKind::iter() {
            let Some(expectation) = get_metadata(line, kind.as_str()) else { continue };
            let mut s = Scanner::new(expectation);
            let range = range(&mut s);
            let rest = if range.is_some() { s.after() } else { s.string() };
            let message = rest
                .trim()
                .replace("VERSION", &PackageVersion::compiler().to_string())
                .into();
            annotations.insert(Annotation { kind, range, message });
        }
    }

    TestPartMetadata {
        part_configuration: TestConfiguration { compare_ref, validate_hints },
        annotations,
    }
}

/// Pseudorandomly edit the source file and test whether a reparse produces the
/// same result as a clean parse.
///
/// The method will first inject 10 strings once every 400 source characters
/// and then select 5 leaf node boundaries to inject an additional, randomly
/// chosen string from the injection list.
fn test_reparse(
    output: &mut String,
    text: &str,
    i: usize,
    rng: &mut LinearShift,
) -> bool {
    let supplements = [
        "[",
        "]",
        "{",
        "}",
        "(",
        ")",
        "#rect()",
        "a word",
        ", a: 1",
        "10.0",
        ":",
        "if i == 0 {true}",
        "for",
        "* hello *",
        "//",
        "/*",
        "\\u{12e4}",
        "```typst",
        " ",
        "trees",
        "\\",
        "$ a $",
        "2.",
        "-",
        "5",
    ];

    let mut ok = true;
    let mut apply = |replace: Range<usize>, with| {
        let mut incr_source = Source::detached(text);
        if incr_source.root().len() != text.len() {
            println!(
                "    Subtest {i} tree length {} does not match string length {} ❌",
                incr_source.root().len(),
                text.len(),
            );
            return false;
        }

        incr_source.edit(replace.clone(), with);

        let edited_src = incr_source.text();
        let ref_source = Source::detached(edited_src);
        let ref_root = ref_source.root();
        let incr_root = incr_source.root();

        // Ensures that the span numbering invariants hold.
        let spans_ok = test_spans(output, ref_root) && test_spans(output, incr_root);

        // Ensure that the reference and incremental trees are the same.
        let tree_ok = ref_root.spanless_eq(incr_root);

        if !tree_ok {
            writeln!(
                output,
                "    Subtest {i} reparse differs from clean parse when inserting '{with}' at {}-{} ❌\n",
                replace.start, replace.end,
            ).unwrap();
            writeln!(output, "    Expected reference tree:\n{ref_root:#?}\n").unwrap();
            writeln!(output, "    Found incremental tree:\n{incr_root:#?}").unwrap();
            writeln!(
                output,
                "    Full source ({}):\n\"{edited_src:?}\"",
                edited_src.len()
            )
            .unwrap();
        }

        spans_ok && tree_ok
    };

    let mut pick = |range: Range<usize>| {
        let ratio = rng.next();
        (range.start as f64 + ratio * (range.end - range.start) as f64).floor() as usize
    };

    let insertions = (text.len() as f64 / 400.0).ceil() as usize;
    for _ in 0..insertions {
        let supplement = supplements[pick(0..supplements.len())];
        let start = pick(0..text.len());
        let end = pick(start..text.len());

        if !text.is_char_boundary(start) || !text.is_char_boundary(end) {
            continue;
        }

        ok &= apply(start..end, supplement);
    }

    let source = Source::detached(text);
    let leafs = leafs(source.root());
    let start = source.find(leafs[pick(0..leafs.len())].span()).unwrap().offset();
    let supplement = supplements[pick(0..supplements.len())];
    ok &= apply(start..start, supplement);

    ok
}

/// Returns all leaf descendants of a node (may include itself).
fn leafs(node: &SyntaxNode) -> Vec<SyntaxNode> {
    if node.children().len() == 0 {
        vec![node.clone()]
    } else {
        node.children().flat_map(leafs).collect()
    }
}

/// Ensure that all spans are properly ordered (and therefore unique).
#[track_caller]
fn test_spans(output: &mut String, root: &SyntaxNode) -> bool {
    test_spans_impl(output, root, 0..u64::MAX)
}

#[track_caller]
fn test_spans_impl(output: &mut String, node: &SyntaxNode, within: Range<u64>) -> bool {
    if !within.contains(&node.span().number()) {
        writeln!(output, "    Node: {node:#?}").unwrap();
        writeln!(
            output,
            "    Wrong span order: {} not in {within:?} ❌",
            node.span().number()
        )
        .unwrap();
    }

    let start = node.span().number() + 1;
    let mut children = node.children().peekable();
    while let Some(child) = children.next() {
        let end = children.peek().map_or(within.end, |next| next.span().number());
        if !test_spans_impl(output, child, start..end) {
            return false;
        }
    }

    true
}

/// Draw all frames into one image with padding in between.
fn render(frames: &[Frame]) -> sk::Pixmap {
    let pixel_per_pt = 2.0;
    let padding = Abs::pt(5.0);

    for frame in frames {
        let limit = Abs::cm(100.0);
        if frame.width() > limit || frame.height() > limit {
            panic!("overlarge frame: {:?}", frame.size());
        }
    }

    let mut pixmap = typst::export::render_merged(
        frames,
        pixel_per_pt,
        Color::WHITE,
        padding,
        Color::BLACK,
    );

    let padding = (pixel_per_pt * padding.to_pt() as f32).round();
    let [x, mut y] = [padding; 2];
    for frame in frames {
        let ts =
            sk::Transform::from_scale(pixel_per_pt, pixel_per_pt).post_translate(x, y);
        render_links(&mut pixmap, ts, frame);
        y += (pixel_per_pt * frame.height().to_pt() as f32).round().max(1.0) + padding;
    }

    pixmap
}

/// Draw extra boxes for links so we can see whether they are there.
fn render_links(canvas: &mut sk::Pixmap, ts: sk::Transform, frame: &Frame) {
    for (pos, item) in frame.items() {
        let ts = ts.pre_translate(pos.x.to_pt() as f32, pos.y.to_pt() as f32);
        match *item {
            FrameItem::Group(ref group) => {
                let ts = ts.pre_concat(group.transform.into());
                render_links(canvas, ts, &group.frame);
            }
            FrameItem::Meta(Meta::Link(_), size) => {
                let w = size.x.to_pt() as f32;
                let h = size.y.to_pt() as f32;
                let rect = sk::Rect::from_xywh(0.0, 0.0, w, h).unwrap();
                let mut paint = sk::Paint::default();
                paint.set_color_rgba8(40, 54, 99, 40);
                canvas.fill_rect(rect, &paint, ts, None);
            }
            _ => {}
        }
    }
}

/// A Linear-feedback shift register using XOR as its shifting function.
/// Can be used as PRNG.
struct LinearShift(u64);

impl LinearShift {
    /// Initialize the shift register with a pre-set seed.
    pub fn new() -> Self {
        Self(0xACE5)
    }

    /// Return a pseudo-random number between `0.0` and `1.0`.
    pub fn next(&mut self) -> f64 {
        self.0 ^= self.0 >> 3;
        self.0 ^= self.0 << 14;
        self.0 ^= self.0 >> 28;
        self.0 ^= self.0 << 36;
        self.0 ^= self.0 >> 52;
        self.0 as f64 / u64::MAX as f64
    }
}
