use std::path::Path;

use criterion::{criterion_group, criterion_main, Criterion};

use typst::env::{Env, FsLoader};
use typst::eval::eval;
use typst::exec::{exec, State};
use typst::layout::layout;
use typst::library;
use typst::parse::parse;
use typst::pdf;
use typst::typeset;

const FONT_DIR: &str = "../fonts";
const TYP_DIR: &str = "../tests/typ";
const CASES: &[&str] = &["full/coma.typ", "text/basic.typ"];

fn benchmarks(c: &mut Criterion) {
    let mut loader = FsLoader::new();
    loader.search_path(FONT_DIR);

    let mut env = Env::new(loader);

    let scope = library::_new();
    let state = State::default();

    for case in CASES {
        let case = Path::new(case);
        let name = case.file_stem().unwrap().to_string_lossy();
        let src = std::fs::read_to_string(Path::new(TYP_DIR).join(case)).unwrap();

        macro_rules! bench {
            ($step:literal: $($tts:tt)*) => {
                c.bench_function(
                    &format!("{}-{}", $step, name),
                    |b| b.iter(|| $($tts)*)
                );
            };
        }

        // Prepare intermediate results and run warm.
        let syntax_tree = parse(&src).output;
        let expr_map = eval(&mut env, &syntax_tree, &scope).output;
        let layout_tree = exec(&mut env, &syntax_tree, &expr_map, state.clone()).output;
        let frames = layout(&mut env, &layout_tree);

        // Bench!
        bench!("parse": parse(&src));
        bench!("eval": eval(&mut env, &syntax_tree, &scope));
        bench!("exec": exec(&mut env, &syntax_tree, &expr_map, state.clone()));
        bench!("layout": layout(&mut env, &layout_tree));
        bench!("typeset": typeset(&mut env, &src, &scope, state.clone()));
        bench!("pdf": pdf::export(&env, &frames));
    }
}

criterion_group!(benches, benchmarks);
criterion_main!(benches);
