use std::cell::RefCell;
use std::rc::Rc;

use criterion::{criterion_group, criterion_main, Criterion};
use fontdock::fs::{FsIndex, FsSource};

use typst::env::{Env, ResourceLoader};
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
    let env = Rc::new(RefCell::new(Env {
        fonts: FontLoader::new(Box::new(FsSource::new(files)), descriptors),
        resources: ResourceLoader::new(),
    }));

    // Prepare intermediate results and run warm.
    let state = State::default();
    let tree = parse(COMA).output;
    let document = eval(&tree, Rc::clone(&env), state.clone()).output;
    let layouts = layout(&document, Rc::clone(&env));

    // Bench!
    bench!("parse-coma": parse(COMA));
    bench!("eval-coma": eval(&tree, Rc::clone(&env), state.clone()));
    bench!("layout-coma": layout(&document, Rc::clone(&env)));
    bench!("typeset-coma": typeset(COMA, Rc::clone(&env), state.clone()));

    let env = env.borrow();
    bench!("export-pdf-coma": pdf::export(&layouts, &env));
}

criterion_group!(benches, benchmarks);
criterion_main!(benches);
