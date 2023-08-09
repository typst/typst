use comemo::{Prehashed, Track, Tracked};
use iai::{black_box, main, Iai};
use typst::diag::FileResult;
use typst::eval::{Bytes, Datetime, Library, Tracer};
use typst::font::{Font, FontBook};
use typst::geom::Color;
use typst::syntax::{FileId, Source};
use typst::World;
use unscanny::Scanner;

const TEXT: &str = include_str!("../typ/compiler/bench.typ");
const FONT: &[u8] = include_bytes!("../../assets/fonts/LinLibertine_R.ttf");

main!(
    bench_decode,
    bench_scan,
    bench_parse,
    bench_edit,
    bench_eval,
    bench_typeset,
    bench_compile,
    bench_render,
);

fn bench_decode(iai: &mut Iai) {
    iai.run(|| {
        // We don't use chars().count() because that has a special
        // superfast implementation.
        let mut count = 0;
        let chars = black_box(TEXT).chars();
        for _ in chars {
            count += 1;
        }
        count
    })
}

fn bench_scan(iai: &mut Iai) {
    iai.run(|| {
        let mut count = 0;
        let mut scanner = Scanner::new(black_box(TEXT));
        while scanner.eat().is_some() {
            count += 1;
        }
        count
    })
}

fn bench_parse(iai: &mut Iai) {
    iai.run(|| typst::syntax::parse(TEXT));
}

fn bench_edit(iai: &mut Iai) {
    let mut source = Source::detached(TEXT);
    iai.run(|| black_box(source.edit(1168..1171, "_Uhr_")));
}

fn bench_eval(iai: &mut Iai) {
    let world = BenchWorld::new();
    let route = typst::eval::Route::default();
    let mut tracer = typst::eval::Tracer::default();
    iai.run(|| {
        typst::eval::eval(world.track(), route.track(), tracer.track_mut(), &world.source)
            .unwrap()
    });
}

fn bench_typeset(iai: &mut Iai) {
    let world = BenchWorld::new();
    let route = typst::eval::Route::default();
    let mut tracer = typst::eval::Tracer::default();
    let module = typst::eval::eval(
        world.track(),
        route.track(),
        tracer.track_mut(),
        &world.source,
    )
    .unwrap();
    let content = module.content();
    iai.run(|| typst::model::typeset(world.track(), tracer.track_mut(), &content));
}

fn bench_compile(iai: &mut Iai) {
    let world = BenchWorld::new();
    let mut tracer = Tracer::default();
    iai.run(|| typst::compile(&world, &mut tracer));
}

fn bench_render(iai: &mut Iai) {
    let world = BenchWorld::new();
    let mut tracer = Tracer::default();
    let document = typst::compile(&world, &mut tracer).unwrap();
    iai.run(|| typst::export::render(&document.pages[0], 1.0, Color::WHITE))
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
