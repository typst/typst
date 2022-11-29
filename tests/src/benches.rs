use std::path::Path;

use comemo::{Prehashed, Track, Tracked};
use iai::{black_box, main, Iai};
use typst::diag::{FileError, FileResult};
use typst::font::{Font, FontBook};
use typst::model::Library;
use typst::syntax::{Source, SourceId, TokenMode, Tokens};
use typst::util::Buffer;
use typst::World;
use unscanny::Scanner;

const TEXT: &str = include_str!("../typ/compiler/bench.typ");
const FONT: &[u8] = include_bytes!("../fonts/IBMPlexSans-Regular.ttf");

main!(
    bench_decode,
    bench_scan,
    bench_tokenize,
    bench_parse,
    bench_edit,
    bench_eval,
    bench_typeset,
    bench_compile,
    bench_highlight,
    bench_render,
);

fn bench_decode(iai: &mut Iai) {
    iai.run(|| {
        // We don't use chars().count() because that has a special
        // superfast implementation.
        let mut count = 0;
        let mut chars = black_box(TEXT).chars();
        while let Some(_) = chars.next() {
            count += 1;
        }
        count
    })
}

fn bench_scan(iai: &mut Iai) {
    iai.run(|| {
        let mut count = 0;
        let mut scanner = Scanner::new(black_box(TEXT));
        while let Some(_) = scanner.eat() {
            count += 1;
        }
        count
    })
}

fn bench_tokenize(iai: &mut Iai) {
    iai.run(|| Tokens::new(black_box(TEXT), black_box(TokenMode::Markup)).count());
}

fn bench_parse(iai: &mut Iai) {
    iai.run(|| typst::syntax::parse(TEXT));
}

fn bench_edit(iai: &mut Iai) {
    let mut source = Source::detached(TEXT);
    iai.run(|| black_box(source.edit(1168..1171, "_Uhr_")));
}

fn bench_highlight(iai: &mut Iai) {
    let source = Source::detached(TEXT);
    iai.run(|| {
        typst::syntax::highlight::highlight_categories(
            source.root(),
            0..source.len_bytes(),
            &mut |_, _| {},
        )
    });
}

fn bench_eval(iai: &mut Iai) {
    let world = BenchWorld::new();
    let route = typst::model::Route::default();
    iai.run(|| typst::model::eval(world.track(), route.track(), &world.source).unwrap());
}

fn bench_typeset(iai: &mut Iai) {
    let world = BenchWorld::new();
    let route = typst::model::Route::default();
    let module = typst::model::eval(world.track(), route.track(), &world.source).unwrap();
    iai.run(|| typst::model::typeset(world.track(), &module.content));
}

fn bench_compile(iai: &mut Iai) {
    let world = BenchWorld::new();
    iai.run(|| typst::compile(&world, &world.source));
}

fn bench_render(iai: &mut Iai) {
    let world = BenchWorld::new();
    let document = typst::compile(&world, &world.source).unwrap();
    iai.run(|| typst::export::render(&document.pages[0], 1.0))
}

struct BenchWorld {
    library: Prehashed<Library>,
    book: Prehashed<FontBook>,
    font: Font,
    source: Source,
}

impl BenchWorld {
    fn new() -> Self {
        let font = Font::new(FONT.into(), 0).unwrap();
        let book = FontBook::from_fonts([&font]);

        Self {
            library: Prehashed::new(typst_library::build()),
            book: Prehashed::new(book),
            font,
            source: Source::detached(TEXT),
        }
    }

    fn track(&self) -> Tracked<dyn World> {
        (self as &dyn World).track()
    }
}

impl World for BenchWorld {
    fn root(&self) -> &Path {
        Path::new("")
    }

    fn library(&self) -> &Prehashed<Library> {
        &self.library
    }

    fn book(&self) -> &Prehashed<FontBook> {
        &self.book
    }

    fn font(&self, _: usize) -> Option<Font> {
        Some(self.font.clone())
    }

    fn file(&self, path: &Path) -> FileResult<Buffer> {
        Err(FileError::NotFound(path.into()))
    }

    fn resolve(&self, path: &Path) -> FileResult<SourceId> {
        Err(FileError::NotFound(path.into()))
    }

    fn source(&self, _: SourceId) -> &Source {
        unimplemented!()
    }
}
