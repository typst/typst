#![no_main]

use libfuzzer_sys::fuzz_target;
use typst::diag::{FileError, FileResult};
use typst::foundations::{Bytes, Datetime};
use typst::layout::PagedDocument;
use typst::syntax::{FileId, Source};
use typst::text::{Font, FontBook};
use typst::utils::LazyHash;
use typst::{Library, LibraryExt, World};

struct FuzzWorld {
    library: LazyHash<Library>,
    book: LazyHash<FontBook>,
    font: Font,
    source: Source,
}

impl FuzzWorld {
    fn new(text: &str) -> Self {
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

    fn main(&self) -> FileId {
        self.source.id()
    }

    fn source(&self, id: FileId) -> FileResult<Source> {
        if id == self.source.id() {
            Ok(self.source.clone())
        } else {
            Err(FileError::NotFound(id.vpath().as_rootless_path().into()))
        }
    }

    fn file(&self, id: FileId) -> FileResult<Bytes> {
        Err(FileError::NotFound(id.vpath().as_rootless_path().into()))
    }

    fn font(&self, _: usize) -> Option<Font> {
        Some(self.font.clone())
    }

    fn today(&self, _: Option<i64>) -> Option<Datetime> {
        None
    }
}

fuzz_target!(|text: &str| {
    let world = FuzzWorld::new(text);
    if let Ok(document) = typst::compile::<PagedDocument>(&world).output {
        if let Some(page) = document.pages.first() {
            std::hint::black_box(typst_render::render(page, 1.0));
        }
    }
    comemo::evict(10);
});
