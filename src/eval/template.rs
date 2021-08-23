use std::convert::TryFrom;
use std::fmt::{self, Debug, Display, Formatter};
use std::mem;
use std::ops::{Add, AddAssign};
use std::rc::Rc;

use super::{State, Str};
use crate::diag::StrResult;
use crate::geom::{Align, Dir, Gen, GenAxis, Length, Linear, Sides, Size};
use crate::layout::{
    LayoutNode, LayoutTree, PadNode, PageRun, ParChild, ParNode, StackChild, StackNode,
};
use crate::util::EcoString;

/// A template value: `[*Hi* there]`.
#[derive(Default, Clone)]
pub struct Template(Rc<Vec<TemplateNode>>);

/// One node in a template.
#[derive(Clone)]
enum TemplateNode {
    /// A word space.
    Space,
    /// A line break.
    Linebreak,
    /// A paragraph break.
    Parbreak,
    /// A page break.
    Pagebreak(bool),
    /// Plain text.
    Text(EcoString, Vec<Decoration>),
    /// Spacing.
    Spacing(GenAxis, Linear),
    /// An inline node builder.
    Inline(Rc<dyn Fn(&State) -> LayoutNode>, Vec<Decoration>),
    /// An block node builder.
    Block(Rc<dyn Fn(&State) -> LayoutNode>),
    /// Save the current state.
    Save,
    /// Restore the last saved state.
    Restore,
    /// A function that can modify the current state.
    Modify(Rc<dyn Fn(&mut State)>),
}

/// A template node decoration.
#[derive(Debug, Clone, Hash)]
pub enum Decoration {
    /// A link.
    Link(EcoString),
}

impl Template {
    /// Create a new, empty template.
    pub fn new() -> Self {
        Self(Rc::new(vec![]))
    }

    /// Create a template from a builder for an inline-level node.
    pub fn from_inline<F, T>(f: F) -> Self
    where
        F: Fn(&State) -> T + 'static,
        T: Into<LayoutNode>,
    {
        let node = TemplateNode::Inline(Rc::new(move |s| f(s).into()), vec![]);
        Self(Rc::new(vec![node]))
    }

    /// Create a template from a builder for a block-level node.
    pub fn from_block<F, T>(f: F) -> Self
    where
        F: Fn(&State) -> T + 'static,
        T: Into<LayoutNode>,
    {
        let node = TemplateNode::Block(Rc::new(move |s| f(s).into()));
        Self(Rc::new(vec![node]))
    }

    /// Add a word space to the template.
    pub fn space(&mut self) {
        self.make_mut().push(TemplateNode::Space);
    }

    /// Add a line break to the template.
    pub fn linebreak(&mut self) {
        self.make_mut().push(TemplateNode::Linebreak);
    }

    /// Add a paragraph break to the template.
    pub fn parbreak(&mut self) {
        self.make_mut().push(TemplateNode::Parbreak);
    }

    /// Add a page break to the template.
    pub fn pagebreak(&mut self, keep: bool) {
        self.make_mut().push(TemplateNode::Pagebreak(keep));
    }

    /// Add text to the template.
    pub fn text(&mut self, text: impl Into<EcoString>) {
        self.make_mut().push(TemplateNode::Text(text.into(), vec![]));
    }

    /// Add text, but in monospace.
    pub fn monospace(&mut self, text: impl Into<EcoString>) {
        self.save();
        self.modify(|state| state.font_mut().monospace = true);
        self.text(text);
        self.restore();
    }

    /// Add spacing along an axis.
    pub fn spacing(&mut self, axis: GenAxis, spacing: Linear) {
        self.make_mut().push(TemplateNode::Spacing(axis, spacing));
    }

    /// Add a decoration to the last template node.
    pub fn decorate(&mut self, decoration: Decoration) {
        for node in self.make_mut() {
            match node {
                TemplateNode::Text(_, decos) => decos.push(decoration.clone()),
                TemplateNode::Inline(_, decos) => decos.push(decoration.clone()),
                _ => {}
            }
        }
    }

    /// Register a restorable snapshot.
    pub fn save(&mut self) {
        self.make_mut().push(TemplateNode::Save);
    }

    /// Ensure that later nodes are untouched by state modifications made since
    /// the last snapshot.
    pub fn restore(&mut self) {
        self.make_mut().push(TemplateNode::Restore);
    }

    /// Modify the state.
    pub fn modify<F>(&mut self, f: F)
    where
        F: Fn(&mut State) + 'static,
    {
        self.make_mut().push(TemplateNode::Modify(Rc::new(f)));
    }

    /// Build the stack node resulting from instantiating the template in the
    /// given state.
    pub fn to_stack(&self, state: &State) -> StackNode {
        let mut builder = Builder::new(state, false);
        builder.template(self);
        builder.build_stack()
    }

    /// Build the layout tree resulting from instantiating the template in the
    /// given state.
    pub fn to_tree(&self, state: &State) -> LayoutTree {
        let mut builder = Builder::new(state, true);
        builder.template(self);
        builder.build_tree()
    }

    /// Repeat this template `n` times.
    pub fn repeat(&self, n: i64) -> StrResult<Self> {
        let count = usize::try_from(n)
            .ok()
            .and_then(|n| self.0.len().checked_mul(n))
            .ok_or_else(|| format!("cannot repeat this template {} times", n))?;

        Ok(Self(Rc::new(
            self.0.iter().cloned().cycle().take(count).collect(),
        )))
    }

    /// Return a mutable reference to the inner vector.
    fn make_mut(&mut self) -> &mut Vec<TemplateNode> {
        Rc::make_mut(&mut self.0)
    }
}

impl Debug for Template {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        f.pad("Template { .. }")
    }
}

impl Display for Template {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        f.pad("<template>")
    }
}

impl PartialEq for Template {
    fn eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.0, &other.0)
    }
}

impl Add for Template {
    type Output = Self;

    fn add(mut self, rhs: Self) -> Self::Output {
        self += rhs;
        self
    }
}

impl AddAssign for Template {
    fn add_assign(&mut self, rhs: Template) {
        let sink = Rc::make_mut(&mut self.0);
        match Rc::try_unwrap(rhs.0) {
            Ok(source) => sink.extend(source),
            Err(rc) => sink.extend(rc.iter().cloned()),
        }
    }
}

impl Add<Str> for Template {
    type Output = Self;

    fn add(mut self, rhs: Str) -> Self::Output {
        Rc::make_mut(&mut self.0).push(TemplateNode::Text(rhs.into(), vec![]));
        self
    }
}

impl Add<Template> for Str {
    type Output = Template;

    fn add(self, mut rhs: Template) -> Self::Output {
        Rc::make_mut(&mut rhs.0).insert(0, TemplateNode::Text(self.into(), vec![]));
        rhs
    }
}

/// Transforms from template to layout representation.
struct Builder {
    /// The active state.
    state: State,
    /// Snapshots of the state.
    snapshots: Vec<State>,
    /// The tree of finished page runs.
    tree: LayoutTree,
    /// When we are building the top-level layout trees, this contains metrics
    /// of the page. While building a stack, this is `None`.
    page: Option<PageBuilder>,
    /// The currently built stack of paragraphs.
    stack: StackBuilder,
}

impl Builder {
    /// Create a new builder with a base state.
    fn new(state: &State, pages: bool) -> Self {
        Self {
            state: state.clone(),
            snapshots: vec![],
            tree: LayoutTree { runs: vec![] },
            page: pages.then(|| PageBuilder::new(state, true)),
            stack: StackBuilder::new(state),
        }
    }

    /// Build a template.
    fn template(&mut self, template: &Template) {
        for node in template.0.iter() {
            self.node(node);
        }
    }

    /// Build a template node.
    fn node(&mut self, node: &TemplateNode) {
        match node {
            TemplateNode::Save => self.snapshots.push(self.state.clone()),
            TemplateNode::Restore => {
                let state = self.snapshots.pop().unwrap();
                let newpage = state.page != self.state.page;
                self.state = state;
                if newpage {
                    self.pagebreak(true, false);
                }
            }
            TemplateNode::Space => self.space(),
            TemplateNode::Linebreak => self.linebreak(),
            TemplateNode::Parbreak => self.parbreak(),
            TemplateNode::Pagebreak(keep) => self.pagebreak(*keep, true),
            TemplateNode::Text(text, decorations) => self.text(text, decorations),
            TemplateNode::Spacing(axis, amount) => self.spacing(*axis, *amount),
            TemplateNode::Inline(f, decorations) => {
                self.inline(f(&self.state), decorations)
            }
            TemplateNode::Block(f) => self.block(f(&self.state)),
            TemplateNode::Modify(f) => f(&mut self.state),
        }
    }

    /// Push a word space into the active paragraph.
    fn space(&mut self) {
        self.stack.par.push_soft(self.make_text_node(' '));
    }

    /// Apply a forced line break.
    fn linebreak(&mut self) {
        self.stack.par.push_hard(self.make_text_node('\n'));
    }

    /// Apply a forced paragraph break.
    fn parbreak(&mut self) {
        let amount = self.state.par_spacing();
        self.stack.finish_par(&self.state);
        self.stack.push_soft(StackChild::Spacing(amount.into()));
    }

    /// Apply a forced page break.
    fn pagebreak(&mut self, keep: bool, hard: bool) {
        if let Some(builder) = &mut self.page {
            let page = mem::replace(builder, PageBuilder::new(&self.state, hard));
            let stack = mem::replace(&mut self.stack, StackBuilder::new(&self.state));
            self.tree.runs.extend(page.build(stack.build(), keep));
        }
    }

    /// Push text into the active paragraph.
    ///
    /// The text is split into lines at newlines.
    fn text(&mut self, text: impl Into<EcoString>, decorations: &[Decoration]) {
        if self.stack.par.push(self.make_text_node(text)) {
            for deco in decorations {
                self.stack.par.push_decoration(deco.clone());
            }
        }
    }

    /// Push an inline node into the active paragraph.
    fn inline(&mut self, node: impl Into<LayoutNode>, decorations: &[Decoration]) {
        let align = self.state.aligns.inline;
        if self.stack.par.push(ParChild::Any(node.into(), align)) {
            for deco in decorations {
                self.stack.par.push_decoration(deco.clone());
            }
        }
    }

    /// Push a block node into the active stack, finishing the active paragraph.
    fn block(&mut self, node: impl Into<LayoutNode>) {
        self.parbreak();
        let aligns = self.state.aligns;
        self.stack.push(StackChild::Any(node.into(), aligns));
        self.parbreak();
    }

    /// Push spacing into the active paragraph or stack depending on the `axis`.
    fn spacing(&mut self, axis: GenAxis, amount: Linear) {
        match axis {
            GenAxis::Block => {
                self.stack.finish_par(&self.state);
                self.stack.push_hard(StackChild::Spacing(amount));
            }
            GenAxis::Inline => {
                self.stack.par.push_hard(ParChild::Spacing(amount));
            }
        }
    }

    /// Finish building and return the created stack.
    fn build_stack(self) -> StackNode {
        assert!(self.page.is_none());
        self.stack.build()
    }

    /// Finish building and return the created layout tree.
    fn build_tree(mut self) -> LayoutTree {
        assert!(self.page.is_some());
        self.pagebreak(true, false);
        self.tree
    }

    /// Construct a text node with the given text and settings from the active
    /// state.
    fn make_text_node(&self, text: impl Into<EcoString>) -> ParChild {
        ParChild::Text(
            text.into(),
            self.state.aligns.inline,
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
            dirs: state.dirs,
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
        StackNode { dirs, children }
    }
}

struct ParBuilder {
    aligns: Gen<Align>,
    dir: Dir,
    line_spacing: Length,
    children: Vec<ParChild>,
    decorations: Vec<(usize, Decoration)>,
    last: Last<ParChild>,
}

impl ParBuilder {
    fn new(state: &State) -> Self {
        Self {
            aligns: state.aligns,
            dir: state.dirs.inline,
            line_spacing: state.line_spacing(),
            children: vec![],
            decorations: vec![],
            last: Last::None,
        }
    }

    fn push(&mut self, child: ParChild) -> bool {
        if let Some(soft) = self.last.any() {
            self.push_inner(soft);
        }
        self.push_inner(child)
    }

    fn push_soft(&mut self, child: ParChild) {
        self.last.soft(child);
    }

    fn push_hard(&mut self, child: ParChild) {
        self.last.hard();
        self.push_inner(child);
    }

    fn push_inner(&mut self, child: ParChild) -> bool {
        if let ParChild::Text(curr_text, curr_align, curr_props) = &child {
            if let Some(ParChild::Text(prev_text, prev_align, prev_props)) =
                self.children.last_mut()
            {
                if prev_align == curr_align && Rc::ptr_eq(prev_props, curr_props) {
                    prev_text.push_str(&curr_text);
                    return false;
                }
            }
        }

        self.children.push(child);
        true
    }

    fn push_decoration(&mut self, deco: Decoration) {
        self.decorations.push((self.children.len() - 1, deco));
    }

    fn build(self) -> Option<StackChild> {
        let Self {
            aligns,
            dir,
            line_spacing,
            children,
            decorations,
            ..
        } = self;
        (!children.is_empty()).then(|| {
            let node = ParNode { dir, line_spacing, children, decorations };
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
