use std::cell::RefCell;
use std::path::Path;
use std::rc::Rc;

use criterion::{criterion_group, criterion_main, Criterion};

use typst::cache::Cache;
use typst::eval::{eval, Module, Scope};
use typst::exec::{exec, State};
use typst::export::pdf;
use typst::layout::{layout, Frame, LayoutTree};
use typst::loading::{FileId, FsLoader};
use typst::parse::parse;
use typst::syntax::SyntaxTree;
use typst::typeset;

const FONT_DIR: &str = "../fonts";
const TYP_DIR: &str = "../tests/typ";
const CASES: &[&str] = &["coma.typ", "text/basic.typ"];

fn benchmarks(c: &mut Criterion) {
    let ctx = Context::new();

    for case in CASES {
        let path = Path::new(TYP_DIR).join(case);
        let name = path.file_stem().unwrap().to_string_lossy();
        let src_id = ctx.borrow_mut().loader.resolve_path(&path).unwrap();
        let src = std::fs::read_to_string(&path).unwrap();
        let case = Case::new(src_id, src, ctx.clone());

        macro_rules! bench {
            ($step:literal, setup = |$cache:ident| $setup:expr, code = $code:expr $(,)?) => {
                c.bench_function(&format!("{}-{}", $step, name), |b| {
                    b.iter_batched(
                        || {
                            let mut borrowed = ctx.borrow_mut();
                            let $cache = &mut borrowed.cache;
                            $setup
                        },
                        |_| $code,
                        criterion::BatchSize::PerIteration,
                    )
                });
            };
            ($step:literal, $code:expr) => {
                c.bench_function(&format!("{}-{}", $step, name), |b| b.iter(|| $code));
            };
        }

        bench!("parse", case.parse());
        bench!("eval", case.eval());
        bench!("exec", case.exec());

        #[cfg(not(feature = "layout-cache"))]
        {
            bench!("layout", case.layout());
            bench!("typeset", case.typeset());
        }

        #[cfg(feature = "layout-cache")]
        {
            bench!(
                "layout",
                setup = |cache| cache.layout.clear(),
                code = case.layout(),
            );
            bench!(
                "typeset",
                setup = |cache| cache.layout.clear(),
                code = case.typeset(),
            );
            bench!("layout-cached", case.layout());
            bench!("typeset-cached", case.typeset());
        }

        bench!("pdf", case.pdf());
    }
}

/// The context required for benchmarking a case.
struct Context {
    loader: FsLoader,
    cache: Cache,
}

impl Context {
    fn new() -> Rc<RefCell<Self>> {
        let mut loader = FsLoader::new();
        loader.search_path(FONT_DIR);
        let cache = Cache::new(&loader);
        Rc::new(RefCell::new(Self { loader, cache }))
    }
}

/// A test case with prepared intermediate results.
struct Case {
    ctx: Rc<RefCell<Context>>,
    src_id: FileId,
    src: String,
    scope: Scope,
    state: State,
    ast: Rc<SyntaxTree>,
    module: Module,
    tree: LayoutTree,
    frames: Vec<Rc<Frame>>,
}

impl Case {
    fn new(src_id: FileId, src: String, ctx: Rc<RefCell<Context>>) -> Self {
        let mut borrowed = ctx.borrow_mut();
        let Context { loader, cache } = &mut *borrowed;
        let scope = typst::library::new();
        let state = typst::exec::State::default();
        let ast = Rc::new(parse(&src).output);
        let module = eval(loader, cache, src_id, Rc::clone(&ast), &scope).output;
        let tree = exec(&module.template, state.clone()).output;
        let frames = layout(loader, cache, &tree);
        drop(borrowed);
        Self {
            ctx,
            src_id,
            src,
            scope,
            state,
            ast,
            module,
            tree,
            frames,
        }
    }

    fn parse(&self) -> SyntaxTree {
        parse(&self.src).output
    }

    fn eval(&self) -> Module {
        let mut borrowed = self.ctx.borrow_mut();
        let Context { loader, cache } = &mut *borrowed;
        let ast = Rc::clone(&self.ast);
        eval(loader, cache, self.src_id, ast, &self.scope).output
    }

    fn exec(&self) -> LayoutTree {
        exec(&self.module.template, self.state.clone()).output
    }

    fn layout(&self) -> Vec<Rc<Frame>> {
        let mut borrowed = self.ctx.borrow_mut();
        let Context { loader, cache } = &mut *borrowed;
        layout(loader, cache, &self.tree)
    }

    fn typeset(&self) -> Vec<Rc<Frame>> {
        let mut borrowed = self.ctx.borrow_mut();
        let Context { loader, cache } = &mut *borrowed;
        let state = self.state.clone();
        typeset(loader, cache, self.src_id, &self.src, &self.scope, state).output
    }

    fn pdf(&self) -> Vec<u8> {
        let ctx = self.ctx.borrow();
        pdf(&ctx.cache, &self.frames)
    }
}

criterion_group!(benches, benchmarks);
criterion_main!(benches);
