use std::fmt::Debug;
use std::hash::Hash;
use std::iter::Sum;
use std::mem;
use std::ops::{Add, AddAssign};

use typed_arena::Arena;

use super::{CollapsingBuilder, Interruption, Property, StyleMap, StyleVecBuilder};
use crate::diag::StrResult;
use crate::layout::{Layout, PackedNode};
use crate::library::prelude::*;
use crate::library::{
    FlowChild, FlowNode, PageNode, ParChild, ParNode, PlaceNode, SpacingKind, TextNode,
};
use crate::util::EcoString;
use crate::Context;

/// Composable representation of styled content.
///
/// This results from:
/// - anything written between square brackets in Typst
/// - any class constructor
///
/// This enum has two notable variants:
///
/// 1. A `Styled` template attaches a style map to a template. This map affects
///    the whole subtemplate. For example, a single bold word could be
///    represented as a `Styled(Text("Hello"), [TextNode::STRONG: true])`
///    template.
///
/// 2. A `Sequence` template combines multiple arbitrary templates and is the
///    representation of a "flow" of content. So, when you write `[Hi] + [you]`
///    in Typst, this type's [`Add`] implementation is invoked and the two
///    [`Text`](Self::Text) templates are combined into a single
///    [`Sequence`](Self::Sequence) template.
///
///    A sequence may contain nested sequences (meaning this variant effectively
///    allows templates to form trees). All nested sequences can equivalently be
///    represented as a single flat sequence, but allowing nesting doesn't hurt
///    since we can just recurse into the nested sequences. Also, in theory,
///    this allows better complexity when adding large sequence nodes just like
///    for something like a text rope.
#[derive(PartialEq, Clone, Hash)]
pub enum Template {
    /// A word space.
    Space,
    /// A line break.
    Linebreak,
    /// Horizontal spacing.
    Horizontal(SpacingKind),
    /// Plain text.
    Text(EcoString),
    /// An inline-level node.
    Inline(PackedNode),
    /// A paragraph break.
    Parbreak,
    /// A column break.
    Colbreak,
    /// Vertical spacing.
    Vertical(SpacingKind),
    /// A block-level node.
    Block(PackedNode),
    /// A page break.
    Pagebreak,
    /// A page node.
    Page(PageNode),
    /// A template with attached styles.
    Styled(Box<Self>, StyleMap),
    /// A sequence of multiple subtemplates.
    Sequence(Vec<Self>),
}

impl Template {
    /// Create an empty template.
    pub fn new() -> Self {
        Self::Sequence(vec![])
    }

    /// Create a template from an inline-level node.
    pub fn inline<T>(node: T) -> Self
    where
        T: Layout + Debug + Hash + Sync + Send + 'static,
    {
        Self::Inline(node.pack())
    }

    /// Create a template from a block-level node.
    pub fn block<T>(node: T) -> Self
    where
        T: Layout + Debug + Hash + Sync + Send + 'static,
    {
        Self::Block(node.pack())
    }

    /// Layout this template into a collection of pages.
    pub fn layout(&self, ctx: &mut Context) -> Vec<Arc<Frame>> {
        let (mut ctx, styles) = LayoutContext::new(ctx);
        let (pages, shared) = Builder::build_pages(self);
        let styles = shared.chain(&styles);
        pages
            .iter()
            .flat_map(|(page, map)| page.layout(&mut ctx, map.chain(&styles)))
            .collect()
    }

    /// Style this template with a single property.
    pub fn styled<P: Property>(mut self, key: P, value: P::Value) -> Self {
        if let Self::Styled(_, map) = &mut self {
            map.set(key, value);
            self
        } else {
            self.styled_with_map(StyleMap::with(key, value))
        }
    }

    /// Style this template with a full style map.
    pub fn styled_with_map(mut self, styles: StyleMap) -> Self {
        if styles.is_empty() {
            self
        } else if let Self::Styled(_, map) = &mut self {
            map.apply(&styles);
            self
        } else {
            Self::Styled(Box::new(self), styles)
        }
    }

    /// Style this template in monospace.
    pub fn monospaced(self) -> Self {
        self.styled(TextNode::MONOSPACE, true)
    }

    /// Repeat this template `n` times.
    pub fn repeat(&self, n: i64) -> StrResult<Self> {
        let count = usize::try_from(n)
            .map_err(|_| format!("cannot repeat this template {} times", n))?;

        Ok(Self::Sequence(vec![self.clone(); count]))
    }
}

impl Default for Template {
    fn default() -> Self {
        Self::new()
    }
}

impl Debug for Template {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        match self {
            Self::Space => f.pad("Space"),
            Self::Linebreak => f.pad("Linebreak"),
            Self::Horizontal(kind) => write!(f, "Horizontal({kind:?})"),
            Self::Text(text) => write!(f, "Text({text:?})"),
            Self::Inline(node) => {
                f.write_str("Inline(")?;
                node.fmt(f)?;
                f.write_str(")")
            }
            Self::Parbreak => f.pad("Parbreak"),
            Self::Colbreak => f.pad("Colbreak"),
            Self::Vertical(kind) => write!(f, "Vertical({kind:?})"),
            Self::Block(node) => {
                f.write_str("Block(")?;
                node.fmt(f)?;
                f.write_str(")")
            }
            Self::Pagebreak => f.pad("Pagebreak"),
            Self::Page(page) => page.fmt(f),
            Self::Styled(sub, map) => {
                map.fmt(f)?;
                sub.fmt(f)
            }
            Self::Sequence(seq) => f.debug_list().entries(seq).finish(),
        }
    }
}

impl Add for Template {
    type Output = Self;

    fn add(self, rhs: Self) -> Self::Output {
        Self::Sequence(match (self, rhs) {
            (Self::Sequence(mut lhs), Self::Sequence(rhs)) => {
                lhs.extend(rhs);
                lhs
            }
            (Self::Sequence(mut lhs), rhs) => {
                lhs.push(rhs);
                lhs
            }
            (lhs, Self::Sequence(mut rhs)) => {
                rhs.insert(0, lhs);
                rhs
            }
            (lhs, rhs) => {
                vec![lhs, rhs]
            }
        })
    }
}

impl AddAssign for Template {
    fn add_assign(&mut self, rhs: Self) {
        *self = mem::take(self) + rhs;
    }
}

impl Sum for Template {
    fn sum<I: Iterator<Item = Self>>(iter: I) -> Self {
        Self::Sequence(iter.collect())
    }
}

impl Layout for Template {
    fn layout(
        &self,
        ctx: &mut LayoutContext,
        regions: &Regions,
        styles: StyleChain,
    ) -> Vec<Constrained<Arc<Frame>>> {
        let (flow, shared) = Builder::build_flow(self);
        flow.layout(ctx, regions, shared.chain(&styles))
    }

    fn pack(self) -> PackedNode {
        match self {
            Template::Block(node) => node,
            other => PackedNode::new(other),
        }
    }
}

/// Builds a flow or page nodes from a template.
struct Builder<'a> {
    /// An arena where intermediate style chains are stored.
    arena: &'a Arena<StyleChain<'a>>,
    /// The already built page runs.
    pages: Option<StyleVecBuilder<'a, PageNode>>,
    /// The currently built flow.
    flow: CollapsingBuilder<'a, FlowChild>,
    /// The currently built paragraph.
    par: CollapsingBuilder<'a, ParChild>,
    /// Whether to keep the next page even if it is empty.
    keep_next: bool,
}

impl<'a> Builder<'a> {
    /// Build page runs from a template.
    fn build_pages(template: &Template) -> (StyleVec<PageNode>, StyleMap) {
        let arena = Arena::new();

        let mut builder = Builder::prepare(&arena, true);
        builder.process(template, StyleChain::default());
        builder.finish_page(true, false, StyleChain::default());

        let (pages, shared) = builder.pages.unwrap().finish();
        (pages, shared.to_map())
    }

    /// Build a subflow from a template.
    fn build_flow(template: &Template) -> (FlowNode, StyleMap) {
        let arena = Arena::new();

        let mut builder = Builder::prepare(&arena, false);
        builder.process(template, StyleChain::default());
        builder.finish_par();

        let (flow, shared) = builder.flow.finish();
        (FlowNode(flow), shared.to_map())
    }

    /// Prepare the builder.
    fn prepare(arena: &'a Arena<StyleChain<'a>>, top: bool) -> Self {
        Self {
            arena,
            pages: top.then(|| StyleVecBuilder::new()),
            flow: CollapsingBuilder::new(),
            par: CollapsingBuilder::new(),
            keep_next: true,
        }
    }

    /// Process a template.
    fn process(&mut self, template: &'a Template, styles: StyleChain<'a>) {
        match template {
            Template::Space => {
                self.par.weak(ParChild::Text(' '.into()), styles);
            }
            Template::Linebreak => {
                self.par.destructive(ParChild::Text('\n'.into()), styles);
            }
            Template::Horizontal(kind) => {
                let child = ParChild::Spacing(*kind);
                if kind.is_fractional() {
                    self.par.destructive(child, styles);
                } else {
                    self.par.ignorant(child, styles);
                }
            }
            Template::Text(text) => {
                self.par.supportive(ParChild::Text(text.clone()), styles);
            }
            Template::Inline(node) => {
                self.par.supportive(ParChild::Node(node.clone()), styles);
            }
            Template::Parbreak => {
                self.finish_par();
                self.flow.weak(FlowChild::Parbreak, styles);
            }
            Template::Colbreak => {
                self.finish_par();
                self.flow.destructive(FlowChild::Colbreak, styles);
            }
            Template::Vertical(kind) => {
                self.finish_par();
                let child = FlowChild::Spacing(*kind);
                if kind.is_fractional() {
                    self.flow.destructive(child, styles);
                } else {
                    self.flow.ignorant(child, styles);
                }
            }
            Template::Block(node) => {
                self.finish_par();
                let child = FlowChild::Node(node.clone());
                if node.is::<PlaceNode>() {
                    self.flow.ignorant(child, styles);
                } else {
                    self.flow.supportive(child, styles);
                }
            }
            Template::Pagebreak => {
                self.finish_page(true, true, styles);
            }
            Template::Page(page) => {
                self.finish_page(false, false, styles);
                if let Some(pages) = &mut self.pages {
                    pages.push(page.clone(), styles);
                }
            }
            Template::Styled(sub, map) => {
                let interruption = map.interruption();
                match interruption {
                    Some(Interruption::Page) => self.finish_page(false, true, styles),
                    Some(Interruption::Par) => self.finish_par(),
                    None => {}
                }

                let outer = self.arena.alloc(styles);
                let styles = map.chain(outer);
                self.process(sub, styles);

                match interruption {
                    Some(Interruption::Page) => self.finish_page(true, false, styles),
                    Some(Interruption::Par) => self.finish_par(),
                    None => {}
                }
            }
            Template::Sequence(seq) => {
                for sub in seq {
                    self.process(sub, styles);
                }
            }
        }
    }

    /// Finish the currently built paragraph.
    fn finish_par(&mut self) {
        let (par, shared) = mem::take(&mut self.par).finish();
        if !par.is_empty() {
            let node = ParNode(par).pack();
            self.flow.supportive(FlowChild::Node(node), shared);
        }
    }

    /// Finish the currently built page run.
    fn finish_page(&mut self, keep_last: bool, keep_next: bool, styles: StyleChain<'a>) {
        self.finish_par();
        if let Some(pages) = &mut self.pages {
            let (flow, shared) = mem::take(&mut self.flow).finish();
            if !flow.is_empty() || (keep_last && self.keep_next) {
                let styles = if flow.is_empty() { styles } else { shared };
                let node = PageNode(FlowNode(flow).pack());
                pages.push(node, styles);
            }
        }
        self.keep_next = keep_next;
    }
}
