use typst::diag::{FileError, FileResult};
use typst::foundations::{Bytes, Datetime};
use typst::syntax::Source;
use typst::syntax::path::VirtualPath;
use typst::text::{Font, FontBook};
use typst::utils::{Id, LazyHash};
use typst::{Library, LibraryExt, World};

pub struct FuzzWorld {
    library: LazyHash<Library>,
    book: LazyHash<FontBook>,
    font: Font,
    source: Source,
}

impl FuzzWorld {
    pub fn new(text: &str) -> Self {
        let data = typst_assets::fonts().next().unwrap();
        let font = Font::new(Bytes::new(data), 0).unwrap();
        let book = FontBook::from_fonts([&font]);
        Self {
            library: LazyHash::new(Library::default()),
            book: LazyHash::new(book),
            font,
            source: Source::detached(text),
        }
    }
}

impl World for FuzzWorld {
    fn library(&self) -> &LazyHash<Library> {
        &self.library
    }

    fn book(&self) -> &LazyHash<FontBook> {
        &self.book
    }

    fn main(&self) -> Id<VirtualPath> {
        self.source.path()
    }

    fn source(&self, path: Id<VirtualPath>) -> FileResult<Source> {
        if path == self.source.path() {
            Ok(self.source.clone())
        } else {
            Err(FileError::NotFound(path.get_without_slash().into()))
        }
    }

    fn file(&self, path: Id<VirtualPath>) -> FileResult<Bytes> {
        Err(FileError::NotFound(path.get_without_slash().into()))
    }

    fn font(&self, _: usize) -> Option<Font> {
        Some(self.font.clone())
    }

    fn today(&self, _: Option<i64>) -> Option<Datetime> {
        None
    }
}
