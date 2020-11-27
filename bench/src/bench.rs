use std::cell::RefCell;
use std::rc::Rc;

use criterion::{criterion_group, criterion_main, Criterion};
use fontdock::fs::{FsIndex, FsSource};

use typst::eval::{eval, State};
use typst::export::pdf;
use typst::font::FontLoader;
use typst::layout::layout;
use typst::parse::parse;
use typst::typeset;

const FONT_DIR: &str = "../fonts";
const COMA: &str = include_str!("../../tests/typ/coma.typ");

fn benchmarks(c: &mut Criterion) {
    macro_rules! bench {
        ($name:literal: $($tts:tt)*) => {
            c.bench_function($name, |b| b.iter(|| $($tts)*));
        };
    }

    let mut index = FsIndex::new();
    index.search_dir(FONT_DIR);

    let (files, descriptors) = index.into_vecs();
    let loader = Rc::new(RefCell::new(FontLoader::new(
        Box::new(FsSource::new(files)),
        descriptors,
    )));

    // Prepare intermediate results and run warm.
    let state = State::default();
    let tree = parse(COMA).output;
    let document = eval(&tree, state.clone()).output;
    let layouts = layout(&document, Rc::clone(&loader));

    // Bench!
    bench!("parse-coma": parse(COMA));
    bench!("eval-coma": eval(&tree, state.clone()));
    bench!("layout-coma": layout(&document, Rc::clone(&loader)));
    bench!("typeset-coma": typeset(COMA, state.clone(), Rc::clone(&loader)));
    bench!("export-pdf-coma": pdf::export(&layouts, &loader.borrow()));
}

criterion_group!(benches, benchmarks);
criterion_main!(benches);
