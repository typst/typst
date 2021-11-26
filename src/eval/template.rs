use std::convert::TryFrom;
use std::fmt::{self, Debug, Formatter};
use std::hash::Hash;
use std::mem;
use std::ops::{Add, AddAssign};
use std::rc::Rc;

use crate::diag::StrResult;
use crate::geom::{Align, Dir, Length, Linear, Paint, Sides, Size, SpecAxis};
use crate::layout::{Layout, PackedNode};
use crate::library::{
    Decoration, DocumentNode, FlowChild, FlowNode, PageNode, ParChild, ParNode,
    PlacedNode, Spacing,
};
use crate::style::Style;
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
    Text(EcoString),
    /// Spacing.
    Spacing(SpecAxis, Spacing),
    /// A decorated template.
    Decorated(Decoration, Template),
    /// An inline node builder.
    Inline(Rc<dyn Fn(&Style) -> PackedNode>),
    /// A block node builder.
    Block(Rc<dyn Fn(&Style) -> PackedNode>),
    /// Save the current style.
    Save,
    /// Restore the last saved style.
    Restore,
    /// A function that can modify the current style.
    Modify(Rc<dyn Fn(&mut Style)>),
}

impl Template {
    /// Create a new, empty template.
    pub fn new() -> Self {
        Self(Rc::new(vec![]))
    }

    /// Create a template from a builder for an inline-level node.
    pub fn from_inline<F, T>(f: F) -> Self
    where
        F: Fn(&Style) -> T + 'static,
        T: Layout + Debug + Hash + 'static,
    {
        let node = TemplateNode::Inline(Rc::new(move |s| f(s).pack()));
        Self(Rc::new(vec![node]))
    }

    /// Create a template from a builder for a block-level node.
    pub fn from_block<F, T>(f: F) -> Self
    where
        F: Fn(&Style) -> T + 'static,
        T: Layout + Debug + Hash + 'static,
    {
        let node = TemplateNode::Block(Rc::new(move |s| f(s).pack()));
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
        self.make_mut().push(TemplateNode::Text(text.into()));
    }

    /// Add text, but in monospace.
    pub fn monospace(&mut self, text: impl Into<EcoString>) {
        self.save();
        self.modify(|style| style.text_mut().monospace = true);
        self.text(text);
        self.restore();
    }

    /// Add spacing along an axis.
    pub fn spacing(&mut self, axis: SpecAxis, spacing: Spacing) {
        self.make_mut().push(TemplateNode::Spacing(axis, spacing));
    }

    /// Register a restorable snapshot.
    pub fn save(&mut self) {
        self.make_mut().push(TemplateNode::Save);
    }

    /// Ensure that later nodes are untouched by style modifications made since
    /// the last snapshot.
    pub fn restore(&mut self) {
        self.make_mut().push(TemplateNode::Restore);
    }

    /// Modify the style.
    pub fn modify<F>(&mut self, f: F)
    where
        F: Fn(&mut Style) + 'static,
    {
        self.make_mut().push(TemplateNode::Modify(Rc::new(f)));
    }

    /// Return a new template which is modified from start to end.
    pub fn modified<F>(self, f: F) -> Self
    where
        F: Fn(&mut Style) + 'static,
    {
        let mut wrapper = Self::new();
        wrapper.save();
        wrapper.modify(f);
        wrapper += self;
        wrapper.restore();
        wrapper
    }

    /// Add a decoration to all contained nodes.
    pub fn decorate(self, deco: Decoration) -> Self {
        Self(Rc::new(vec![TemplateNode::Decorated(deco, self)]))
    }

    /// Pack the template into a layout node.
    pub fn pack(&self, style: &Style) -> PackedNode {
        if let [TemplateNode::Block(f) | TemplateNode::Inline(f)] = self.0.as_slice() {
            f(style)
        } else {
            let mut builder = Builder::new(style, false);
            builder.template(self);
            builder.build_flow().pack()
        }
    }

    /// Build the layout tree resulting from instantiating the template with the
    /// given style.
    pub fn to_document(&self, style: &Style) -> DocumentNode {
        let mut builder = Builder::new(style, true);
        builder.template(self);
        builder.build_document()
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

impl Add<EcoString> for Template {
    type Output = Self;

    fn add(mut self, rhs: EcoString) -> Self::Output {
        Rc::make_mut(&mut self.0).push(TemplateNode::Text(rhs));
        self
    }
}

impl Add<Template> for EcoString {
    type Output = Template;

    fn add(self, mut rhs: Template) -> Self::Output {
        Rc::make_mut(&mut rhs.0).insert(0, TemplateNode::Text(self));
        rhs
    }
}

/// Transforms from template to layout representation.
struct Builder {
    /// The current style.
    style: Style,
    /// Snapshots of the style.
    snapshots: Vec<Style>,
    /// The finished page nodes.
    finished: Vec<PageNode>,
    /// When we are building the top-level layout trees, this contains metrics
    /// of the page. While building a flow, this is `None`.
    page: Option<PageBuilder>,
    /// The currently built flow of paragraphs.
    flow: FlowBuilder,
}

impl Builder {
    /// Create a new builder with a base style.
    fn new(style: &Style, pages: bool) -> Self {
        Self {
            style: style.clone(),
            snapshots: vec![],
            finished: vec![],
            page: pages.then(|| PageBuilder::new(style, true)),
            flow: FlowBuilder::new(style),
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
            TemplateNode::Save => self.snapshots.push(self.style.clone()),
            TemplateNode::Restore => {
                let style = self.snapshots.pop().unwrap();
                let newpage = style.page != self.style.page;
                self.style = style;
                if newpage {
                    self.pagebreak(true, false);
                }
            }
            TemplateNode::Space => self.space(),
            TemplateNode::Linebreak => self.linebreak(),
            TemplateNode::Parbreak => self.parbreak(),
            TemplateNode::Pagebreak(keep) => self.pagebreak(*keep, true),
            TemplateNode::Text(text) => self.text(text),
            TemplateNode::Spacing(axis, amount) => self.spacing(*axis, *amount),
            TemplateNode::Decorated(deco, template) => {
                self.flow.par.push(ParChild::Decorate(deco.clone()));
                self.template(template);
                self.flow.par.push(ParChild::Undecorate);
            }
            TemplateNode::Inline(f) => self.inline(f(&self.style)),
            TemplateNode::Block(f) => self.block(f(&self.style)),
            TemplateNode::Modify(f) => f(&mut self.style),
        }
    }

    /// Push a word space into the active paragraph.
    fn space(&mut self) {
        self.flow.par.push_soft(self.make_text_node(' '));
    }

    /// Apply a forced line break.
    fn linebreak(&mut self) {
        self.flow.par.push_hard(self.make_text_node('\n'));
    }

    /// Apply a forced paragraph break.
    fn parbreak(&mut self) {
        let amount = self.style.par_spacing();
        self.flow.finish_par(&self.style);
        self.flow
            .push_soft(FlowChild::Spacing(Spacing::Linear(amount.into())));
    }

    /// Apply a forced page break.
    fn pagebreak(&mut self, keep: bool, hard: bool) {
        if let Some(builder) = &mut self.page {
            let page = mem::replace(builder, PageBuilder::new(&self.style, hard));
            let flow = mem::replace(&mut self.flow, FlowBuilder::new(&self.style));
            self.finished.extend(page.build(flow.build(), keep));
        }
    }

    /// Push text into the active paragraph.
    fn text(&mut self, text: impl Into<EcoString>) {
        self.flow.par.push(self.make_text_node(text));
    }

    /// Push an inline node into the active paragraph.
    fn inline(&mut self, node: PackedNode) {
        self.flow.par.push(ParChild::Node(node.into()));
    }

    /// Push a block node into the active flow, finishing the active paragraph.
    fn block(&mut self, node: PackedNode) {
        let mut is_placed = false;
        if let Some(placed) = node.downcast::<PlacedNode>() {
            is_placed = true;

            // This prevents paragraph spacing after the placed node if it
            // is completely out-of-flow.
            if placed.out_of_flow() {
                self.flow.last = Last::None;
            }
        }

        self.parbreak();
        self.flow.push(FlowChild::Node(node));
        self.parbreak();

        // This prevents paragraph spacing between the placed node and
        // the paragraph below it.
        if is_placed {
            self.flow.last = Last::None;
        }
    }

    /// Push spacing into the active paragraph or flow depending on the `axis`.
    fn spacing(&mut self, axis: SpecAxis, spacing: Spacing) {
        match axis {
            SpecAxis::Vertical => {
                self.flow.finish_par(&self.style);
                self.flow.push_hard(FlowChild::Spacing(spacing));
            }
            SpecAxis::Horizontal => {
                self.flow.par.push_hard(ParChild::Spacing(spacing));
            }
        }
    }

    /// Finish building and return the created flow.
    fn build_flow(self) -> FlowNode {
        assert!(self.page.is_none());
        self.flow.build()
    }

    /// Finish building and return the created layout tree.
    fn build_document(mut self) -> DocumentNode {
        assert!(self.page.is_some());
        self.pagebreak(true, false);
        DocumentNode { pages: self.finished }
    }

    /// Construct a text node with the given text and settings from the current
    /// style.
    fn make_text_node(&self, text: impl Into<EcoString>) -> ParChild {
        ParChild::Text(text.into(), Rc::clone(&self.style.text))
    }
}

struct PageBuilder {
    size: Size,
    padding: Sides<Linear>,
    fill: Option<Paint>,
    hard: bool,
}

impl PageBuilder {
    fn new(style: &Style, hard: bool) -> Self {
        Self {
            size: style.page.size,
            padding: style.page.margins(),
            fill: style.page.fill,
            hard,
        }
    }

    fn build(self, child: FlowNode, keep: bool) -> Option<PageNode> {
        let Self { size, padding, fill, hard } = self;
        (!child.children.is_empty() || (keep && hard)).then(|| PageNode {
            child: child.pack().padded(padding),
            size,
            fill,
        })
    }
}

struct FlowBuilder {
    children: Vec<FlowChild>,
    last: Last<FlowChild>,
    par: ParBuilder,
}

impl FlowBuilder {
    fn new(style: &Style) -> Self {
        Self {
            children: vec![],
            last: Last::None,
            par: ParBuilder::new(style),
        }
    }

    fn push(&mut self, child: FlowChild) {
        self.children.extend(self.last.any());
        self.children.push(child);
    }

    fn push_soft(&mut self, child: FlowChild) {
        self.last.soft(child);
    }

    fn push_hard(&mut self, child: FlowChild) {
        self.last.hard();
        self.children.push(child);
    }

    fn finish_par(&mut self, style: &Style) {
        let par = mem::replace(&mut self.par, ParBuilder::new(style));
        if let Some(par) = par.build() {
            self.push(par);
        }
    }

    fn build(self) -> FlowNode {
        let Self { mut children, par, mut last } = self;
        if let Some(par) = par.build() {
            children.extend(last.any());
            children.push(par);
        }
        FlowNode { children }
    }
}

struct ParBuilder {
    dir: Dir,
    align: Align,
    leading: Length,
    children: Vec<ParChild>,
    last: Last<ParChild>,
}

impl ParBuilder {
    fn new(style: &Style) -> Self {
        Self {
            dir: style.par.dir,
            align: style.par.align,
            leading: style.leading(),
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
        if let ParChild::Text(text2, style2) = &child {
            if let Some(ParChild::Text(text1, style1)) = self.children.last_mut() {
                if Rc::ptr_eq(style1, style2) {
                    text1.push_str(text2);
                    return;
                }
            }
        }

        self.children.push(child);
    }

    fn build(self) -> Option<FlowChild> {
        let Self { dir, align, leading, children, .. } = self;
        (!children.is_empty())
            .then(|| FlowChild::Node(ParNode { dir, align, leading, children }.pack()))
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
