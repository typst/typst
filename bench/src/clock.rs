use std::cell::RefCell;
use std::path::Path;
use std::rc::Rc;

use criterion::{criterion_group, criterion_main, Criterion};

use typst::eval::{eval, Module};
use typst::exec::exec;
use typst::export::pdf;
use typst::layout::{layout, Frame, LayoutTree};
use typst::loading::{FileId, FsLoader};
use typst::parse::parse;
use typst::syntax::SyntaxTree;
use typst::Context;

const FONT_DIR: &str = "../fonts";
const TYP_DIR: &str = "../tests/typ";
const CASES: &[&str] = &["coma.typ", "text/basic.typ"];

fn benchmarks(c: &mut Criterion) {
    let loader = {
        let mut loader = FsLoader::new();
        loader.search_path(FONT_DIR);
        Rc::new(loader)
    };

    let ctx = Rc::new(RefCell::new(Context::new(loader.clone())));

    for case in CASES {
        let path = Path::new(TYP_DIR).join(case);
        let name = path.file_stem().unwrap().to_string_lossy();
        let src_id = loader.resolve_path(&path).unwrap();
        let src = std::fs::read_to_string(&path).unwrap();
        let case = Case::new(src_id, src, ctx.clone());

        macro_rules! bench {
            ($step:literal, setup = |$ctx:ident| $setup:expr, code = $code:expr $(,)?) => {
                c.bench_function(&format!("{}-{}", $step, name), |b| {
                    b.iter_batched(
                        || {
                            let mut $ctx = ctx.borrow_mut();
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
                setup = |ctx| ctx.layouts.clear(),
                code = case.layout(),
            );
            bench!(
                "typeset",
                setup = |ctx| ctx.layouts.clear(),
                code = case.typeset(),
            );
            bench!("layout-cached", case.layout());
            bench!("typeset-cached", case.typeset());
        }

        bench!("pdf", case.pdf());
    }
}

/// A test case with prepared intermediate results.
struct Case {
    ctx: Rc<RefCell<Context>>,
    src_id: FileId,
    src: String,
    ast: Rc<SyntaxTree>,
    module: Module,
    tree: LayoutTree,
    frames: Vec<Rc<Frame>>,
}

impl Case {
    fn new(src_id: FileId, src: String, ctx: Rc<RefCell<Context>>) -> Self {
        let mut borrowed = ctx.borrow_mut();
        let ast = Rc::new(parse(&src).output);
        let module = eval(&mut borrowed, src_id, Rc::clone(&ast)).output;
        let tree = exec(&mut borrowed, &module.template).output;
        let frames = layout(&mut borrowed, &tree);
        drop(borrowed);
        Self {
            ctx,
            src_id,
            src,
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
        eval(&mut borrowed, self.src_id, Rc::clone(&self.ast)).output
    }

    fn exec(&self) -> LayoutTree {
        exec(&mut self.ctx.borrow_mut(), &self.module.template).output
    }

    fn layout(&self) -> Vec<Rc<Frame>> {
        layout(&mut self.ctx.borrow_mut(), &self.tree)
    }

    fn typeset(&self) -> Vec<Rc<Frame>> {
        self.ctx.borrow_mut().typeset(self.src_id, &self.src).output
    }

    fn pdf(&self) -> Vec<u8> {
        pdf(&self.ctx.borrow(), &self.frames)
    }
}

criterion_group!(benches, benchmarks);
criterion_main!(benches);
