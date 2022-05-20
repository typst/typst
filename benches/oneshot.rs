use std::path::Path;

use iai::{black_box, main, Iai};
use unscanny::Scanner;

use typst::loading::MemLoader;
use typst::parse::{parse, TokenMode, Tokens};
use typst::source::SourceId;
use typst::syntax::highlight_node;
use typst::Context;

const SRC: &str = include_str!("bench.typ");
const FONT: &[u8] = include_bytes!("../fonts/IBMPlexSans-Regular.ttf");

fn context() -> (Context, SourceId) {
    let loader = MemLoader::new().with(Path::new("font.ttf"), FONT).wrap();
    let mut ctx = Context::new(loader);
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
    iai.run(|| parse(SRC));
}

fn bench_edit(iai: &mut Iai) {
    let (mut ctx, id) = context();
    iai.run(|| black_box(ctx.sources.edit(id, 1168 .. 1171, "_Uhr_")));
}

fn bench_highlight(iai: &mut Iai) {
    let (ctx, id) = context();
    let source = ctx.sources.get(id);
    iai.run(|| {
        highlight_node(
            source.red().as_ref(),
            0 .. source.len_bytes(),
            &mut |_, _| {},
        )
    });
}

fn bench_eval(iai: &mut Iai) {
    let (mut ctx, id) = context();
    iai.run(|| ctx.evaluate(id).unwrap());
}

fn bench_layout(iai: &mut Iai) {
    let (mut ctx, id) = context();
    let module = ctx.evaluate(id).unwrap();
    iai.run(|| module.content.layout(&mut ctx));
}

fn bench_render(iai: &mut Iai) {
    let (mut ctx, id) = context();
    let frames = ctx.typeset(id).unwrap();
    iai.run(|| typst::export::render(&mut ctx, &frames[0], 1.0))
}
