use std::fmt::Debug;
use std::hash::Hash;
use std::iter::Sum;
use std::ops::{Add, AddAssign};

use typed_arena::Arena;

use super::{
    CollapsingBuilder, Interruption, Key, Layout, LayoutNode, Show, ShowNode, StyleMap,
    StyleVecBuilder,
};
use crate::diag::StrResult;
use crate::library::layout::{FlowChild, FlowNode, PageNode, PlaceNode, Spacing};
use crate::library::prelude::*;
use crate::library::structure::{ListItem, ListKind, ListNode, ORDERED, UNORDERED};
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
    /// A line break.
    Linebreak,
    /// Horizontal spacing.
    Horizontal(Spacing),
    /// Plain text.
    Text(EcoString),
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
    /// An item in an unordered list.
    List(ListItem),
    /// An item in an ordered list.
    Enum(ListItem),
    /// A page break.
    Pagebreak,
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

    /// Layout this content into a collection of pages.
    pub fn layout(&self, ctx: &mut Context) -> TypResult<Vec<Arc<Frame>>> {
        let sya = Arena::new();
        let tpa = Arena::new();

        let styles = ctx.styles.clone();
        let styles = StyleChain::with_root(&styles);

        let mut builder = Builder::new(&sya, &tpa, true);
        builder.process(ctx, self, styles)?;
        builder.finish(ctx, styles)?;

        let mut frames = vec![];
        let (pages, shared) = builder.pages.unwrap().finish();

        for (page, map) in pages.iter() {
            let number = 1 + frames.len();
            frames.extend(page.layout(ctx, number, map.chain(&shared))?);
        }

        Ok(frames)
    }
}

impl Default for Content {
    fn default() -> Self {
        Self::new()
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

impl Layout for Content {
    fn layout(
        &self,
        ctx: &mut Context,
        regions: &Regions,
        styles: StyleChain,
    ) -> TypResult<Vec<Arc<Frame>>> {
        let sya = Arena::new();
        let tpa = Arena::new();

        let mut builder = Builder::new(&sya, &tpa, false);
        builder.process(ctx, self, styles)?;
        builder.finish(ctx, styles)?;

        let (flow, shared) = builder.flow.finish();
        FlowNode(flow).layout(ctx, regions, shared)
    }

    fn pack(self) -> LayoutNode {
        match self {
            Content::Block(node) => node,
            other => LayoutNode::new(other),
        }
    }
}

/// Builds a flow or page nodes from content.
struct Builder<'a> {
    /// An arena where intermediate style chains are stored.
    sya: &'a Arena<StyleChain<'a>>,
    /// An arena where intermediate content resulting from show rules is stored.
    tpa: &'a Arena<Content>,
    /// The already built page runs.
    pages: Option<StyleVecBuilder<'a, PageNode>>,
    /// The currently built list.
    list: Option<ListBuilder<'a>>,
    /// The currently built flow.
    flow: CollapsingBuilder<'a, FlowChild>,
    /// The currently built paragraph.
    par: CollapsingBuilder<'a, ParChild>,
    /// Whether to keep the next page even if it is empty.
    keep_next: bool,
}

impl<'a> Builder<'a> {
    /// Prepare the builder.
    fn new(sya: &'a Arena<StyleChain<'a>>, tpa: &'a Arena<Content>, top: bool) -> Self {
        Self {
            sya,
            tpa,
            pages: top.then(|| StyleVecBuilder::new()),
            flow: CollapsingBuilder::new(),
            list: None,
            par: CollapsingBuilder::new(),
            keep_next: true,
        }
    }

    /// Process content.
    fn process(
        &mut self,
        ctx: &mut Context,
        content: &'a Content,
        styles: StyleChain<'a>,
    ) -> TypResult<()> {
        if let Some(builder) = &mut self.list {
            match content {
                Content::Space => {
                    builder.staged.push((content, styles));
                    return Ok(());
                }
                Content::Parbreak => {
                    builder.staged.push((content, styles));
                    return Ok(());
                }
                Content::List(item) if builder.kind == UNORDERED => {
                    builder.wide |=
                        builder.staged.iter().any(|&(t, _)| *t == Content::Parbreak);
                    builder.staged.clear();
                    builder.items.push(item.clone());
                    return Ok(());
                }
                Content::Enum(item) if builder.kind == ORDERED => {
                    builder.wide |=
                        builder.staged.iter().any(|&(t, _)| *t == Content::Parbreak);
                    builder.staged.clear();
                    builder.items.push(item.clone());
                    return Ok(());
                }
                _ => self.finish_list(ctx)?,
            }
        }

        match content {
            Content::Space => {
                self.par.weak(ParChild::Text(' '.into()), 0, styles);
            }
            Content::Linebreak => {
                self.par.destructive(ParChild::Text('\n'.into()), styles);
            }
            Content::Horizontal(kind) => {
                let child = ParChild::Spacing(*kind);
                if kind.is_fractional() {
                    self.par.destructive(child, styles);
                } else {
                    self.par.ignorant(child, styles);
                }
            }
            Content::Text(text) => {
                self.par.supportive(ParChild::Text(text.clone()), styles);
            }
            Content::Inline(node) => {
                self.par.supportive(ParChild::Node(node.clone()), styles);
            }
            Content::Parbreak => {
                self.finish_par(styles);
                self.flow.weak(FlowChild::Parbreak, 1, styles);
            }
            Content::Colbreak => {
                self.finish_par(styles);
                self.flow.destructive(FlowChild::Colbreak, styles);
            }
            Content::Vertical(kind) => {
                self.finish_par(styles);
                let child = FlowChild::Spacing(*kind);
                if kind.is_fractional() {
                    self.flow.destructive(child, styles);
                } else {
                    self.flow.ignorant(child, styles);
                }
            }
            Content::Block(node) => {
                self.finish_par(styles);
                let child = FlowChild::Node(node.clone());
                if node.is::<PlaceNode>() {
                    self.flow.ignorant(child, styles);
                } else {
                    self.flow.supportive(child, styles);
                }
                self.finish_par(styles);
            }
            Content::List(item) => {
                self.list = Some(ListBuilder {
                    styles,
                    kind: UNORDERED,
                    items: vec![item.clone()],
                    wide: false,
                    staged: vec![],
                });
            }
            Content::Enum(item) => {
                self.list = Some(ListBuilder {
                    styles,
                    kind: ORDERED,
                    items: vec![item.clone()],
                    wide: false,
                    staged: vec![],
                });
            }
            Content::Pagebreak => {
                self.finish_page(ctx, true, true, styles)?;
            }
            Content::Page(page) => {
                self.finish_page(ctx, false, false, styles)?;
                if let Some(pages) = &mut self.pages {
                    pages.push(page.clone(), styles);
                }
            }
            Content::Show(node) => {
                let id = node.id();
                let content = node.show(ctx, styles)?;
                let stored = self.tpa.alloc(content);
                self.process(ctx, stored, styles.unscoped(id))?;
            }
            Content::Styled(styled) => {
                let (sub, map) = styled.as_ref();
                let stored = self.sya.alloc(styles);
                let styles = map.chain(stored);

                let interruption = map.interruption();
                match interruption {
                    Some(Interruption::Page) => {
                        self.finish_page(ctx, false, true, styles)?
                    }
                    Some(Interruption::Par) => self.finish_par(styles),
                    None => {}
                }

                self.process(ctx, sub, styles)?;

                match interruption {
                    Some(Interruption::Page) => {
                        self.finish_page(ctx, true, false, styles)?
                    }
                    Some(Interruption::Par) => self.finish_par(styles),
                    None => {}
                }
            }
            Content::Sequence(seq) => {
                for sub in seq.iter() {
                    self.process(ctx, sub, styles)?;
                }
            }
        }

        Ok(())
    }

    /// Finish the currently built paragraph.
    fn finish_par(&mut self, styles: StyleChain<'a>) {
        let (mut par, shared) = std::mem::take(&mut self.par).finish();
        if !par.is_empty() {
            // Paragraph indent should only apply if the paragraph starts with
            // text and follows directly after another paragraph.
            let indent = shared.get(ParNode::INDENT);
            if !indent.is_zero()
                && par
                    .items()
                    .find_map(|child| match child {
                        ParChild::Spacing(_) => None,
                        ParChild::Text(_) => Some(true),
                        ParChild::Node(_) => Some(false),
                    })
                    .unwrap_or_default()
                && self
                    .flow
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
                par.push_front(ParChild::Spacing(indent.into()));
            }

            let node = ParNode(par).pack();
            self.flow.supportive(FlowChild::Node(node), shared);
        }
        self.flow.weak(FlowChild::Leading, 0, styles);
    }

    /// Finish the currently built list.
    fn finish_list(&mut self, ctx: &mut Context) -> TypResult<()> {
        let ListBuilder { styles, kind, items, wide, staged } = match self.list.take() {
            Some(list) => list,
            None => return Ok(()),
        };

        let content = match kind {
            UNORDERED => Content::show(ListNode::<UNORDERED> { start: 1, wide, items }),
            ORDERED | _ => Content::show(ListNode::<ORDERED> { start: 1, wide, items }),
        };

        let stored = self.tpa.alloc(content);
        self.process(ctx, stored, styles)?;
        for (content, styles) in staged {
            self.process(ctx, content, styles)?;
        }

        Ok(())
    }

    /// Finish the currently built page run.
    fn finish_page(
        &mut self,
        ctx: &mut Context,
        keep_last: bool,
        keep_next: bool,
        styles: StyleChain<'a>,
    ) -> TypResult<()> {
        self.finish_list(ctx)?;
        self.finish_par(styles);
        if let Some(pages) = &mut self.pages {
            let (flow, shared) = std::mem::take(&mut self.flow).finish();
            if !flow.is_empty() || (keep_last && self.keep_next) {
                let styles = if flow.is_empty() { styles } else { shared };
                let node = PageNode(FlowNode(flow).pack());
                pages.push(node, styles);
            }
        }
        self.keep_next = keep_next;
        Ok(())
    }

    /// Finish everything.
    fn finish(&mut self, ctx: &mut Context, styles: StyleChain<'a>) -> TypResult<()> {
        self.finish_page(ctx, true, false, styles)
    }
}

/// Builds an unordered or ordered list from items.
struct ListBuilder<'a> {
    styles: StyleChain<'a>,
    kind: ListKind,
    items: Vec<ListItem>,
    wide: bool,
    staged: Vec<(&'a Content, StyleChain<'a>)>,
}

impl Debug for Content {
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
            Self::List(item) => {
                f.write_str("- ")?;
                item.body.fmt(f)
            }
            Self::Enum(item) => {
                if let Some(number) = item.number {
                    write!(f, "{}", number)?;
                }
                f.write_str(". ")?;
                item.body.fmt(f)
            }
            Self::Pagebreak => f.pad("Pagebreak"),
            Self::Page(page) => page.fmt(f),
            Self::Show(node) => {
                f.write_str("Show(")?;
                node.fmt(f)?;
                f.write_str(")")
            }
            Self::Styled(styled) => {
                let (sub, map) = styled.as_ref();
                map.fmt(f)?;
                sub.fmt(f)
            }
            Self::Sequence(seq) => f.debug_list().entries(seq.iter()).finish(),
        }
    }
}
