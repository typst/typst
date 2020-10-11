use std::cell::RefCell;
use std::rc::Rc;

use criterion::{criterion_group, criterion_main, Criterion};
use fontdock::fs::{FsIndex, FsProvider};

use typstc::eval::{eval, State};
use typstc::font::FontLoader;
use typstc::parse::parse;
use typstc::typeset;

const FONT_DIR: &str = "fonts";
const COMA: &str = include_str!("../tests/coma.typ");

fn parse_benchmark(c: &mut Criterion) {
    c.bench_function("parse-coma", |b| b.iter(|| parse(COMA)));
}

fn eval_benchmark(c: &mut Criterion) {
    let tree = parse(COMA).output;
    let state = State::default();
    c.bench_function("eval-coma", |b| b.iter(|| eval(&tree, state.clone())));
}

fn typeset_benchmark(c: &mut Criterion) {
    let mut index = FsIndex::new();
    index.search_dir(FONT_DIR);

    let (descriptors, files) = index.into_vecs();
    let provider = FsProvider::new(files);
    let loader = FontLoader::new(Box::new(provider), descriptors);
    let loader = Rc::new(RefCell::new(loader));

    let state = State::default();
    c.bench_function("typeset-coma", |b| {
        b.iter(|| typeset(COMA, state.clone(), Rc::clone(&loader)))
    });
}

criterion_group!(benches, parse_benchmark, eval_benchmark, typeset_benchmark);
criterion_main!(benches);
