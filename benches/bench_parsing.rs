use criterion::{criterion_group, criterion_main, Criterion};
use typstc::library::_std;
use typstc::syntax::parsing::{parse, ParseState};
use typstc::syntax::span::Pos;

// 28 not too dense lines.
const COMA: &str = include_str!("../tests/coma.typ");

fn parsing_benchmark(c: &mut Criterion) {
    let state = ParseState { scope: _std() };

    c.bench_function("parse-coma-28-lines", |b| {
        b.iter(|| parse(COMA, Pos::ZERO, &state))
    });

    // 2800 lines of Typst code.
    let long = COMA.repeat(100);
    c.bench_function("parse-coma-2800-lines", |b| {
        b.iter(|| parse(&long, Pos::ZERO, &state))
    });
}

criterion_group!(benches, parsing_benchmark);
criterion_main!(benches);
