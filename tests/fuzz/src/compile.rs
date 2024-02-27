#![no_main]

use comemo::Prehashed;
use libfuzzer_sys::fuzz_target;
use typst::diag::{FileError, FileResult};
use typst::eval::Tracer;
use typst::foundations::{Bytes, Datetime};
use typst::syntax::{FileId, Source};
use typst::text::{Font, FontBook};
use typst::visualize::Color;
use typst::{Library, World};

struct FuzzWorld {
    library: Prehashed<Library>,
    book: Prehashed<FontBook>,
    font: Font,
    source: Source,
}

impl FuzzWorld {
    fn new(text: &str) -> Self {
        let data = typst_assets::fonts().next().unwrap();
        let font = Font::new(Bytes::from_static(data), 0).unwrap();
        let book = FontBook::from_fonts([&font]);
        Self {
            library: Prehashed::new(Library::default()),
            book: Prehashed::new(book),
            font,
            source: Source::detached(text),
        }
    }
}

impl World for FuzzWorld {
    fn library(&self) -> &Prehashed<Library> {
        &self.library
    }

    fn book(&self) -> &Prehashed<FontBook> {
        &self.book
    }

    fn main(&self) -> Source {
        self.source.clone()
    }

    fn source(&self, src: FileId) -> FileResult<Source> {
        Err(FileError::NotFound(src.vpath().as_rootless_path().into()))
    }

    fn file(&self, src: FileId) -> FileResult<Bytes> {
        Err(FileError::NotFound(src.vpath().as_rootless_path().into()))
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
    let mut tracer = Tracer::new();
    if let Ok(document) = typst::compile(&world, &mut tracer) {
        if let Some(page) = document.pages.first() {
            std::hint::black_box(typst_render::render(&page.frame, 1.0, Color::WHITE));
        }
    }
    comemo::evict(10);
});
