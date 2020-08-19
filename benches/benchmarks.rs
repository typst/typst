use std::cell::RefCell;
use std::rc::Rc;

use criterion::{criterion_group, criterion_main, Criterion};
use fontdock::fs::{FsIndex, FsProvider};
use fontdock::FontLoader;

use typstc::font::DynProvider;
use typstc::syntax::parsing::parse;
use typstc::Typesetter;

const FONT_DIR: &str = "fonts";

// 28 not too dense lines.
const COMA: &str = include_str!("../tests/coma.typ");

fn parsing_benchmark(c: &mut Criterion) {
    c.bench_function("parse-coma-28-lines", |b| {
        b.iter(|| parse(COMA))
    });

    let long = COMA.repeat(100);
    c.bench_function("parse-coma-2800-lines", |b| {
        b.iter(|| parse(&long))
    });
}

fn typesetting_benchmark(c: &mut Criterion) {
    let mut index = FsIndex::new();
    index.search_dir(FONT_DIR);

    let (descriptors, files) = index.clone().into_vecs();
    let provider = FsProvider::new(files.clone());
    let dynamic = Box::new(provider) as Box<DynProvider>;
    let loader = FontLoader::new(dynamic, descriptors);
    let loader = Rc::new(RefCell::new(loader));
    let typesetter = Typesetter::new(loader.clone());

    c.bench_function("typeset-coma-28-lines", |b| {
        b.iter(|| futures_executor::block_on(typesetter.typeset(COMA)))
    });

    let long = COMA.repeat(100);
    c.bench_function("typeset-coma-2800-lines", |b| {
        b.iter(|| futures_executor::block_on(typesetter.typeset(&long)))
    });
}

criterion_group!(benches, parsing_benchmark, typesetting_benchmark);
criterion_main!(benches);
