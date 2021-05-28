use std::path::Path;
use std::rc::Rc;

use criterion::{criterion_group, criterion_main, Criterion};

use typst::eval::eval;
use typst::exec::exec;
use typst::export::pdf;
use typst::layout::layout;
use typst::loading::FsLoader;
use typst::parse::parse;
use typst::typeset;

const FONT_DIR: &str = "../fonts";
const TYP_DIR: &str = "../tests/typ";
const CASES: &[&str] = &["full/coma.typ", "text/basic.typ"];

fn benchmarks(c: &mut Criterion) {
    let mut loader = FsLoader::new();
    loader.search_path(FONT_DIR);

    let mut cache = typst::cache::Cache::new(&loader);
    let scope = typst::library::new();
    let state = typst::exec::State::default();

    for case in CASES {
        let case = Path::new(case);
        let name = case.file_stem().unwrap().to_string_lossy();

        macro_rules! bench {
            ($step:literal: $code:expr) => {
                c.bench_function(&format!("{}-{}", $step, name), |b| {
                    b.iter(|| {
                        cache.layout.clear();
                        $code
                    });
                });
            };
        }

        // Prepare intermediate results, run warm and fill caches.
        let src = std::fs::read_to_string(Path::new(TYP_DIR).join(case)).unwrap();
        let parsed = Rc::new(parse(&src).output);
        let evaluated = eval(&mut loader, &mut cache, parsed.clone(), &scope).output;
        let executed = exec(&evaluated.template, state.clone()).output;
        let layouted = layout(&mut loader, &mut cache, &executed);

        // Bench!
        bench!("parse": parse(&src));
        bench!("eval": eval(&mut loader, &mut cache, parsed.clone(), &scope));
        bench!("exec": exec(&evaluated.template, state.clone()));
        bench!("layout": layout(&mut loader, &mut cache, &executed));
        bench!("typeset": typeset(&mut loader, &mut cache, &src, &scope, state.clone()));
        bench!("pdf": pdf(&cache, &layouted));
    }
}

criterion_group!(benches, benchmarks);
criterion_main!(benches);
