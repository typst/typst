use std::cell::RefCell;
use std::path::Path;
use std::rc::Rc;

use criterion::{criterion_group, criterion_main, Criterion};

use typst::diag::TypResult;
use typst::eval::{eval, Module, State};
use typst::export::pdf;
#[cfg(feature = "layout-cache")]
use typst::layout::LayoutCache;
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
        let mut case = Case::new(ctx.clone(), id);

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

            case.layout();
            let original_cache = case.dump_layout_cache();
            let original_text = case.append_to_source("\n\nLorem ipsum dolor sit amet, consectetuer adipiscing elit. Aenean commodo ligula eget dolor. Aenean massa. Cum sociis natoque penatibus et magnis dis parturient montes, nascetur ridiculus mus.\n");
            // let original_text = case.replace_in_source("conquest", "mild success");

            bench!(
                "layout-modified",
                setup = |cache| {
                    cache.layout = original_cache.clone();
                },
                code = case.layout(),
            );

            bench!(
                "typeset-letters",
                setup = |cache| {
                    cache.layout = original_cache.clone();
                },
                code = {
                    for _ in 0 .. 20 {
                        case.typeset();
                        case.append_to_source("text ");
                        case.turnaround();
                    }
                },
            );

            bench!(
                "typeset-modified",
                setup = |cache| {
                    cache.layout = original_cache.clone();
                },
                code = case.typeset(),
            );

            case.src = original_text;
            case.refresh();
            case.clear_cache();
            case.layout();
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

    fn append_to_source(&mut self, extra: impl Into<String>) -> String {
        let orig = self.src.clone();

        self.src.push_str(&extra.into());
        self.refresh();

        orig
    }

    fn replace_in_source(
        &mut self,
        from: impl Into<String>,
        to: impl Into<String>,
    ) -> String {
        let orig = self.src.clone();

        self.src = self.src.replace(&from.into(), &to.into());
        self.refresh();

        orig
    }

    fn refresh(&mut self) {
        let mut borrowed = self.ctx.borrow_mut();

        self.ast = Rc::new(parse(&self.src).output);
        self.module = eval(borrowed.loader, borrowed.cache, None, Rc::clone(&self.ast), &self.scope).output;
        self.tree = exec(&self.module.template, self.state.clone()).output;
    }

    #[cfg(feature = "layout-cache")]
    fn dump_layout_cache(&self) -> LayoutCache {
        let borrowed = self.ctx.borrow();
        borrowed.cache.layout.clone()
    }

    #[cfg(feature = "layout-cache")]
    fn clear_cache(&mut self) {
        let mut borrowed = self.ctx.borrow_mut();
        borrowed.cache.layout.clear();
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

    fn turnaround(&self) {
        let mut borrowed = self.ctx.borrow_mut();

        // for item in borrowed.cache.layout.frames.iter().flat_map(|(_, item)| item.iter()) {
        //     println!("{:?}", item.properties().verdict());
        // }

        borrowed.cache.layout.turnaround();
    }
}

criterion_group!(benches, benchmarks);
criterion_main!(benches);
