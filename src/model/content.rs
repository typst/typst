use std::fmt::Debug;
use std::hash::Hash;
use std::iter::Sum;
use std::mem;
use std::ops::{Add, AddAssign};

use typed_arena::Arena;

use super::{
    CollapsingBuilder, Interruption, Key, Layout, LayoutNode, Show, ShowNode, StyleMap,
    StyleVecBuilder,
};
use crate::diag::StrResult;
use crate::library::layout::{FlowChild, FlowNode, PageNode, PlaceNode, Spacing};
use crate::library::prelude::*;
use crate::library::structure::{DocNode, ListItem, ListNode, ORDERED, UNORDERED};
use crate::library::text::{DecoNode, ParChild, ParNode, UNDERLINE};
use crate::util::EcoString;

/// Composable representation of styled content.
///
/// This results from:
/// - anything written between square brackets in Typst
/// - any node constructor
///
/// Content is represented as a tree of nodes. There are two nodes of special
/// interest:
///
/// 1. A `Styled` node attaches a style map to other content. For example, a
///    single bold word could be represented as a `Styled(Text("Hello"),
///    [TextNode::STRONG: true])` node.
///
/// 2. A `Sequence` node content combines other arbitrary content and is the
///    representation of a "flow" of other nodes. So, when you write `[Hi] +
///    [you]` in Typst, this type's [`Add`] implementation is invoked and the
///    two [`Text`](Self::Text) nodes are combined into a single
///    [`Sequence`](Self::Sequence) node. A sequence may contain nested
///    sequences.
#[derive(PartialEq, Clone, Hash)]
pub enum Content {
    /// A word space.
    Space,
    /// A forced line break. If `true`, the preceding line can still be
    /// justified, if `false` not.
    Linebreak(bool),
    /// Horizontal spacing.
    Horizontal(Spacing),
    /// Plain text.
    Text(EcoString),
    /// A smart quote, may be single (`false`) or double (`true`).
    Quote(bool),
    /// An inline-level node.
    Inline(LayoutNode),
    /// A paragraph break.
    Parbreak,
    /// A column break.
    Colbreak,
    /// Vertical spacing.
    Vertical(Spacing),
    /// A block-level node.
    Block(LayoutNode),
    /// A list / enum item.
    Item(ListItem),
    /// A page break.
    Pagebreak(bool),
    /// A page node.
    Page(PageNode),
    /// A node that can be realized with styles.
    Show(ShowNode),
    /// Content with attached styles.
    Styled(Arc<(Self, StyleMap)>),
    /// A sequence of multiple nodes.
    Sequence(Arc<Vec<Self>>),
}

impl Content {
    /// Create empty content.
    pub fn new() -> Self {
        Self::sequence(vec![])
    }

    /// Create content from an inline-level node.
    pub fn inline<T>(node: T) -> Self
    where
        T: Layout + Debug + Hash + Sync + Send + 'static,
    {
        Self::Inline(node.pack())
    }

    /// Create content from a block-level node.
    pub fn block<T>(node: T) -> Self
    where
        T: Layout + Debug + Hash + Sync + Send + 'static,
    {
        Self::Block(node.pack())
    }

    /// Create content from a showable node.
    pub fn show<T>(node: T) -> Self
    where
        T: Show + Debug + Hash + Sync + Send + 'static,
    {
        Self::Show(node.pack())
    }

    /// Create a new sequence nodes from multiples nodes.
    pub fn sequence(seq: Vec<Self>) -> Self {
        if seq.len() == 1 {
            seq.into_iter().next().unwrap()
        } else {
            Self::Sequence(Arc::new(seq))
        }
    }

    /// Repeat this content `n` times.
    pub fn repeat(&self, n: i64) -> StrResult<Self> {
        let count = usize::try_from(n)
            .map_err(|_| format!("cannot repeat this content {} times", n))?;

        Ok(Self::sequence(vec![self.clone(); count]))
    }

    /// Style this content with a single style property.
    pub fn styled<'k, K: Key<'k>>(mut self, key: K, value: K::Value) -> Self {
        if let Self::Styled(styled) = &mut self {
            if let Some((_, map)) = Arc::get_mut(styled) {
                map.apply(key, value);
                return self;
            }
        }

        Self::Styled(Arc::new((self, StyleMap::with(key, value))))
    }

    /// Style this content with a full style map.
    pub fn styled_with_map(mut self, styles: StyleMap) -> Self {
        if styles.is_empty() {
            return self;
        }

        if let Self::Styled(styled) = &mut self {
            if let Some((_, map)) = Arc::get_mut(styled) {
                map.apply_map(&styles);
                return self;
            }
        }

        Self::Styled(Arc::new((self, styles)))
    }

    /// Underline this content.
    pub fn underlined(self) -> Self {
        Self::show(DecoNode::<UNDERLINE>(self))
    }

    /// Add vertical spacing above and below the node.
    pub fn spaced(self, above: Length, below: Length) -> Self {
        if above.is_zero() && below.is_zero() {
            return self;
        }

        let mut seq = vec![];
        if !above.is_zero() {
            seq.push(Content::Vertical(above.into()));
        }

        seq.push(self);

        if !below.is_zero() {
            seq.push(Content::Vertical(below.into()));
        }

        Self::sequence(seq)
    }

    /// Layout this content into a collection of pages.
    pub fn layout(&self, ctx: &mut Context) -> TypResult<Vec<Arc<Frame>>> {
        let copy = ctx.styles.clone();
        let styles = StyleChain::with_root(&copy);
        let scratch = Scratch::default();

        let mut builder = Builder::new(ctx, &scratch, true);
        builder.accept(self, styles)?;

        let (doc, shared) = builder.into_doc(styles)?;
        doc.layout(ctx, shared)
    }
}

impl Layout for Content {
    fn layout(
        &self,
        ctx: &mut Context,
        regions: &Regions,
        styles: StyleChain,
    ) -> TypResult<Vec<Arc<Frame>>> {
        let scratch = Scratch::default();
        let mut builder = Builder::new(ctx, &scratch, false);
        builder.accept(self, styles)?;
        let (flow, shared) = builder.into_flow(styles)?;
        flow.layout(ctx, regions, shared)
    }

    fn pack(self) -> LayoutNode {
        match self {
            Content::Block(node) => node,
            other => LayoutNode::new(other),
        }
    }
}

impl Default for Content {
    fn default() -> Self {
        Self::new()
    }
}

impl Debug for Content {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        match self {
            Self::Space => f.pad("Space"),
            Self::Linebreak(justified) => write!(f, "Linebreak({justified})"),
            Self::Horizontal(kind) => write!(f, "Horizontal({kind:?})"),
            Self::Text(text) => write!(f, "Text({text:?})"),
            Self::Quote(double) => write!(f, "Quote({double})"),
            Self::Inline(node) => node.fmt(f),
            Self::Parbreak => f.pad("Parbreak"),
            Self::Colbreak => f.pad("Colbreak"),
            Self::Vertical(kind) => write!(f, "Vertical({kind:?})"),
            Self::Block(node) => node.fmt(f),
            Self::Item(item) => item.fmt(f),
            Self::Pagebreak(soft) => write!(f, "Pagebreak({soft})"),
            Self::Page(page) => page.fmt(f),
            Self::Show(node) => node.fmt(f),
            Self::Styled(styled) => {
                let (sub, map) = styled.as_ref();
                map.fmt(f)?;
                sub.fmt(f)
            }
            Self::Sequence(seq) => f.debug_list().entries(seq.iter()).finish(),
        }
    }
}

impl Add for Content {
    type Output = Self;

    fn add(self, rhs: Self) -> Self::Output {
        Self::Sequence(match (self, rhs) {
            (Self::Sequence(mut lhs), Self::Sequence(rhs)) => {
                let mutable = Arc::make_mut(&mut lhs);
                match Arc::try_unwrap(rhs) {
                    Ok(vec) => mutable.extend(vec),
                    Err(rc) => mutable.extend(rc.iter().cloned()),
                }
                lhs
            }
            (Self::Sequence(mut lhs), rhs) => {
                Arc::make_mut(&mut lhs).push(rhs);
                lhs
            }
            (lhs, Self::Sequence(mut rhs)) => {
                Arc::make_mut(&mut rhs).insert(0, lhs);
                rhs
            }
            (lhs, rhs) => Arc::new(vec![lhs, rhs]),
        })
    }
}

impl AddAssign for Content {
    fn add_assign(&mut self, rhs: Self) {
        *self = std::mem::take(self) + rhs;
    }
}

impl Sum for Content {
    fn sum<I: Iterator<Item = Self>>(iter: I) -> Self {
        Self::sequence(iter.collect())
    }
}

/// Builds a document or a flow node from content.
struct Builder<'a, 'ctx> {
    /// The core context.
    ctx: &'ctx mut Context,
    /// Scratch arenas for building.
    scratch: &'a Scratch<'a>,
    /// The current document building state.
    doc: Option<DocBuilder<'a>>,
    /// The current flow building state.
    flow: FlowBuilder<'a>,
    /// The current paragraph building state.
    par: ParBuilder<'a>,
    /// The current list building state.
    list: ListBuilder<'a>,
}

/// Temporary storage arenas for building.
#[derive(Default)]
struct Scratch<'a> {
    /// An arena where intermediate style chains are stored.
    styles: Arena<StyleChain<'a>>,
    /// An arena where intermediate content resulting from show rules is stored.
    templates: Arena<Content>,
}

impl<'a, 'ctx> Builder<'a, 'ctx> {
    fn new(ctx: &'ctx mut Context, scratch: &'a Scratch<'a>, top: bool) -> Self {
        Self {
            ctx,
            scratch,
            doc: top.then(|| DocBuilder::default()),
            flow: FlowBuilder::default(),
            par: ParBuilder::default(),
            list: ListBuilder::default(),
        }
    }

    fn into_doc(
        mut self,
        styles: StyleChain<'a>,
    ) -> TypResult<(DocNode, StyleChain<'a>)> {
        self.interrupt(Interruption::Page, styles, true)?;
        let (pages, shared) = self.doc.unwrap().pages.finish();
        Ok((DocNode(pages), shared))
    }

    fn into_flow(
        mut self,
        styles: StyleChain<'a>,
    ) -> TypResult<(FlowNode, StyleChain<'a>)> {
        self.interrupt(Interruption::Par, styles, false)?;
        let (children, shared) = self.flow.0.finish();
        Ok((FlowNode(children), shared))
    }

    fn accept(&mut self, content: &'a Content, styles: StyleChain<'a>) -> TypResult<()> {
        // Handle special content kinds.
        match content {
            Content::Show(node) => return self.show(node, styles),
            Content::Styled(styled) => return self.styled(styled, styles),
            Content::Sequence(seq) => return self.sequence(seq, styles),
            _ => {}
        }

        if self.list.accept(content, styles) {
            return Ok(());
        }

        self.interrupt(Interruption::List, styles, false)?;

        if self.par.accept(content, styles) {
            return Ok(());
        }

        self.interrupt(Interruption::Par, styles, false)?;

        if self.flow.accept(content, styles) {
            return Ok(());
        }

        let keep = matches!(content, Content::Pagebreak(false));
        self.interrupt(Interruption::Page, styles, keep)?;

        if let Some(doc) = &mut self.doc {
            doc.accept(content, styles);
        }

        // We might want to issue a warning or error for content that wasn't
        // handled (e.g. a pagebreak in a flow building process). However, we
        // don't have the spans here at the moment.
        Ok(())
    }

    fn show(&mut self, node: &ShowNode, styles: StyleChain<'a>) -> TypResult<()> {
        let id = node.id();
        let realized = match styles.realize(self.ctx, node)? {
            Some(content) => content,
            None => node.realize(self.ctx, styles)?,
        };

        let content = node.finalize(self.ctx, styles, realized)?;
        let stored = self.scratch.templates.alloc(content);
        self.accept(stored, styles.unscoped(id))
    }

    fn styled(
        &mut self,
        (content, map): &'a (Content, StyleMap),
        styles: StyleChain<'a>,
    ) -> TypResult<()> {
        let stored = self.scratch.styles.alloc(styles);
        let styles = map.chain(stored);
        let intr = map.interruption();

        if let Some(intr) = intr {
            self.interrupt(intr, styles, false)?;
        }

        self.accept(content, styles)?;

        if let Some(intr) = intr {
            self.interrupt(intr, styles, true)?;
        }

        Ok(())
    }

    fn interrupt(
        &mut self,
        intr: Interruption,
        styles: StyleChain<'a>,
        keep: bool,
    ) -> TypResult<()> {
        if intr >= Interruption::List && !self.list.is_empty() {
            mem::take(&mut self.list).finish(self)?;
        }

        if intr >= Interruption::Par {
            if !self.par.is_empty() {
                self.flow.0.weak(FlowChild::Leading, 0, styles);
                mem::take(&mut self.par).finish(self);
            }
            self.flow.0.weak(FlowChild::Leading, 0, styles);
        }

        if intr >= Interruption::Page {
            if let Some(doc) = &mut self.doc {
                if !self.flow.is_empty() || (doc.keep_next && keep) {
                    mem::take(&mut self.flow).finish(doc, styles);
                }
                doc.keep_next = !keep;
            }
        }

        Ok(())
    }

    fn sequence(&mut self, seq: &'a [Content], styles: StyleChain<'a>) -> TypResult<()> {
        for content in seq {
            self.accept(content, styles)?;
        }
        Ok(())
    }
}

/// Accepts pagebreaks and pages.
struct DocBuilder<'a> {
    /// The page runs built so far.
    pages: StyleVecBuilder<'a, PageNode>,
    /// Whether to keep a following page even if it is empty.
    keep_next: bool,
}

impl<'a> DocBuilder<'a> {
    fn accept(&mut self, content: &'a Content, styles: StyleChain<'a>) {
        match content {
            Content::Pagebreak(soft) => {
                self.keep_next = !soft;
            }
            Content::Page(page) => {
                self.pages.push(page.clone(), styles);
                self.keep_next = false;
            }
            _ => {}
        }
    }
}

impl Default for DocBuilder<'_> {
    fn default() -> Self {
        Self {
            pages: StyleVecBuilder::new(),
            keep_next: true,
        }
    }
}

/// Accepts flow content.
#[derive(Default)]
struct FlowBuilder<'a>(CollapsingBuilder<'a, FlowChild>);

impl<'a> FlowBuilder<'a> {
    fn accept(&mut self, content: &'a Content, styles: StyleChain<'a>) -> bool {
        match content {
            Content::Parbreak => {
                self.0.weak(FlowChild::Parbreak, 1, styles);
            }
            Content::Colbreak => {
                self.0.destructive(FlowChild::Colbreak, styles);
            }
            Content::Vertical(kind) => {
                let child = FlowChild::Spacing(*kind);
                if kind.is_fractional() {
                    self.0.destructive(child, styles);
                } else {
                    self.0.ignorant(child, styles);
                }
            }
            Content::Block(node) => {
                let child = FlowChild::Node(node.clone());
                if node.is::<PlaceNode>() {
                    self.0.ignorant(child, styles);
                } else {
                    self.0.supportive(child, styles);
                }
            }
            _ => return false,
        }

        true
    }

    fn finish(self, doc: &mut DocBuilder<'a>, styles: StyleChain<'a>) {
        let (flow, shared) = self.0.finish();
        let styles = if flow.is_empty() { styles } else { shared };
        let node = PageNode(FlowNode(flow).pack());
        doc.pages.push(node, styles);
    }

    fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}

/// Accepts paragraph content.
#[derive(Default)]
struct ParBuilder<'a>(CollapsingBuilder<'a, ParChild>);

impl<'a> ParBuilder<'a> {
    fn accept(&mut self, content: &'a Content, styles: StyleChain<'a>) -> bool {
        match content {
            Content::Space => {
                self.0.weak(ParChild::Text(' '.into()), 0, styles);
            }
            Content::Linebreak(justified) => {
                let c = if *justified { '\u{2028}' } else { '\n' };
                self.0.destructive(ParChild::Text(c.into()), styles);
            }
            Content::Horizontal(kind) => {
                let child = ParChild::Spacing(*kind);
                if kind.is_fractional() {
                    self.0.destructive(child, styles);
                } else {
                    self.0.ignorant(child, styles);
                }
            }
            Content::Quote(double) => {
                self.0.supportive(ParChild::Quote(*double), styles);
            }
            Content::Text(text) => {
                self.0.supportive(ParChild::Text(text.clone()), styles);
            }
            Content::Inline(node) => {
                self.0.supportive(ParChild::Node(node.clone()), styles);
            }
            _ => return false,
        }

        true
    }

    fn finish(self, parent: &mut Builder<'a, '_>) {
        let (mut children, shared) = self.0.finish();
        if children.is_empty() {
            return;
        }

        // Paragraph indent should only apply if the paragraph starts with
        // text and follows directly after another paragraph.
        let indent = shared.get(ParNode::INDENT);
        if !indent.is_zero()
            && children
                .items()
                .find_map(|child| match child {
                    ParChild::Spacing(_) => None,
                    ParChild::Text(_) | ParChild::Quote(_) => Some(true),
                    ParChild::Node(_) => Some(false),
                })
                .unwrap_or_default()
            && parent
                .flow
                .0
                .items()
                .rev()
                .find_map(|child| match child {
                    FlowChild::Leading => None,
                    FlowChild::Parbreak => None,
                    FlowChild::Node(node) => Some(node.is::<ParNode>()),
                    FlowChild::Spacing(_) => Some(false),
                    FlowChild::Colbreak => Some(false),
                })
                .unwrap_or_default()
        {
            children.push_front(ParChild::Spacing(indent.into()));
        }

        let node = ParNode(children).pack();
        parent.flow.0.supportive(FlowChild::Node(node), shared);
    }

    fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}

/// Accepts list / enum items, spaces, paragraph breaks.
struct ListBuilder<'a> {
    /// The list items collected so far.
    items: StyleVecBuilder<'a, ListItem>,
    /// Whether the list contains no paragraph breaks.
    tight: bool,
    /// Trailing content for which it is unclear whether it is part of the list.
    staged: Vec<(&'a Content, StyleChain<'a>)>,
}

impl<'a> ListBuilder<'a> {
    fn accept(&mut self, content: &'a Content, styles: StyleChain<'a>) -> bool {
        match content {
            Content::Space if !self.items.is_empty() => {
                self.staged.push((content, styles));
            }
            Content::Parbreak if !self.items.is_empty() => {
                self.staged.push((content, styles));
            }
            Content::Item(item)
                if self
                    .items
                    .items()
                    .next()
                    .map_or(true, |first| item.kind == first.kind) =>
            {
                self.items.push(item.clone(), styles);
                self.tight &= self.staged.drain(..).all(|(t, _)| *t != Content::Parbreak);
            }
            _ => return false,
        }

        true
    }

    fn finish(self, parent: &mut Builder<'a, '_>) -> TypResult<()> {
        let (items, shared) = self.items.finish();
        let kind = match items.items().next() {
            Some(item) => item.kind,
            None => return Ok(()),
        };

        let tight = self.tight;
        let content = match kind {
            UNORDERED => Content::show(ListNode::<UNORDERED> { start: 1, tight, items }),
            ORDERED | _ => Content::show(ListNode::<ORDERED> { start: 1, tight, items }),
        };

        let stored = parent.scratch.templates.alloc(content);
        parent.accept(stored, shared)?;

        for (content, styles) in self.staged {
            parent.accept(content, styles)?;
        }

        Ok(())
    }

    fn is_empty(&self) -> bool {
        self.items.is_empty()
    }
}

impl Default for ListBuilder<'_> {
    fn default() -> Self {
        Self {
            items: StyleVecBuilder::default(),
            tight: true,
            staged: vec![],
        }
    }
}
