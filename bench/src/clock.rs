use std::cell::RefCell;
use std::path::Path;
use std::rc::Rc;

use criterion::{criterion_group, criterion_main, Criterion};

use typst::diag::TypResult;
use typst::eval::{eval, Module, State};
use typst::export::pdf;
use typst::layout::{layout, Frame, LayoutTree};
use typst::loading::FsLoader;
use typst::parse::parse;
use typst::source::SourceId;
use typst::syntax::SyntaxTree;
use typst::Context;

const FONT_DIR: &str = "../fonts";
const TYP_DIR: &str = "../tests/typ";
const CASES: &[&str] = &["coma.typ", "text/basic.typ"];

fn benchmarks(c: &mut Criterion) {
    let loader = FsLoader::new().with_path(FONT_DIR).wrap();
    let ctx = Rc::new(RefCell::new(Context::new(loader)));

    for case in CASES {
        let path = Path::new(TYP_DIR).join(case);
        let name = path.file_stem().unwrap().to_string_lossy();
        let id = ctx.borrow_mut().sources.load(&path).unwrap();
        let case = Case::new(ctx.clone(), id);

        macro_rules! bench {
            ($step:literal, setup: |$ctx:ident| $setup:expr, code: $code:expr $(,)?) => {
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
        bench!("build", case.build());

        #[cfg(not(feature = "layout-cache"))]
        {
            bench!("layout", case.layout());
            bench!("typeset", case.typeset());
        }

        #[cfg(feature = "layout-cache")]
        {
            bench!("layout", setup: |ctx| ctx.layouts.clear(), code: case.layout());
            bench!("typeset", setup: |ctx| ctx.layouts.clear(), code: case.typeset());
            bench!("layout-cached", case.layout());
            bench!("typeset-cached", case.typeset());
        }

        bench!("pdf", case.pdf());
    }
}

/// A test case with prepared intermediate results.
struct Case {
    ctx: Rc<RefCell<Context>>,
    state: State,
    id: SourceId,
    ast: SyntaxTree,
    module: Module,
    tree: LayoutTree,
    frames: Vec<Rc<Frame>>,
}

impl Case {
    fn new(ctx: Rc<RefCell<Context>>, id: SourceId) -> Self {
        let mut borrowed = ctx.borrow_mut();
        let state = State::default();
        let source = borrowed.sources.get(id);
        let ast = parse(source).unwrap();
        let module = eval(&mut borrowed, id, &ast).unwrap();
        let tree = module.template.to_tree(&state);
        let frames = layout(&mut borrowed, &tree);
        drop(borrowed);
        Self {
            ctx,
            state,
            id,
            ast,
            module,
            tree,
            frames,
        }
    }

    fn parse(&self) -> SyntaxTree {
        parse(self.ctx.borrow().sources.get(self.id)).unwrap()
    }

    fn eval(&self) -> TypResult<Module> {
        eval(&mut self.ctx.borrow_mut(), self.id, &self.ast)
    }

    fn build(&self) -> LayoutTree {
        self.module.template.to_tree(&self.state)
    }

    fn layout(&self) -> Vec<Rc<Frame>> {
        layout(&mut self.ctx.borrow_mut(), &self.tree)
    }

    fn typeset(&self) -> TypResult<Vec<Rc<Frame>>> {
        self.ctx.borrow_mut().typeset(self.id)
    }

    fn pdf(&self) -> Vec<u8> {
        pdf(&self.ctx.borrow(), &self.frames)
    }
}

criterion_group!(benches, benchmarks);
criterion_main!(benches);
