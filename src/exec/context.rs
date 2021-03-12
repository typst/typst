use std::mem;
use std::rc::Rc;

use fontdock::FontStyle;

use super::*;
use crate::diag::{Diag, DiagSet};
use crate::geom::{Dir, Gen, Linear, Sides, Size};
use crate::layout::{
    Node, NodePad, NodePages, NodePar, NodeSpacing, NodeStack, NodeText, Tree,
};
use crate::parse::is_newline;

/// The context for execution.
#[derive(Debug)]
pub struct ExecContext<'a> {
    /// The environment from which resources are gathered.
    pub env: &'a mut Env,
    /// The active execution state.
    pub state: State,
    /// Execution diagnostics.
    pub diags: DiagSet,
    /// The tree of finished page runs.
    tree: Tree,
    /// Metrics of the active page.
    page: Option<PageInfo>,
    /// The content of the active stack. This may be the top-level stack for the
    /// page or a lower one created by [`exec`](Self::exec).
    stack: NodeStack,
    /// The content of the active paragraph.
    par: NodePar,
}

impl<'a> ExecContext<'a> {
    /// Create a new execution context with a base state.
    pub fn new(env: &'a mut Env, state: State) -> Self {
        Self {
            env,
            diags: DiagSet::new(),
            tree: Tree { runs: vec![] },
            page: Some(PageInfo::new(&state, Softness::Hard)),
            stack: NodeStack::new(&state),
            par: NodePar::new(&state),
            state,
        }
    }

    /// Add a diagnostic.
    pub fn diag(&mut self, diag: Diag) {
        self.diags.insert(diag);
    }

    /// Set the directions.
    ///
    /// Produces an error if the axes aligned.
    pub fn set_dirs(&mut self, new: Gen<Option<Spanned<Dir>>>) {
        let dirs = Gen::new(
            new.main.map(|s| s.v).unwrap_or(self.state.dirs.main),
            new.cross.map(|s| s.v).unwrap_or(self.state.dirs.cross),
        );

        if dirs.main.axis() != dirs.cross.axis() {
            self.state.dirs = dirs;
        } else {
            for dir in new.main.iter().chain(new.cross.iter()) {
                self.diag(error!(dir.span, "aligned axis"));
            }
        }
    }

    /// Set the font to monospace.
    pub fn set_monospace(&mut self) {
        let families = self.state.font.families_mut();
        families.list.insert(0, "monospace".to_string());
        families.flatten();
    }

    /// Push a layout node into the active paragraph.
    ///
    /// Spacing nodes will be handled according to their [`Softness`].
    pub fn push(&mut self, node: impl Into<Node>) {
        push(&mut self.par.children, node.into());
    }

    /// Push a word space into the active paragraph.
    pub fn push_space(&mut self) {
        let em = self.state.font.font_size();
        self.push(NodeSpacing {
            amount: self.state.par.word_spacing.resolve(em),
            softness: Softness::Soft,
        });
    }

    /// Push text into the active paragraph.
    ///
    /// The text is split into lines at newlines.
    pub fn push_text(&mut self, text: &str) {
        let mut newline = false;
        for line in text.split_terminator(is_newline) {
            if newline {
                self.push_linebreak();
            }

            let node = self.make_text_node(line.into());
            self.push(node);
            newline = true;
        }
    }

    /// Apply a forced line break.
    pub fn push_linebreak(&mut self) {
        self.finish_par();
    }

    /// Apply a forced paragraph break.
    pub fn push_parbreak(&mut self) {
        let em = self.state.font.font_size();
        self.push_into_stack(NodeSpacing {
            amount: self.state.par.par_spacing.resolve(em),
            softness: Softness::Soft,
        });
    }

    /// Push a node directly into the stack above the paragraph. This finishes
    /// the active paragraph and starts a new one.
    pub fn push_into_stack(&mut self, node: impl Into<Node>) {
        self.finish_par();
        push(&mut self.stack.children, node.into());
    }

    /// Execute a template and return the result as a stack node.
    pub fn exec(&mut self, template: &ValueTemplate) -> NodeStack {
        let page = self.page.take();
        let stack = mem::replace(&mut self.stack, NodeStack::new(&self.state));
        let par = mem::replace(&mut self.par, NodePar::new(&self.state));

        template.exec(self);
        let result = self.finish_stack();

        self.page = page;
        self.stack = stack;
        self.par = par;

        result
    }

    /// Construct a text node from the given string based on the active text
    /// state.
    pub fn make_text_node(&self, text: String) -> NodeText {
        let mut variant = self.state.font.variant;

        if self.state.font.strong {
            variant.weight = variant.weight.thicken(300);
        }

        if self.state.font.emph {
            variant.style = match variant.style {
                FontStyle::Normal => FontStyle::Italic,
                FontStyle::Italic => FontStyle::Normal,
                FontStyle::Oblique => FontStyle::Normal,
            }
        }

        NodeText {
            text,
            aligns: self.state.aligns,
            dir: self.state.dirs.cross,
            font_size: self.state.font.font_size(),
            families: Rc::clone(&self.state.font.families),
            variant,
        }
    }

    /// Finish the active paragraph.
    fn finish_par(&mut self) {
        let mut par = mem::replace(&mut self.par, NodePar::new(&self.state));
        trim(&mut par.children);

        if !par.children.is_empty() {
            self.stack.children.push(par.into());
        }
    }

    /// Finish the active stack.
    fn finish_stack(&mut self) -> NodeStack {
        self.finish_par();

        let mut stack = mem::replace(&mut self.stack, NodeStack::new(&self.state));
        trim(&mut stack.children);

        stack
    }

    /// Finish the active page.
    pub fn finish_page(&mut self, keep: bool, new_softness: Softness, source: Span) {
        if let Some(info) = &mut self.page {
            let info = mem::replace(info, PageInfo::new(&self.state, new_softness));
            let stack = self.finish_stack();

            if !stack.children.is_empty() || (keep && info.softness == Softness::Hard) {
                self.tree.runs.push(NodePages {
                    size: info.size,
                    child: NodePad {
                        padding: info.padding,
                        child: stack.into(),
                    }
                    .into(),
                });
            }
        } else {
            self.diag(error!(source, "cannot modify page from here"));
        }
    }

    /// Finish execution and return the created layout tree.
    pub fn finish(mut self) -> Pass<Tree> {
        assert!(self.page.is_some());
        self.finish_page(true, Softness::Soft, Span::default());
        Pass::new(self.tree, self.diags)
    }
}

/// Push a node into a list, taking care of spacing softness.
fn push(nodes: &mut Vec<Node>, node: Node) {
    if let Node::Spacing(spacing) = node {
        if spacing.softness == Softness::Soft && nodes.is_empty() {
            return;
        }

        if let Some(&Node::Spacing(other)) = nodes.last() {
            if spacing.softness > other.softness {
                nodes.pop();
            } else if spacing.softness == Softness::Soft {
                return;
            }
        }
    }

    nodes.push(node);
}

/// Remove trailing soft spacing from a node list.
fn trim(nodes: &mut Vec<Node>) {
    if let Some(&Node::Spacing(spacing)) = nodes.last() {
        if spacing.softness == Softness::Soft {
            nodes.pop();
        }
    }
}

#[derive(Debug)]
struct PageInfo {
    size: Size,
    padding: Sides<Linear>,
    softness: Softness,
}

impl PageInfo {
    fn new(state: &State, softness: Softness) -> Self {
        Self {
            size: state.page.size,
            padding: state.page.margins(),
            softness,
        }
    }
}

impl NodeStack {
    fn new(state: &State) -> Self {
        Self {
            dirs: state.dirs,
            aligns: state.aligns,
            children: vec![],
        }
    }
}

impl NodePar {
    fn new(state: &State) -> Self {
        let em = state.font.font_size();
        Self {
            dirs: state.dirs,
            aligns: state.aligns,
            line_spacing: state.par.line_spacing.resolve(em),
            children: vec![],
        }
    }
}
