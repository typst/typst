use std::cell::RefCell;
use std::rc::Rc;

use criterion::{criterion_group, criterion_main, Criterion};
use fontdock::fs::{FsIndex, FsSource};
use futures_executor::block_on;

use typstc::eval::{eval, State};
use typstc::font::FontLoader;
use typstc::layout::layout;
use typstc::parse::parse;
use typstc::typeset;

const FONT_DIR: &str = "fonts";
const COMA: &str = include_str!("../tests/coma.typ");

fn benchmarks(c: &mut Criterion) {
    let state = State::default();

    let mut index = FsIndex::new();
    index.search_dir(FONT_DIR);

    let (files, descriptors) = index.into_vecs();
    let loader = Rc::new(RefCell::new(FontLoader::new(
        Box::new(FsSource::new(files)),
        descriptors,
    )));

    let tree = parse(COMA).output;
    let document = eval(&tree, state.clone()).output;
    let _ = block_on(layout(&document, Rc::clone(&loader)));

    c.bench_function("parse-coma", |b| b.iter(|| parse(COMA)));
    c.bench_function("eval-coma", |b| b.iter(|| eval(&tree, state.clone())));
    c.bench_function("layout-coma", |b| {
        b.iter(|| block_on(layout(&document, Rc::clone(&loader))))
    });
    c.bench_function("typeset-coma", |b| {
        b.iter(|| block_on(typeset(COMA, state.clone(), Rc::clone(&loader))))
    });
}

criterion_group!(benches, benchmarks);
criterion_main!(benches);
