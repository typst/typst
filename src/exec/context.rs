use std::mem;

use super::{Exec, FontFamily, State};
use crate::diag::{Diag, DiagSet, Pass};
use crate::env::Env;
use crate::eval::TemplateValue;
use crate::geom::{Align, Dir, Gen, GenAxis, Length, Linear, Sides, Size};
use crate::layout::{
    AnyNode, PadNode, PageRun, ParChild, ParNode, StackChild, StackNode, TextNode, Tree,
};
use crate::parse::{is_newline, Scanner};
use crate::syntax::Span;

/// The context for execution.
pub struct ExecContext<'a> {
    /// The environment from which resources are gathered.
    pub env: &'a mut Env,
    /// The active execution state.
    pub state: State,
    /// Execution diagnostics.
    pub diags: DiagSet,
    /// The tree of finished page runs.
    tree: Tree,
    /// When we are building the top-level stack, this contains metrics of the
    /// page. While building a group stack through `exec_group`, this is `None`.
    page: Option<PageBuilder>,
    /// The currently built stack of paragraphs.
    stack: StackBuilder,
}

impl<'a> ExecContext<'a> {
    /// Create a new execution context with a base state.
    pub fn new(env: &'a mut Env, state: State) -> Self {
        Self {
            env,
            diags: DiagSet::new(),
            tree: Tree { runs: vec![] },
            page: Some(PageBuilder::new(&state, true)),
            stack: StackBuilder::new(&state),
            state,
        }
    }

    /// Add a diagnostic.
    pub fn diag(&mut self, diag: Diag) {
        self.diags.insert(diag);
    }

    /// Set the font to monospace.
    pub fn set_monospace(&mut self) {
        let families = self.state.font.families_mut();
        families.list.insert(0, FontFamily::Monospace);
    }

    /// Execute a template and return the result as a stack node.
    pub fn exec_group(&mut self, template: &TemplateValue) -> StackNode {
        let snapshot = self.state.clone();
        let page = self.page.take();
        let stack = mem::replace(&mut self.stack, StackBuilder::new(&self.state));

        template.exec(self);

        self.state = snapshot;
        self.page = page;
        mem::replace(&mut self.stack, stack).build()
    }

    /// Push text into the active paragraph.
    ///
    /// The text is split into lines at newlines.
    pub fn push_text(&mut self, text: &str) {
        let mut scanner = Scanner::new(text);
        let mut line = String::new();
        let push = |this: &mut Self, text| {
            let props = this.state.font.resolve_props();
            let node = TextNode { text, props };
            let align = this.state.aligns.cross;
            this.stack.par.folder.push(ParChild::Text(node, align))
        };

        while let Some(c) = scanner.eat_merging_crlf() {
            if is_newline(c) {
                push(self, mem::take(&mut line));
                self.push_linebreak();
            } else {
                line.push(c);
            }
        }

        push(self, line);
    }

    /// Push a word space.
    pub fn push_word_space(&mut self) {
        let em = self.state.font.resolve_size();
        let amount = self.state.par.word_spacing.resolve(em);
        self.push_spacing(GenAxis::Cross, amount, 1);
    }

    /// Apply a forced line break.
    pub fn push_linebreak(&mut self) {
        let em = self.state.font.resolve_size();
        let amount = self.state.par.leading.resolve(em);
        self.push_spacing(GenAxis::Main, amount, 2);
    }

    /// Apply a forced paragraph break.
    pub fn push_parbreak(&mut self) {
        let em = self.state.font.resolve_size();
        let amount = self.state.par.spacing.resolve(em);
        self.push_spacing(GenAxis::Main, amount, 1);
    }

    /// Push spacing into paragraph or stack depending on `axis`.
    ///
    /// The `softness` configures how the spacing interacts with surrounding
    /// spacing.
    pub fn push_spacing(&mut self, axis: GenAxis, amount: Length, softness: u8) {
        match axis {
            GenAxis::Main => {
                let spacing = StackChild::Spacing(amount);
                self.stack.finish_par(&self.state);
                self.stack.folder.push_soft(spacing, softness);
            }
            GenAxis::Cross => {
                let spacing = ParChild::Spacing(amount);
                self.stack.par.folder.push_soft(spacing, softness);
            }
        }
    }

    /// Push any node into the active paragraph.
    pub fn push_into_par(&mut self, node: impl Into<AnyNode>) {
        let align = self.state.aligns.cross;
        self.stack.par.folder.push(ParChild::Any(node.into(), align));
    }

    /// Push any node directly into the stack of paragraphs.
    ///
    /// This finishes the active paragraph and starts a new one.
    pub fn push_into_stack(&mut self, node: impl Into<AnyNode>) {
        let aligns = self.state.aligns;
        self.stack.finish_par(&self.state);
        self.stack.folder.push(StackChild::Any(node.into(), aligns));
    }

    /// Finish the active page.
    pub fn finish_page(&mut self, keep: bool, hard: bool, source: Span) {
        if let Some(builder) = &mut self.page {
            let page = mem::replace(builder, PageBuilder::new(&self.state, hard));
            let stack = mem::replace(&mut self.stack, StackBuilder::new(&self.state));
            self.tree.runs.extend(page.build(stack.build(), keep));
        } else {
            self.diag(error!(source, "cannot modify page from here"));
        }
    }

    /// Finish execution and return the created layout tree.
    pub fn finish(mut self) -> Pass<Tree> {
        assert!(self.page.is_some());
        self.finish_page(true, false, Span::default());
        Pass::new(self.tree, self.diags)
    }
}

struct PageBuilder {
    size: Size,
    padding: Sides<Linear>,
    hard: bool,
}

impl PageBuilder {
    fn new(state: &State, hard: bool) -> Self {
        Self {
            size: state.page.size,
            padding: state.page.margins(),
            hard,
        }
    }

    fn build(self, child: StackNode, keep: bool) -> Option<PageRun> {
        let Self { size, padding, hard } = self;
        (!child.children.is_empty() || (keep && hard)).then(|| PageRun {
            size,
            child: PadNode { padding, child: child.into() }.into(),
        })
    }
}

struct StackBuilder {
    dirs: Gen<Dir>,
    folder: SoftFolder<StackChild>,
    par: ParBuilder,
}

impl StackBuilder {
    fn new(state: &State) -> Self {
        Self {
            dirs: Gen::new(Dir::TTB, state.lang.dir),
            folder: SoftFolder::new(),
            par: ParBuilder::new(state),
        }
    }

    fn finish_par(&mut self, state: &State) {
        let par = mem::replace(&mut self.par, ParBuilder::new(state));
        self.folder.extend(par.build());
    }

    fn build(self) -> StackNode {
        let Self { dirs, mut folder, par } = self;
        folder.extend(par.build());
        StackNode { dirs, children: folder.finish() }
    }
}

struct ParBuilder {
    aligns: Gen<Align>,
    dir: Dir,
    line_spacing: Length,
    folder: SoftFolder<ParChild>,
}

impl ParBuilder {
    fn new(state: &State) -> Self {
        let em = state.font.resolve_size();
        Self {
            aligns: state.aligns,
            dir: state.lang.dir,
            line_spacing: state.par.leading.resolve(em),
            folder: SoftFolder::new(),
        }
    }

    fn build(self) -> Option<StackChild> {
        let Self { aligns, dir, line_spacing, folder } = self;
        let children = folder.finish();
        (!children.is_empty()).then(|| {
            let node = ParNode { dir, line_spacing, children };
            StackChild::Any(node.into(), aligns)
        })
    }
}

/// This is used to remove leading and trailing word/line/paragraph spacing
/// as well as collapse sequences of spacings into just one.
struct SoftFolder<N> {
    nodes: Vec<N>,
    last: Last<N>,
}

enum Last<N> {
    None,
    Hard,
    Soft(N, u8),
}

impl<N> SoftFolder<N> {
    fn new() -> Self {
        Self { nodes: vec![], last: Last::Hard }
    }

    fn push(&mut self, node: N) {
        let last = mem::replace(&mut self.last, Last::None);
        if let Last::Soft(soft, _) = last {
            self.nodes.push(soft);
        }
        self.nodes.push(node);
    }

    fn push_soft(&mut self, node: N, softness: u8) {
        if softness == 0 {
            self.last = Last::Hard;
            self.nodes.push(node);
        } else {
            match self.last {
                Last::Hard => {}
                Last::Soft(_, other) if softness >= other => {}
                _ => self.last = Last::Soft(node, softness),
            }
        }
    }

    fn finish(self) -> Vec<N> {
        self.nodes
    }
}

impl<N> Extend<N> for SoftFolder<N> {
    fn extend<T: IntoIterator<Item = N>>(&mut self, iter: T) {
        for elem in iter {
            self.push(elem);
        }
    }
}
