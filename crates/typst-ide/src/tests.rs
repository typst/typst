use std::borrow::Borrow;
use std::collections::HashMap;
use std::sync::Arc;

use ecow::EcoString;
use typst::diag::{FileError, FileResult};
use typst::foundations::{Bytes, Datetime, Smart};
use typst::layout::{Abs, Margin, PageElem};
use typst::syntax::package::{PackageSpec, PackageVersion};
use typst::syntax::{FileId, Source, VirtualPath};
use typst::text::{Font, FontBook, TextElem, TextSize};
use typst::utils::{singleton, LazyHash};
use typst::{Library, World};

use crate::IdeWorld;

/// A world for IDE testing.
#[derive(Clone)]
pub struct TestWorld {
    pub main: Source,
    files: Arc<TestFiles>,
    base: &'static TestBase,
}

impl TestWorld {
    /// Create a new world for a single test.
    ///
    /// This is cheap because the shared base for all test runs is lazily
    /// initialized just once.
    pub fn new(text: &str) -> Self {
        let main = Source::new(Self::main_id(), text.into());
        Self {
            main,
            files: Arc::new(TestFiles::default()),
            base: singleton!(TestBase, TestBase::default()),
        }
    }

    /// Add an additional source file to the test world.
    pub fn with_source(mut self, path: &str, text: &str) -> Self {
        let id = FileId::new(None, VirtualPath::new(path));
        let source = Source::new(id, text.into());
        Arc::make_mut(&mut self.files).sources.insert(id, source);
        self
    }

    /// Add an additional asset file to the test world.
    #[track_caller]
    pub fn with_asset(self, filename: &str) -> Self {
        self.with_asset_at(filename, filename)
    }

    /// Add an additional asset file to the test world.
    #[track_caller]
    pub fn with_asset_at(mut self, path: &str, filename: &str) -> Self {
        let id = FileId::new(None, VirtualPath::new(path));
        let data = typst_dev_assets::get_by_name(filename).unwrap();
        let bytes = Bytes::from_static(data);
        Arc::make_mut(&mut self.files).assets.insert(id, bytes);
        self
    }

    /// The ID of the main file in a `TestWorld`.
    pub fn main_id() -> FileId {
        *singleton!(FileId, FileId::new(None, VirtualPath::new("main.typ")))
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
        } else if let Some(source) = self.files.sources.get(&id) {
            Ok(source.clone())
        } else {
            Err(FileError::NotFound(id.vpath().as_rootless_path().into()))
        }
    }

    fn file(&self, id: FileId) -> FileResult<Bytes> {
        match self.files.assets.get(&id) {
            Some(bytes) => Ok(bytes.clone()),
            None => Err(FileError::NotFound(id.vpath().as_rootless_path().into())),
        }
    }

    fn font(&self, index: usize) -> Option<Font> {
        Some(self.base.fonts[index].clone())
    }

    fn today(&self, _: Option<i64>) -> Option<Datetime> {
        None
    }
}

impl IdeWorld for TestWorld {
    fn upcast(&self) -> &dyn World {
        self
    }

    fn files(&self) -> Vec<FileId> {
        std::iter::once(self.main.id())
            .chain(self.files.sources.keys().copied())
            .chain(self.files.assets.keys().copied())
            .collect()
    }

    fn packages(&self) -> &[(PackageSpec, Option<EcoString>)] {
        const LIST: &[(PackageSpec, Option<EcoString>)] = &[(
            PackageSpec {
                // NOTE: This literal, `"preview"`, should match the const, `DEFAULT_NAMESPACE`,
                // defined in `crates/typst-kit/src/package.rs`. However, we should always use the
                // literal here, not `DEFAULT_NAMESPACE`, so that this test fails if its value
                // changes in an unexpected way.
                namespace: EcoString::inline("preview"),
                name: EcoString::inline("example"),
                version: PackageVersion { major: 0, minor: 1, patch: 0 },
            },
            None,
        )];
        LIST
    }
}

/// Test-specific files.
#[derive(Default, Clone)]
struct TestFiles {
    assets: HashMap<FileId, Bytes>,
    sources: HashMap<FileId, Source>,
}

/// Shared foundation of all test worlds.
struct TestBase {
    library: LazyHash<Library>,
    book: LazyHash<FontBook>,
    fonts: Vec<Font>,
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
        }
    }
}

/// The extended standard library for testing.
fn library() -> Library {
    // Set page width to 120pt with 10pt margins, so that the inner page is
    // exactly 100pt wide. Page height is unbounded and font size is 10pt so
    // that it multiplies to nice round numbers.
    let mut lib = typst::Library::default();
    lib.styles
        .set(PageElem::set_width(Smart::Custom(Abs::pt(120.0).into())));
    lib.styles.set(PageElem::set_height(Smart::Auto));
    lib.styles.set(PageElem::set_margin(Margin::splat(Some(Smart::Custom(
        Abs::pt(10.0).into(),
    )))));
    lib.styles.set(TextElem::set_size(TextSize(Abs::pt(10.0).into())));
    lib
}

/// The input to a test: Either just a string or a full `TestWorld`.
pub trait WorldLike {
    type World: Borrow<TestWorld>;

    fn acquire(self) -> Self::World;
}

impl<'a> WorldLike for &'a TestWorld {
    type World = &'a TestWorld;

    fn acquire(self) -> Self::World {
        self
    }
}

impl WorldLike for &str {
    type World = TestWorld;

    fn acquire(self) -> Self::World {
        TestWorld::new(self)
    }
}

/// Extra methods for [`Source`].
pub trait SourceExt {
    /// Negative cursors index from the back.
    fn cursor(&self, cursor: isize) -> usize;
}

impl SourceExt for Source {
    fn cursor(&self, cursor: isize) -> usize {
        if cursor < 0 {
            self.len_bytes().checked_add_signed(cursor + 1).unwrap()
        } else {
            cursor as usize
        }
    }
}
