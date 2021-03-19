use std::mem;
use std::rc::Rc;

use fontdock::FontStyle;

use super::{Exec, State};
use crate::diag::{Diag, DiagSet, Pass};
use crate::env::Env;
use crate::eval::TemplateValue;
use crate::geom::{Dir, Gen, Linear, Sides, Size};
use crate::layout::{
    Node, PadNode, PageRun, ParNode, SpacingNode, StackNode, TextNode, Tree,
};
use crate::parse::is_newline;
use crate::syntax::{Span, Spanned};

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
    stack: StackNode,
    /// The content of the active paragraph.
    par: ParNode,
}

impl<'a> ExecContext<'a> {
    /// Create a new execution context with a base state.
    pub fn new(env: &'a mut Env, state: State) -> Self {
        Self {
            env,
            diags: DiagSet::new(),
            tree: Tree { runs: vec![] },
            page: Some(PageInfo::new(&state, true)),
            stack: StackNode::new(&state),
            par: ParNode::new(&state),
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
    /// Spacing nodes will be handled according to their
    /// [`softness`](SpacingNode::softness).
    pub fn push(&mut self, node: impl Into<Node>) {
        push(&mut self.par.children, node.into());
    }

    /// Push a word space into the active paragraph.
    pub fn push_space(&mut self) {
        let em = self.state.font.font_size();
        self.push(SpacingNode {
            amount: self.state.par.word_spacing.resolve(em),
            softness: 1,
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
        let em = self.state.font.font_size();
        self.push_into_stack(SpacingNode {
            amount: self.state.par.leading.resolve(em),
            softness: 2,
        });
    }

    /// Apply a forced paragraph break.
    pub fn push_parbreak(&mut self) {
        let em = self.state.font.font_size();
        self.push_into_stack(SpacingNode {
            amount: self.state.par.spacing.resolve(em),
            softness: 1,
        });
    }

    /// Push a node directly into the stack above the paragraph. This finishes
    /// the active paragraph and starts a new one.
    pub fn push_into_stack(&mut self, node: impl Into<Node>) {
        self.finish_par();
        push(&mut self.stack.children, node.into());
    }

    /// Execute a template and return the result as a stack node.
    pub fn exec(&mut self, template: &TemplateValue) -> StackNode {
        let page = self.page.take();
        let stack = mem::replace(&mut self.stack, StackNode::new(&self.state));
        let par = mem::replace(&mut self.par, ParNode::new(&self.state));

        template.exec(self);
        let result = self.finish_stack();

        self.page = page;
        self.stack = stack;
        self.par = par;

        result
    }

    /// Construct a text node from the given string based on the active text
    /// state.
    pub fn make_text_node(&self, text: String) -> TextNode {
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

        TextNode {
            text,
            dir: self.state.dirs.cross,
            aligns: self.state.aligns,
            families: Rc::clone(&self.state.font.families),
            variant,
            font_size: self.state.font.font_size(),
            top_edge: self.state.font.top_edge,
            bottom_edge: self.state.font.bottom_edge,
        }
    }

    /// Finish the active paragraph.
    fn finish_par(&mut self) {
        let mut par = mem::replace(&mut self.par, ParNode::new(&self.state));
        trim(&mut par.children);

        if !par.children.is_empty() {
            self.stack.children.push(par.into());
        }
    }

    /// Finish the active stack.
    fn finish_stack(&mut self) -> StackNode {
        self.finish_par();

        let mut stack = mem::replace(&mut self.stack, StackNode::new(&self.state));
        trim(&mut stack.children);

        stack
    }

    /// Finish the active page.
    pub fn finish_page(&mut self, keep: bool, hard: bool, source: Span) {
        if let Some(info) = &mut self.page {
            let info = mem::replace(info, PageInfo::new(&self.state, hard));
            let stack = self.finish_stack();

            if !stack.children.is_empty() || (keep && info.hard) {
                self.tree.runs.push(PageRun {
                    size: info.size,
                    child: PadNode {
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
        self.finish_page(true, false, Span::default());
        Pass::new(self.tree, self.diags)
    }
}

/// Push a node into a list, taking care of spacing softness.
fn push(nodes: &mut Vec<Node>, node: Node) {
    if let Node::Spacing(spacing) = node {
        if nodes.is_empty() && spacing.softness > 0 {
            return;
        }

        if let Some(&Node::Spacing(other)) = nodes.last() {
            if spacing.softness > 0 && spacing.softness >= other.softness {
                return;
            }

            if spacing.softness < other.softness {
                nodes.pop();
            }
        }
    }

    nodes.push(node);
}

/// Remove trailing soft spacing from a node list.
fn trim(nodes: &mut Vec<Node>) {
    if let Some(&Node::Spacing(spacing)) = nodes.last() {
        if spacing.softness > 0 {
            nodes.pop();
        }
    }
}

#[derive(Debug)]
struct PageInfo {
    size: Size,
    padding: Sides<Linear>,
    hard: bool,
}

impl PageInfo {
    fn new(state: &State, hard: bool) -> Self {
        Self {
            size: state.page.size,
            padding: state.page.margins(),
            hard,
        }
    }
}

impl StackNode {
    fn new(state: &State) -> Self {
        Self {
            dirs: state.dirs,
            aligns: state.aligns,
            children: vec![],
        }
    }
}

impl ParNode {
    fn new(state: &State) -> Self {
        let em = state.font.font_size();
        Self {
            dirs: state.dirs,
            aligns: state.aligns,
            line_spacing: state.par.leading.resolve(em),
            children: vec![],
        }
    }
}
