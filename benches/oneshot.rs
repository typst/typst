use std::io;
use std::path::Path;
use std::sync::Arc;

use iai::{black_box, main, Iai};
use unscanny::Scanner;

use typst::font::{Font, FontBook};
use typst::loading::{Buffer, FileHash, Loader};
use typst::parse::{TokenMode, Tokens};
use typst::source::SourceId;
use typst::{Config, Context};

const SRC: &str = include_str!("bench.typ");
const FONT: &[u8] = include_bytes!("../fonts/IBMPlexSans-Regular.ttf");

fn context() -> (Context, SourceId) {
    let loader = BenchLoader::new();
    let mut ctx = Context::new(Arc::new(loader), Config::default());
    let id = ctx.sources.provide(Path::new("src.typ"), SRC.to_string());
    (ctx, id)
}

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
        let mut chars = black_box(SRC).chars();
        while let Some(_) = chars.next() {
            count += 1;
        }
        count
    })
}

fn bench_scan(iai: &mut Iai) {
    iai.run(|| {
        let mut count = 0;
        let mut scanner = Scanner::new(black_box(SRC));
        while let Some(_) = scanner.eat() {
            count += 1;
        }
        count
    })
}

fn bench_tokenize(iai: &mut Iai) {
    iai.run(|| Tokens::new(black_box(SRC), black_box(TokenMode::Markup)).count());
}

fn bench_parse(iai: &mut Iai) {
    iai.run(|| typst::parse::parse(SRC));
}

fn bench_edit(iai: &mut Iai) {
    let (mut ctx, id) = context();
    iai.run(|| black_box(ctx.sources.edit(id, 1168 .. 1171, "_Uhr_")));
}

fn bench_highlight(iai: &mut Iai) {
    let (ctx, id) = context();
    let source = ctx.sources.get(id);
    iai.run(|| {
        typst::syntax::highlight_node(
            source.root(),
            0 .. source.len_bytes(),
            &mut |_, _| {},
        )
    });
}

fn bench_eval(iai: &mut Iai) {
    let (mut ctx, id) = context();
    iai.run(|| typst::eval::evaluate(&mut ctx, id, vec![]).unwrap());
}

fn bench_layout(iai: &mut Iai) {
    let (mut ctx, id) = context();
    let module = typst::eval::evaluate(&mut ctx, id, vec![]).unwrap();
    iai.run(|| typst::model::layout(&mut ctx, &module.content));
}

fn bench_render(iai: &mut Iai) {
    let (mut ctx, id) = context();
    let frames = typst::typeset(&mut ctx, id).unwrap();
    iai.run(|| typst::export::render(&frames[0], 1.0))
}

struct BenchLoader {
    book: FontBook,
    font: Font,
}

impl BenchLoader {
    fn new() -> Self {
        let font = Font::new(FONT.into(), 0).unwrap();
        let book = FontBook::from_fonts([&font]);
        Self { book, font }
    }
}

impl Loader for BenchLoader {
    fn book(&self) -> &FontBook {
        &self.book
    }

    fn font(&self, _: usize) -> io::Result<Font> {
        Ok(self.font.clone())
    }

    fn resolve(&self, _: &Path) -> io::Result<FileHash> {
        Err(io::ErrorKind::NotFound.into())
    }

    fn file(&self, _: &Path) -> io::Result<Buffer> {
        Err(io::ErrorKind::NotFound.into())
    }
}
