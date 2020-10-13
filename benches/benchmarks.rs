use std::cell::RefCell;
use std::rc::Rc;

use criterion::{criterion_group, criterion_main, Criterion};
use fontdock::fs::{FsIndex, FsSource};

use typst::eval::{eval, State};
use typst::font::FontLoader;
use typst::layout::layout;
use typst::parse::parse;
use typst::typeset;

const FONT_DIR: &str = "fonts";
const COMA: &str = include_str!("../tests/typ/coma.typ");

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
    let _ = layout(&document, Rc::clone(&loader));

    c.bench_function("parse-coma", |b| b.iter(|| parse(COMA)));
    c.bench_function("eval-coma", |b| b.iter(|| eval(&tree, state.clone())));
    c.bench_function("layout-coma", |b| {
        b.iter(|| layout(&document, Rc::clone(&loader)))
    });
    c.bench_function("typeset-coma", |b| {
        b.iter(|| typeset(COMA, state.clone(), Rc::clone(&loader)))
    });
}

criterion_group!(benches, benchmarks);
criterion_main!(benches);
