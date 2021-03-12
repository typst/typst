use std::any::Any;
use std::rc::Rc;

use fontdock::FontStyle;

use super::*;
use crate::diag::{Diag, DiagSet};
use crate::geom::{Dir, Gen, LayoutAligns, LayoutDirs, Length, Linear, Sides, Size};
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
    /// The finished page runs.
    runs: Vec<NodePages>,
    /// The stack of logical groups (paragraphs and such).
    ///
    /// Each entry contains metadata about the group and nodes that are at the
    /// same level as the group, which will return to `inner` once the group is
    /// finished.
    groups: Vec<(Box<dyn Any>, Vec<Node>)>,
    /// The nodes in the current innermost group
    /// (whose metadata is in `groups.last()`).
    inner: Vec<Node>,
}

impl<'a> ExecContext<'a> {
    /// Create a new execution context with a base state.
    pub fn new(env: &'a mut Env, state: State) -> Self {
        Self {
            env,
            state,
            diags: DiagSet::new(),
            runs: vec![],
            groups: vec![],
            inner: vec![],
        }
    }

    /// Finish execution and return the created layout tree.
    pub fn finish(self) -> Pass<Tree> {
        assert!(self.groups.is_empty(), "unfinished group");
        Pass::new(Tree { runs: self.runs }, self.diags)
    }

    /// Add a diagnostic.
    pub fn diag(&mut self, diag: Diag) {
        self.diags.insert(diag);
    }

    /// Push a layout node to the active group.
    ///
    /// Spacing nodes will be handled according to their [`Softness`].
    pub fn push(&mut self, node: impl Into<Node>) {
        let node = node.into();

        if let Node::Spacing(this) = node {
            if this.softness == Softness::Soft && self.inner.is_empty() {
                return;
            }

            if let Some(&Node::Spacing(other)) = self.inner.last() {
                if this.softness > other.softness {
                    self.inner.pop();
                } else if this.softness == Softness::Soft {
                    return;
                }
            }
        }

        self.inner.push(node);
    }

    /// Push a normal word space.
    pub fn push_space(&mut self) {
        let em = self.state.font.font_size();
        self.push(NodeSpacing {
            amount: self.state.par.word_spacing.resolve(em),
            softness: Softness::Soft,
        });
    }

    /// Push text into the context.
    ///
    /// The text is split into lines at newlines.
    pub fn push_text(&mut self, text: &str) {
        let mut newline = false;
        for line in text.split_terminator(is_newline) {
            if newline {
                self.apply_linebreak();
            }

            let node = self.make_text_node(line.into());
            self.push(node);
            newline = true;
        }
    }

    /// Execute a template and return the result as a stack node.
    pub fn exec(&mut self, template: &ValueTemplate) -> Node {
        let dirs = self.state.dirs;
        let aligns = self.state.aligns;

        self.start_group(ContentGroup);
        self.start_par_group();
        template.exec(self);
        self.end_par_group();
        let children = self.end_group::<ContentGroup>().1;

        NodeStack { dirs, aligns, children }.into()
    }

    /// Start a page group based on the active page state.
    ///
    /// The `softness` is a hint on whether empty pages should be kept in the
    /// output.
    ///
    /// This also starts an inner paragraph.
    pub fn start_page_group(&mut self, softness: Softness) {
        self.start_group(PageGroup {
            size: self.state.page.size,
            padding: self.state.page.margins(),
            dirs: self.state.dirs,
            aligns: self.state.aligns,
            softness,
        });
        self.start_par_group();
    }

    /// End a page group, returning its [`Softness`].
    ///
    /// Whether the page is kept when it's empty is decided by `keep_empty`
    /// based on its softness. If kept, the page is pushed to the finished page
    /// runs.
    ///
    /// This also ends an inner paragraph.
    pub fn end_page_group<F>(&mut self, keep_empty: F) -> Softness
    where
        F: FnOnce(Softness) -> bool,
    {
        self.end_par_group();
        let (group, children) = self.end_group::<PageGroup>();
        if !children.is_empty() || keep_empty(group.softness) {
            self.runs.push(NodePages {
                size: group.size,
                child: NodePad {
                    padding: group.padding,
                    child: NodeStack {
                        dirs: group.dirs,
                        aligns: group.aligns,
                        children,
                    }
                    .into(),
                }
                .into(),
            })
        }
        group.softness
    }

    /// Start a paragraph group based on the active text state.
    pub fn start_par_group(&mut self) {
        let em = self.state.font.font_size();
        self.start_group(ParGroup {
            dirs: self.state.dirs,
            aligns: self.state.aligns,
            line_spacing: self.state.par.line_spacing.resolve(em),
        });
    }

    /// End a paragraph group and push it to its parent group if it's not empty.
    pub fn end_par_group(&mut self) {
        let (group, children) = self.end_group::<ParGroup>();
        if !children.is_empty() {
            self.push(NodePar {
                dirs: group.dirs,
                aligns: group.aligns,
                line_spacing: group.line_spacing,
                children,
            });
        }
    }

    /// Start a layouting group.
    ///
    /// All further calls to [`push`](Self::push) will collect nodes for this
    /// group. The given metadata will be returned alongside the collected nodes
    /// in a matching call to [`end_group`](Self::end_group).
    fn start_group<T: 'static>(&mut self, meta: T) {
        self.groups.push((Box::new(meta), std::mem::take(&mut self.inner)));
    }

    /// End a layouting group started with [`start_group`](Self::start_group).
    ///
    /// This returns the stored metadata and the collected nodes.
    #[track_caller]
    fn end_group<T: 'static>(&mut self) -> (T, Vec<Node>) {
        if let Some(&Node::Spacing(spacing)) = self.inner.last() {
            if spacing.softness == Softness::Soft {
                self.inner.pop();
            }
        }

        let (any, outer) = self.groups.pop().expect("no pushed group");
        let group = *any.downcast::<T>().expect("bad group type");
        (group, std::mem::replace(&mut self.inner, outer))
    }

    /// Set the directions if they would apply to different axes, producing an
    /// appropriate error otherwise.
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
    pub fn apply_monospace(&mut self) {
        let families = self.state.font.families_mut();
        families.list.insert(0, "monospace".to_string());
        families.flatten();
    }

    /// Apply a forced line break.
    pub fn apply_linebreak(&mut self) {
        self.end_par_group();
        self.start_par_group();
    }

    /// Apply a forced paragraph break.
    pub fn apply_parbreak(&mut self) {
        self.end_par_group();
        let em = self.state.font.font_size();
        self.push(NodeSpacing {
            amount: self.state.par.par_spacing.resolve(em),
            softness: Softness::Soft,
        });
        self.start_par_group();
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
}

/// Defines how an item interacts with surrounding items.
#[derive(Debug, Copy, Clone, Eq, PartialEq, Ord, PartialOrd)]
pub enum Softness {
    /// A soft item can be skipped in some circumstances.
    Soft,
    /// A hard item is always retained.
    Hard,
}

/// A group for a page run.
#[derive(Debug)]
struct PageGroup {
    dirs: LayoutDirs,
    aligns: LayoutAligns,
    size: Size,
    padding: Sides<Linear>,
    softness: Softness,
}

/// A group for generic content.
#[derive(Debug)]
struct ContentGroup;

/// A group for a paragraph.
#[derive(Debug)]
struct ParGroup {
    dirs: LayoutDirs,
    aligns: LayoutAligns,
    line_spacing: Length,
}
