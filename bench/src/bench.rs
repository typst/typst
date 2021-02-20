use criterion::{criterion_group, criterion_main, Criterion};
use fontdock::fs::FsIndex;

use typst::env::{Env, ResourceLoader};
use typst::eval::eval;
use typst::exec::{exec, State};
use typst::export::pdf;
use typst::font::FsIndexExt;
use typst::layout::layout;
use typst::library;
use typst::parse::parse;
use typst::typeset;

const FONT_DIR: &str = "../fonts";
const COMA: &str = include_str!("../../tests/typ/full/coma.typ");

fn benchmarks(c: &mut Criterion) {
    macro_rules! bench {
        ($name:literal: $($tts:tt)*) => {
            c.bench_function($name, |b| b.iter(|| $($tts)*));
        };
    }

    let mut index = FsIndex::new();
    index.search_dir(FONT_DIR);

    let mut env = Env {
        fonts: index.into_dynamic_loader(),
        resources: ResourceLoader::new(),
    };

    let scope = library::new();
    let state = State::default();

    // Prepare intermediate results and run warm.
    let syntax_tree = parse(COMA).output;
    let expr_map = eval(&mut env, &syntax_tree, &scope).output;
    let layout_tree = exec(&mut env, &syntax_tree, &expr_map, state.clone()).output;
    let frames = layout(&mut env, &layout_tree);

    // Bench!
    bench!("parse-coma": parse(COMA));
    bench!("eval-coma": eval(&mut env, &syntax_tree, &scope));
    bench!("exec-coma": exec(&mut env, &syntax_tree, &expr_map, state.clone()));
    bench!("layout-coma": layout(&mut env, &layout_tree));
    bench!("typeset-coma": typeset(&mut env, COMA, &scope, state.clone()));
    bench!("export-pdf-coma": pdf::export(&env, &frames));
}

criterion_group!(benches, benchmarks);
criterion_main!(benches);
