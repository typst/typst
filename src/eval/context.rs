use std::any::Any;
use std::rc::Rc;

use fontdock::FontStyle;

use super::*;
use crate::diag::Diag;
use crate::diag::{Deco, Feedback, Pass};
use crate::geom::{ChildAlign, Dir, Gen, LayoutDirs, Length, Linear, Sides, Size};
use crate::layout::{
    Expansion, Node, NodePad, NodePages, NodePar, NodeSpacing, NodeStack, NodeText, Tree,
};

/// The context for evaluation.
#[derive(Debug)]
pub struct EvalContext<'a> {
    /// The environment from which resources are gathered.
    pub env: &'a mut Env,
    /// The active scopes.
    pub scopes: Scopes<'a>,
    /// The active evaluation state.
    pub state: State,
    /// The accumulated feedback.
    feedback: Feedback,
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

impl<'a> EvalContext<'a> {
    /// Create a new evaluation context with a base state and scope.
    pub fn new(env: &'a mut Env, scope: &'a Scope, state: State) -> Self {
        Self {
            env,
            scopes: Scopes::new(Some(scope)),
            state,
            groups: vec![],
            inner: vec![],
            runs: vec![],
            feedback: Feedback::new(),
        }
    }

    /// Finish evaluation and return the created document.
    pub fn finish(self) -> Pass<Tree> {
        assert!(self.groups.is_empty(), "unfinished group");
        Pass::new(Tree { runs: self.runs }, self.feedback)
    }

    /// Add a diagnostic to the feedback.
    pub fn diag(&mut self, diag: Spanned<Diag>) {
        self.feedback.diags.push(diag);
    }

    /// Add a decoration to the feedback.
    pub fn deco(&mut self, deco: Spanned<Deco>) {
        self.feedback.decos.push(deco);
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

    /// Start a page group based on the active page state.
    ///
    /// The `softness` is a hint on whether empty pages should be kept in the
    /// output.
    ///
    /// This also starts an inner paragraph.
    pub fn start_page_group(&mut self, softness: Softness) {
        self.start_group(PageGroup {
            size: self.state.page.size,
            expand: self.state.page.expand,
            padding: self.state.page.margins(),
            dirs: self.state.dirs,
            align: self.state.align,
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
                        align: group.align,
                        expand: group.expand,
                        children,
                    }
                    .into(),
                }
                .into(),
            })
        }
        group.softness
    }

    /// Start a content group.
    ///
    /// This also starts an inner paragraph.
    pub fn start_content_group(&mut self) {
        self.start_group(ContentGroup);
        self.start_par_group();
    }

    /// End a content group and return the resulting nodes.
    ///
    /// This also ends an inner paragraph.
    pub fn end_content_group(&mut self) -> Vec<Node> {
        self.end_par_group();
        self.end_group::<ContentGroup>().1
    }

    /// Start a paragraph group based on the active text state.
    pub fn start_par_group(&mut self) {
        let em = self.state.font.font_size();
        self.start_group(ParGroup {
            dirs: self.state.dirs,
            align: self.state.align,
            line_spacing: self.state.par.line_spacing.resolve(em),
        });
    }

    /// End a paragraph group and push it to its parent group if it's not empty.
    pub fn end_par_group(&mut self) {
        let (group, children) = self.end_group::<ParGroup>();
        if !children.is_empty() {
            self.push(NodePar {
                dirs: group.dirs,
                align: group.align,
                // FIXME: This is a hack and should be superseded by something
                //        better.
                cross_expansion: if self.groups.len() <= 1 {
                    Expansion::Fill
                } else {
                    Expansion::Fit
                },
                line_spacing: group.line_spacing,
                children,
            });
        }
    }

    /// Start a layouting group.
    ///
    /// All further calls to [`push`](Self::push) will collect nodes for this group.
    /// The given metadata will be returned alongside the collected nodes
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
            align: self.state.align,
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
    size: Size,
    expand: Spec<Expansion>,
    padding: Sides<Linear>,
    dirs: LayoutDirs,
    align: ChildAlign,
    softness: Softness,
}

/// A group for generic content.
#[derive(Debug)]
struct ContentGroup;

/// A group for a paragraph.
#[derive(Debug)]
struct ParGroup {
    dirs: LayoutDirs,
    align: ChildAlign,
    line_spacing: Length,
}
