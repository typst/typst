use std::cell::RefCell;
use std::rc::Rc;

use criterion::{criterion_group, criterion_main, Criterion};
use fontdock::fs::{FsIndex, FsProvider};
use futures_executor::block_on;

use typstc::font::FontLoader;
use typstc::parse::parse;
use typstc::typeset;

const FONT_DIR: &str = "fonts";
const COMA: &str = include_str!("../tests/coma.typ");

fn parsing_benchmark(c: &mut Criterion) {
    c.bench_function("parse-coma", |b| b.iter(|| parse(COMA)));
}

fn typesetting_benchmark(c: &mut Criterion) {
    let mut index = FsIndex::new();
    index.search_dir(FONT_DIR);

    let (descriptors, files) = index.into_vecs();
    let provider = FsProvider::new(files);
    let loader = FontLoader::new(Box::new(provider), descriptors);
    let loader = Rc::new(RefCell::new(loader));

    let style = Default::default();
    let scope = typstc::library::_std();
    c.bench_function("typeset-coma", |b| {
        b.iter(|| block_on(typeset(COMA, &style, &scope, Rc::clone(&loader))))
    });
}

criterion_group!(benches, parsing_benchmark, typesetting_benchmark);
criterion_main!(benches);
