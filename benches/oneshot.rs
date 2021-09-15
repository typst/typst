use std::path::Path;

use iai::{black_box, main, Iai};

use typst::eval::eval;
use typst::layout::layout;
use typst::loading::MemLoader;
use typst::parse::{parse_markup, Scanner, TokenMode, Tokens};
use typst::source::{SourceFile, SourceId};
use typst::Context;

const SRC: &str = include_str!("bench.typ");

fn context() -> (Context, SourceId) {
    let font = include_bytes!("../fonts/EBGaramond-Regular.ttf");
    let loader = MemLoader::new()
        .with(Path::new("EBGaramond-Regular.ttf"), &font[..])
        .wrap();
    let mut ctx = Context::new(loader);
    let id = ctx.sources.provide(Path::new(""), SRC.to_string());
    (ctx, id)
}

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
    iai.run(|| parse_markup(&SourceFile::detached(SRC)));
}

fn bench_eval(iai: &mut Iai) {
    let (mut ctx, id) = context();
    let markup = ctx.parse(id).unwrap();
    iai.run(|| eval(&mut ctx, id, &markup).unwrap());
}

fn bench_to_tree(iai: &mut Iai) {
    let (mut ctx, id) = context();
    let template = ctx.evaluate(id).unwrap();
    iai.run(|| template.to_tree(ctx.state()));
}

fn bench_layout(iai: &mut Iai) {
    let (mut ctx, id) = context();
    let tree = ctx.execute(id).unwrap();
    iai.run(|| layout(&mut ctx, &tree));
}

main!(
    bench_decode,
    bench_scan,
    bench_tokenize,
    bench_parse,
    bench_eval,
    bench_to_tree,
    bench_layout
);
