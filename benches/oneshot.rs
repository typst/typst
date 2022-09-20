use std::io;
use std::path::Path;

use iai::{black_box, main, Iai};
use unscanny::Scanner;

use typst::font::{Font, FontBook};
use typst::parse::{TokenMode, Tokens};
use typst::source::{Source, SourceId};
use typst::util::Buffer;
use typst::{Config, World};

const TEXT: &str = include_str!("bench.typ");
const FONT: &[u8] = include_bytes!("../fonts/IBMPlexSans-Regular.ttf");

main!(
    bench_decode,
    bench_scan,
    bench_tokenize,
    bench_parse,
    bench_edit,
    bench_eval,
    bench_layout,
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
    iai.run(|| typst::parse::parse(TEXT));
}

fn bench_edit(iai: &mut Iai) {
    let mut source = Source::detached(TEXT);
    iai.run(|| black_box(source.edit(1168 .. 1171, "_Uhr_")));
}

fn bench_highlight(iai: &mut Iai) {
    let source = Source::detached(TEXT);
    iai.run(|| {
        typst::syntax::highlight_node(
            source.root(),
            0 .. source.len_bytes(),
            &mut |_, _| {},
        )
    });
}

fn bench_eval(iai: &mut Iai) {
    let world = BenchWorld::new();
    let id = world.source.id();
    iai.run(|| typst::eval::evaluate(&world, id, vec![]).unwrap());
}

fn bench_layout(iai: &mut Iai) {
    let world = BenchWorld::new();
    let id = world.source.id();
    let module = typst::eval::evaluate(&world, id, vec![]).unwrap();
    iai.run(|| typst::model::layout(&world, &module.content));
}

fn bench_render(iai: &mut Iai) {
    let world = BenchWorld::new();
    let id = world.source.id();
    let frames = typst::typeset(&world, id).unwrap();
    iai.run(|| typst::export::render(&frames[0], 1.0))
}

struct BenchWorld {
    config: Config,
    book: FontBook,
    font: Font,
    source: Source,
}

impl BenchWorld {
    fn new() -> Self {
        let font = Font::new(FONT.into(), 0).unwrap();
        let book = FontBook::from_fonts([&font]);
        let id = SourceId::from_raw(0);
        let source = Source::new(id, Path::new("bench.typ"), TEXT.into());
        Self {
            config: Config::default(),
            book,
            font,
            source,
        }
    }
}

impl World for BenchWorld {
    fn config(&self) -> &Config {
        &self.config
    }

    fn resolve(&self, _: &Path) -> io::Result<SourceId> {
        Err(io::ErrorKind::NotFound.into())
    }

    fn source(&self, _: SourceId) -> &Source {
        &self.source
    }

    fn book(&self) -> &FontBook {
        &self.book
    }

    fn font(&self, _: usize) -> io::Result<Font> {
        Ok(self.font.clone())
    }

    fn file(&self, _: &Path) -> io::Result<Buffer> {
        Err(io::ErrorKind::NotFound.into())
    }
}
