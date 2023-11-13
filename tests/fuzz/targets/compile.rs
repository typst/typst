#![no_main]
use comemo::{Prehashed};

use typst::diag::FileResult;
use typst::eval::{Bytes, Datetime, Library, Tracer};
use typst::font::{Font, FontBook};
use typst::geom::Color;
use typst::syntax::{FileId, Source};
use typst::World;

const FONT: &[u8] = include_bytes!("../../../assets/fonts/LinLibertine_R.ttf");

use libfuzzer_sys::fuzz_target;

struct FuzzWorld {
    library: Prehashed<Library>,
    book: Prehashed<FontBook>,
    font: Font,
    source: Source,
}

impl FuzzWorld {
    fn new(text: &str) -> Self {
        let font = Font::new(FONT.into(), 0).unwrap();
        let book = FontBook::from_fonts([&font]);

        Self {
            library: Prehashed::new(typst_library::build()),
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

    fn source(&self, _: FileId) -> FileResult<Source> {
        unimplemented!()
    }

    fn file(&self, _: FileId) -> FileResult<Bytes> {
        unimplemented!()
    }

    fn font(&self, _: usize) -> Option<Font> {
        Some(self.font.clone())
    }

    fn today(&self, _: Option<i64>) -> Option<Datetime> {
        unimplemented!()
    }
}

fuzz_target!(|text: &str| {
    let world = FuzzWorld::new(&text);
    let mut tracer = Tracer::new();
    if let Ok(document) = typst::compile(&world, &mut tracer) {
        typst_render::render(&document.pages[0], 1.0, Color::WHITE);
    }
});
