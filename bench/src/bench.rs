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
        let path = Path::new(TYP_DIR).join(case);
        let name = path.file_stem().unwrap().to_string_lossy();

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
        let src = std::fs::read_to_string(&path).unwrap();
        let tree = Rc::new(parse(&src).output);
        let evaluated = eval(&mut loader, &mut cache, Some(&path), tree.clone(), &scope);
        let executed = exec(&evaluated.output.template, state.clone());
        let layouted = layout(&mut loader, &mut cache, &executed.output);

        // Bench!
        bench!("parse": parse(&src));
        bench!("eval": eval(&mut loader, &mut cache, Some(&path), tree.clone(), &scope));
        bench!("exec": exec(&evaluated.output.template, state.clone()));
        bench!("layout": layout(&mut loader, &mut cache, &executed.output));
        bench!("typeset": typeset(&mut loader, &mut cache, Some(&path), &src, &scope, state.clone()));
        bench!("pdf": pdf(&cache, &layouted));
    }
}

criterion_group!(benches, benchmarks);
criterion_main!(benches);
