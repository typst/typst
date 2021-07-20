use std::mem;
use std::rc::Rc;

use super::{Exec, ExecWithMap, FontFamily, State};
use crate::diag::{Diag, DiagSet, Pass};
use crate::eco::EcoString;
use crate::eval::{ExprMap, Template};
use crate::geom::{Align, Dir, Gen, GenAxis, Length, Linear, Sides, Size};
use crate::layout::{
    LayoutNode, LayoutTree, PadNode, PageRun, ParChild, ParNode, StackChild, StackNode,
};
use crate::syntax::{Span, SyntaxTree};
use crate::Context;

/// The context for execution.
pub struct ExecContext {
    /// The active execution state.
    pub state: State,
    /// Execution diagnostics.
    pub diags: DiagSet,
    /// The tree of finished page runs.
    tree: LayoutTree,
    /// When we are building the top-level stack, this contains metrics of the
    /// page. While building a group stack through `exec_group`, this is `None`.
    page: Option<PageBuilder>,
    /// The currently built stack of paragraphs.
    stack: StackBuilder,
}

impl ExecContext {
    /// Create a new execution context with a base state.
    pub fn new(ctx: &mut Context) -> Self {
        Self {
            state: ctx.state.clone(),
            diags: DiagSet::new(),
            tree: LayoutTree { runs: vec![] },
            page: Some(PageBuilder::new(&ctx.state, true)),
            stack: StackBuilder::new(&ctx.state),
        }
    }

    /// Add a diagnostic.
    pub fn diag(&mut self, diag: Diag) {
        self.diags.insert(diag);
    }

    /// Set the font to monospace.
    pub fn set_monospace(&mut self) {
        self.state
            .font_mut()
            .families_mut()
            .list
            .insert(0, FontFamily::Monospace);
    }

    /// Execute a template and return the result as a stack node.
    pub fn exec_template_stack(&mut self, template: &Template) -> StackNode {
        self.exec_stack(|ctx| template.exec(ctx))
    }

    /// Execute a syntax tree with a map and return the result as a stack node.
    pub fn exec_tree_stack(&mut self, tree: &SyntaxTree, map: &ExprMap) -> StackNode {
        self.exec_stack(|ctx| tree.exec_with_map(ctx, map))
    }

    /// Execute something and return the result as a stack node.
    pub fn exec_stack(&mut self, f: impl FnOnce(&mut Self)) -> StackNode {
        let snapshot = self.state.clone();
        let page = self.page.take();
        let stack = mem::replace(&mut self.stack, StackBuilder::new(&self.state));

        f(self);

        self.state = snapshot;
        self.page = page;
        mem::replace(&mut self.stack, stack).build()
    }

    /// Push text into the active paragraph.
    ///
    /// The text is split into lines at newlines.
    pub fn push_text(&mut self, text: impl Into<EcoString>) {
        self.stack.par.push(self.make_text_node(text));
    }

    /// Push a word space into the active paragraph.
    pub fn push_word_space(&mut self) {
        self.stack.par.push_soft(self.make_text_node(" "));
    }

    /// Push any node into the active paragraph.
    pub fn push_into_par(&mut self, node: impl Into<LayoutNode>) {
        let align = self.state.aligns.cross;
        self.stack.par.push(ParChild::Any(node.into(), align));
    }

    /// Push any node into the active stack.
    pub fn push_into_stack(&mut self, node: impl Into<LayoutNode>) {
        self.parbreak();
        let aligns = self.state.aligns;
        self.stack.push(StackChild::Any(node.into(), aligns));
        self.parbreak();
    }

    /// Push spacing into the active paragraph or stack depending on the `axis`.
    pub fn push_spacing(&mut self, axis: GenAxis, amount: Length) {
        match axis {
            GenAxis::Main => {
                self.stack.finish_par(&self.state);
                self.stack.push_hard(StackChild::Spacing(amount));
            }
            GenAxis::Cross => {
                self.stack.par.push_hard(ParChild::Spacing(amount));
            }
        }
    }

    /// Apply a forced line break.
    pub fn linebreak(&mut self) {
        self.stack.par.push_hard(self.make_text_node("\n"));
    }

    /// Apply a forced paragraph break.
    pub fn parbreak(&mut self) {
        let amount = self.state.par.spacing.resolve(self.state.font.size);
        self.stack.finish_par(&self.state);
        self.stack.push_soft(StackChild::Spacing(amount));
    }

    /// Apply a forced page break.
    pub fn pagebreak(&mut self, keep: bool, hard: bool, span: Span) {
        if let Some(builder) = &mut self.page {
            let page = mem::replace(builder, PageBuilder::new(&self.state, hard));
            let stack = mem::replace(&mut self.stack, StackBuilder::new(&self.state));
            self.tree.runs.extend(page.build(stack.build(), keep));
        } else {
            self.diag(error!(span, "cannot modify page from here"));
        }
    }

    /// Finish execution and return the created layout tree.
    pub fn finish(mut self) -> Pass<LayoutTree> {
        assert!(self.page.is_some());
        self.pagebreak(true, false, Span::default());
        Pass::new(self.tree, self.diags)
    }

    fn make_text_node(&self, text: impl Into<EcoString>) -> ParChild {
        ParChild::Text(
            text.into(),
            self.state.aligns.cross,
            Rc::clone(&self.state.font),
        )
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
    children: Vec<StackChild>,
    last: Last<StackChild>,
    par: ParBuilder,
}

impl StackBuilder {
    fn new(state: &State) -> Self {
        Self {
            dirs: Gen::new(state.lang.dir, Dir::TTB),
            children: vec![],
            last: Last::None,
            par: ParBuilder::new(state),
        }
    }

    fn push(&mut self, child: StackChild) {
        self.children.extend(self.last.any());
        self.children.push(child);
    }

    fn push_soft(&mut self, child: StackChild) {
        self.last.soft(child);
    }

    fn push_hard(&mut self, child: StackChild) {
        self.last.hard();
        self.children.push(child);
    }

    fn finish_par(&mut self, state: &State) {
        let par = mem::replace(&mut self.par, ParBuilder::new(state));
        if let Some(par) = par.build() {
            self.push(par);
        }
    }

    fn build(self) -> StackNode {
        let Self { dirs, mut children, par, mut last } = self;
        if let Some(par) = par.build() {
            children.extend(last.any());
            children.push(par);
        }
        StackNode { dirs, aspect: None, children }
    }
}

struct ParBuilder {
    aligns: Gen<Align>,
    dir: Dir,
    line_spacing: Length,
    children: Vec<ParChild>,
    last: Last<ParChild>,
}

impl ParBuilder {
    fn new(state: &State) -> Self {
        Self {
            aligns: state.aligns,
            dir: state.lang.dir,
            line_spacing: state.par.leading.resolve(state.font.size),
            children: vec![],
            last: Last::None,
        }
    }

    fn push(&mut self, child: ParChild) {
        if let Some(soft) = self.last.any() {
            self.push_inner(soft);
        }
        self.push_inner(child);
    }

    fn push_soft(&mut self, child: ParChild) {
        self.last.soft(child);
    }

    fn push_hard(&mut self, child: ParChild) {
        self.last.hard();
        self.push_inner(child);
    }

    fn push_inner(&mut self, child: ParChild) {
        if let ParChild::Text(curr_text, curr_props, curr_align) = &child {
            if let Some(ParChild::Text(prev_text, prev_props, prev_align)) =
                self.children.last_mut()
            {
                if prev_align == curr_align && prev_props == curr_props {
                    prev_text.push_str(&curr_text);
                    return;
                }
            }
        }

        self.children.push(child);
    }

    fn build(self) -> Option<StackChild> {
        let Self { aligns, dir, line_spacing, children, .. } = self;
        (!children.is_empty()).then(|| {
            let node = ParNode { dir, line_spacing, children };
            StackChild::Any(node.into(), aligns)
        })
    }
}

/// Finite state machine for spacing coalescing.
enum Last<N> {
    None,
    Any,
    Soft(N),
}

impl<N> Last<N> {
    fn any(&mut self) -> Option<N> {
        match mem::replace(self, Self::Any) {
            Self::Soft(soft) => Some(soft),
            _ => None,
        }
    }

    fn soft(&mut self, soft: N) {
        if let Self::Any = self {
            *self = Self::Soft(soft);
        }
    }

    fn hard(&mut self) {
        *self = Self::None;
    }
}
