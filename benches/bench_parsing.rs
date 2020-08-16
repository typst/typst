use criterion::{criterion_group, criterion_main, Criterion};
use typstc::syntax::parsing::parse;
use typstc::syntax::span::Pos;

// 28 not too dense lines.
const COMA: &str = include_str!("../tests/coma.typ");

fn parsing_benchmark(c: &mut Criterion) {
    c.bench_function("parse-coma-28-lines", |b| {
        b.iter(|| parse(COMA, Pos::ZERO))
    });

    let long = COMA.repeat(100);
    c.bench_function("parse-coma-2800-lines", |b| {
        b.iter(|| parse(&long, Pos::ZERO))
    });
}

criterion_group!(benches, parsing_benchmark);
criterion_main!(benches);
