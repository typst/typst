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
    let loader = FsLoader::new().with_path(FONT_DIR).wrap();
    let ctx = Rc::new(RefCell::new(Context::new(loader.clone())));

    for case in CASES {
        let path = Path::new(TYP_DIR).join(case);
        let name = path.file_stem().unwrap().to_string_lossy();
        let file = loader.resolve(&path).unwrap();
        let src = std::fs::read_to_string(&path).unwrap();
        let case = Case::new(file, src, ctx.clone());

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
    file: FileId,
    src: String,
    ast: Rc<SyntaxTree>,
    module: Module,
    tree: LayoutTree,
    frames: Vec<Rc<Frame>>,
}

impl Case {
    fn new(file: FileId, src: String, ctx: Rc<RefCell<Context>>) -> Self {
        let mut borrowed = ctx.borrow_mut();
        let ast = Rc::new(parse(file, &src).unwrap());
        let module = eval(&mut borrowed, file, Rc::clone(&ast)).unwrap();
        let tree = exec(&mut borrowed, &module.template);
        let frames = layout(&mut borrowed, &tree);
        drop(borrowed);
        Self {
            ctx,
            file,
            src,
            ast,
            module,
            tree,
            frames,
        }
    }

    fn parse(&self) -> SyntaxTree {
        parse(self.file, &self.src).unwrap()
    }

    fn eval(&self) -> Module {
        eval(&mut self.ctx.borrow_mut(), self.file, Rc::clone(&self.ast)).unwrap()
    }

    fn exec(&self) -> LayoutTree {
        exec(&mut self.ctx.borrow_mut(), &self.module.template)
    }

    fn layout(&self) -> Vec<Rc<Frame>> {
        layout(&mut self.ctx.borrow_mut(), &self.tree)
    }

    fn typeset(&self) -> Vec<Rc<Frame>> {
        self.ctx.borrow_mut().typeset(self.file, &self.src).unwrap()
    }

    fn pdf(&self) -> Vec<u8> {
        pdf(&self.ctx.borrow(), &self.frames)
    }
}

criterion_group!(benches, benchmarks);
criterion_main!(benches);
