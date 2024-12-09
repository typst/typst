use std::borrow::Cow;
use std::collections::HashMap;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::str::FromStr;
use std::sync::OnceLock;

use comemo::Tracked;
use parking_lot::Mutex;
use typst::diag::{bail, At, FileError, FileResult, SourceResult, StrResult};
use typst::engine::Engine;
use typst::foundations::{
    func, Array, Bytes, Context, Datetime, IntoValue, NoneValue, Repr, Smart, Value,
};
use typst::layout::{Abs, Margin, PageElem};
use typst::model::{Numbering, NumberingPattern};
use typst::syntax::{FileId, Source, Span};
use typst::text::{Font, FontBook, TextElem, TextSize};
use typst::utils::{singleton, LazyHash};
use typst::visualize::Color;
use typst::{Library, World};

/// A world that provides access to the tests environment.
#[derive(Clone)]
pub struct TestWorld {
    main: Source,
    base: &'static TestBase,
}

impl TestWorld {
    /// Create a new world for a single test.
    ///
    /// This is cheap because the shared base for all test runs is lazily
    /// initialized just once.
    pub fn new(source: Source) -> Self {
        Self {
            main: source,
            base: singleton!(TestBase, TestBase::default()),
        }
    }
}

impl World for TestWorld {
    fn library(&self) -> &LazyHash<Library> {
        &self.base.library
    }

    fn book(&self) -> &LazyHash<FontBook> {
        &self.base.book
    }

    fn main(&self) -> FileId {
        self.main.id()
    }

    fn source(&self, id: FileId) -> FileResult<Source> {
        if id == self.main.id() {
            Ok(self.main.clone())
        } else {
            self.slot(id, FileSlot::source)
        }
    }

    fn file(&self, id: FileId) -> FileResult<Bytes> {
        self.slot(id, FileSlot::file)
    }

    fn font(&self, index: usize) -> Option<Font> {
        Some(self.base.fonts[index].clone())
    }

    fn today(&self, _: Option<i64>) -> Option<Datetime> {
        Some(Datetime::from_ymd(1970, 1, 1).unwrap())
    }

    fn last_modified(&self, _id: FileId) -> FileResult<Option<Datetime>> {
        Ok(None)
    }
}

impl TestWorld {
    /// Access the canonical slot for the given file id.
    fn slot<F, T>(&self, id: FileId, f: F) -> T
    where
        F: FnOnce(&mut FileSlot) -> T,
    {
        let mut map = self.base.slots.lock();
        f(map.entry(id).or_insert_with(|| FileSlot::new(id)))
    }
}

/// Shared foundation of all test worlds.
struct TestBase {
    library: LazyHash<Library>,
    book: LazyHash<FontBook>,
    fonts: Vec<Font>,
    slots: Mutex<HashMap<FileId, FileSlot>>,
}

impl Default for TestBase {
    fn default() -> Self {
        let fonts: Vec<_> = typst_assets::fonts()
            .chain(typst_dev_assets::fonts())
            .flat_map(|data| Font::iter(Bytes::from_static(data)))
            .collect();

        Self {
            library: LazyHash::new(library()),
            book: LazyHash::new(FontBook::from_fonts(&fonts)),
            fonts,
            slots: Mutex::new(HashMap::new()),
        }
    }
}

/// Holds the processed data for a file ID.
#[derive(Clone)]
struct FileSlot {
    id: FileId,
    source: OnceLock<FileResult<Source>>,
    file: OnceLock<FileResult<Bytes>>,
}

impl FileSlot {
    /// Create a new file slot.
    fn new(id: FileId) -> Self {
        Self { id, file: OnceLock::new(), source: OnceLock::new() }
    }

    /// Retrieve the source for this file.
    fn source(&mut self) -> FileResult<Source> {
        self.source
            .get_or_init(|| {
                let buf = read(&system_path(self.id)?)?;
                let text = String::from_utf8(buf.into_owned())?;
                Ok(Source::new(self.id, text))
            })
            .clone()
    }

    /// Retrieve the file's bytes.
    fn file(&mut self) -> FileResult<Bytes> {
        self.file
            .get_or_init(|| {
                read(&system_path(self.id)?).map(|cow| match cow {
                    Cow::Owned(buf) => buf.into(),
                    Cow::Borrowed(buf) => Bytes::from_static(buf),
                })
            })
            .clone()
    }
}

/// The file system path for a file ID.
fn system_path(id: FileId) -> FileResult<PathBuf> {
    let root: PathBuf = match id.package() {
        Some(spec) => format!("tests/packages/{}-{}", spec.name, spec.version).into(),
        None => PathBuf::new(),
    };

    id.vpath().resolve(&root).ok_or(FileError::AccessDenied)
}

/// Read a file.
fn read(path: &Path) -> FileResult<Cow<'static, [u8]>> {
    // Resolve asset.
    if let Ok(suffix) = path.strip_prefix("assets/") {
        return typst_dev_assets::get(&suffix.to_string_lossy())
            .map(Cow::Borrowed)
            .ok_or_else(|| FileError::NotFound(path.into()));
    }

    let f = |e| FileError::from_io(e, path);
    if fs::metadata(path).map_err(f)?.is_dir() {
        Err(FileError::IsDirectory)
    } else {
        fs::read(path).map(Cow::Owned).map_err(f)
    }
}

/// The extended standard library for testing.
fn library() -> Library {
    // Set page width to 120pt with 10pt margins, so that the inner page is
    // exactly 100pt wide. Page height is unbounded and font size is 10pt so
    // that it multiplies to nice round numbers.
    let mut lib = Library::default();

    // Hook up helpers into the global scope.
    lib.global.scope_mut().define_func::<test>();
    lib.global.scope_mut().define_func::<test_repr>();
    lib.global.scope_mut().define_func::<print>();
    lib.global.scope_mut().define_func::<lines>();
    lib.global
        .scope_mut()
        .define("conifer", Color::from_u8(0x9f, 0xEB, 0x52, 0xFF));
    lib.global
        .scope_mut()
        .define("forest", Color::from_u8(0x43, 0xA1, 0x27, 0xFF));

    // Hook up default styles.
    lib.styles
        .set(PageElem::set_width(Smart::Custom(Abs::pt(120.0).into())));
    lib.styles.set(PageElem::set_height(Smart::Auto));
    lib.styles.set(PageElem::set_margin(Margin::splat(Some(Smart::Custom(
        Abs::pt(10.0).into(),
    )))));
    lib.styles.set(TextElem::set_size(TextSize(Abs::pt(10.0).into())));

    lib
}

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
    let mut out = std::io::stdout().lock();
    write!(out, "> ").unwrap();
    for (i, value) in values.into_iter().enumerate() {
        if i > 0 {
            write!(out, ", ").unwrap();
        }
        write!(out, "{value:?}").unwrap();
    }
    writeln!(out).unwrap();
    NoneValue
}

/// Generates `count` lines of text based on the numbering.
#[func]
fn lines(
    engine: &mut Engine,
    context: Tracked<Context>,
    span: Span,
    count: usize,
    #[default(Numbering::Pattern(NumberingPattern::from_str("A").unwrap()))]
    numbering: Numbering,
) -> SourceResult<Value> {
    (1..=count)
        .map(|n| numbering.apply(engine, context, &[n]))
        .collect::<SourceResult<Array>>()?
        .join(Some('\n'.into_value()), None)
        .at(span)
}
