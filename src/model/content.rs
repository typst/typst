use std::fmt::{self, Debug, Formatter};
use std::hash::Hash;
use std::iter::Sum;
use std::mem;
use std::ops::{Add, AddAssign};
use std::sync::Arc;

use comemo::Tracked;
use typed_arena::Arena;

use super::{
    Barrier, CollapsingBuilder, Dict, Interruption, Key, Layout, LayoutNode, Property,
    Regions, Selector, Show, ShowNode, StyleChain, StyleEntry, StyleMap, StyleVecBuilder,
    Target,
};
use crate::diag::{SourceResult, StrResult};
use crate::frame::{Frame, Role};
use crate::geom::{Length, Numeric};
use crate::library::layout::{FlowChild, FlowNode, PageNode, PlaceNode, Spacing};
use crate::library::structure::{DocNode, ListItem, ListNode, DESC, ENUM, LIST};
use crate::library::text::{ParChild, ParNode};
use crate::util::EcoString;
use crate::World;

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
    /// Empty content.
    Empty,
    /// A word space.
    Space,
    /// A forced line break.
    Linebreak { justify: bool },
    /// Horizontal spacing.
    Horizontal { amount: Spacing, weak: bool },
    /// Plain text.
    Text(EcoString),
    /// A smart quote.
    Quote { double: bool },
    /// An inline-level node.
    Inline(LayoutNode),
    /// A paragraph break.
    Parbreak,
    /// A column break.
    Colbreak { weak: bool },
    /// Vertical spacing.
    Vertical {
        amount: Spacing,
        weak: bool,
        generated: bool,
    },
    /// A block-level node.
    Block(LayoutNode),
    /// A list / enum item.
    Item(ListItem),
    /// A page break.
    Pagebreak { weak: bool },
    /// A page node.
    Page(PageNode),
    /// A node that can be realized with styles, optionally with attached
    /// properties.
    Show(ShowNode, Option<Dict>),
    /// Content with attached styles.
    Styled(Arc<(Self, StyleMap)>),
    /// A sequence of multiple nodes.
    Sequence(Arc<Vec<Self>>),
}

impl Content {
    /// Create empty content.
    pub fn new() -> Self {
        Self::Empty
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
        Self::Show(node.pack(), None)
    }

    /// Create a new sequence node from multiples nodes.
    pub fn sequence(seq: Vec<Self>) -> Self {
        match seq.as_slice() {
            [] => Self::Empty,
            [_] => seq.into_iter().next().unwrap(),
            _ => Self::Sequence(Arc::new(seq)),
        }
    }

    /// Repeat this content `n` times.
    pub fn repeat(&self, n: i64) -> StrResult<Self> {
        let count = usize::try_from(n)
            .map_err(|_| format!("cannot repeat this content {} times", n))?;

        Ok(Self::sequence(vec![self.clone(); count]))
    }

    /// Style this content with a single style property.
    pub fn styled<'k, K: Key<'k>>(self, key: K, value: K::Value) -> Self {
        self.styled_with_entry(StyleEntry::Property(Property::new(key, value)))
    }

    /// Style this content with a style entry.
    pub fn styled_with_entry(mut self, entry: StyleEntry) -> Self {
        if let Self::Styled(styled) = &mut self {
            if let Some((_, map)) = Arc::get_mut(styled) {
                map.apply(entry);
                return self;
            }
        }

        Self::Styled(Arc::new((self, entry.into())))
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

    /// Assign a semantic role to this content.
    pub fn role(self, role: Role) -> Self {
        self.styled_with_entry(StyleEntry::Role(role))
    }

    /// Reenable the show rule identified by the selector.
    pub fn unguard(&self, sel: Selector) -> Self {
        self.clone().styled_with_entry(StyleEntry::Unguard(sel))
    }

    /// Add weak vertical spacing above and below the node.
    pub fn spaced(self, above: Option<Length>, below: Option<Length>) -> Self {
        if above.is_none() && below.is_none() {
            return self;
        }

        let mut seq = vec![];
        if let Some(above) = above {
            seq.push(Content::Vertical {
                amount: above.into(),
                weak: true,
                generated: true,
            });
        }

        seq.push(self);
        if let Some(below) = below {
            seq.push(Content::Vertical {
                amount: below.into(),
                weak: true,
                generated: true,
            });
        }

        Self::sequence(seq)
    }
}

impl Layout for Content {
    fn layout(
        &self,
        world: Tracked<dyn World>,
        regions: &Regions,
        styles: StyleChain,
    ) -> SourceResult<Vec<Frame>> {
        let scratch = Scratch::default();
        let mut builder = Builder::new(world, &scratch, false);
        builder.accept(self, styles)?;
        let (flow, shared) = builder.into_flow(styles)?;
        flow.layout(world, regions, shared)
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
            Self::Empty => f.pad("Empty"),
            Self::Space => f.pad("Space"),
            Self::Linebreak { justify } => write!(f, "Linebreak({justify})"),
            Self::Horizontal { amount, weak } => {
                write!(f, "Horizontal({amount:?}, {weak})")
            }
            Self::Text(text) => write!(f, "Text({text:?})"),
            Self::Quote { double } => write!(f, "Quote({double})"),
            Self::Inline(node) => node.fmt(f),
            Self::Parbreak => f.pad("Parbreak"),
            Self::Colbreak { weak } => write!(f, "Colbreak({weak})"),
            Self::Vertical { amount, weak, generated } => {
                write!(f, "Vertical({amount:?}, {weak}, {generated})")
            }
            Self::Block(node) => node.fmt(f),
            Self::Item(item) => item.fmt(f),
            Self::Pagebreak { weak } => write!(f, "Pagebreak({weak})"),
            Self::Page(page) => page.fmt(f),
            Self::Show(node, _) => node.fmt(f),
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
            (Self::Empty, rhs) => return rhs,
            (lhs, Self::Empty) => return lhs,
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
pub(super) struct Builder<'a> {
    /// The core context.
    world: Tracked<'a, dyn World>,
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
pub(super) struct Scratch<'a> {
    /// An arena where intermediate style chains are stored.
    styles: Arena<StyleChain<'a>>,
    /// An arena where intermediate content resulting from show rules is stored.
    templates: Arena<Content>,
}

impl<'a> Builder<'a> {
    pub fn new(
        world: Tracked<'a, dyn World>,
        scratch: &'a Scratch<'a>,
        top: bool,
    ) -> Self {
        Self {
            world,
            scratch,
            doc: top.then(|| DocBuilder::default()),
            flow: FlowBuilder::default(),
            par: ParBuilder::default(),
            list: ListBuilder::default(),
        }
    }

    pub fn into_doc(
        mut self,
        styles: StyleChain<'a>,
    ) -> SourceResult<(DocNode, StyleChain<'a>)> {
        self.interrupt(Interruption::Page, styles, true)?;
        let (pages, shared) = self.doc.unwrap().pages.finish();
        Ok((DocNode(pages), shared))
    }

    pub fn into_flow(
        mut self,
        styles: StyleChain<'a>,
    ) -> SourceResult<(FlowNode, StyleChain<'a>)> {
        self.interrupt(Interruption::Par, styles, false)?;
        let (children, shared) = self.flow.0.finish();
        Ok((FlowNode(children), shared))
    }

    pub fn accept(
        &mut self,
        content: &'a Content,
        styles: StyleChain<'a>,
    ) -> SourceResult<()> {
        match content {
            Content::Empty => return Ok(()),
            Content::Text(text) => {
                if let Some(realized) = styles.apply(self.world, Target::Text(text))? {
                    let stored = self.scratch.templates.alloc(realized);
                    return self.accept(stored, styles);
                }
            }

            Content::Show(node, _) => return self.show(node, styles),
            Content::Styled(styled) => return self.styled(styled, styles),
            Content::Sequence(seq) => return self.sequence(seq, styles),

            _ => {}
        }

        if self.list.accept(content, styles) {
            return Ok(());
        }

        self.interrupt(Interruption::List, styles, false)?;

        if let Content::Item(_) = content {
            self.list.accept(content, styles);
            return Ok(());
        }

        if self.par.accept(content, styles) {
            return Ok(());
        }

        self.interrupt(Interruption::Par, styles, false)?;

        if self.flow.accept(content, styles) {
            return Ok(());
        }

        let keep = matches!(content, Content::Pagebreak { weak: false });
        self.interrupt(Interruption::Page, styles, keep)?;

        if let Some(doc) = &mut self.doc {
            doc.accept(content, styles);
        }

        // We might want to issue a warning or error for content that wasn't
        // handled (e.g. a pagebreak in a flow building process). However, we
        // don't have the spans here at the moment.
        Ok(())
    }

    fn show(&mut self, node: &ShowNode, styles: StyleChain<'a>) -> SourceResult<()> {
        if let Some(mut realized) = styles.apply(self.world, Target::Node(node))? {
            let mut map = StyleMap::new();
            let barrier = Barrier::new(node.id());
            map.push(StyleEntry::Barrier(barrier));
            map.push(StyleEntry::Barrier(barrier));
            realized = realized.styled_with_map(map);
            let stored = self.scratch.templates.alloc(realized);
            self.accept(stored, styles)?;
        }
        Ok(())
    }

    fn styled(
        &mut self,
        (content, map): &'a (Content, StyleMap),
        styles: StyleChain<'a>,
    ) -> SourceResult<()> {
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
    ) -> SourceResult<()> {
        if intr >= Interruption::List && !self.list.is_empty() {
            mem::take(&mut self.list).finish(self)?;
        }

        if intr >= Interruption::Par {
            if !self.par.is_empty() {
                mem::take(&mut self.par).finish(self);
            }
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

    fn sequence(
        &mut self,
        seq: &'a [Content],
        styles: StyleChain<'a>,
    ) -> SourceResult<()> {
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
            Content::Pagebreak { weak } => {
                self.keep_next = !weak;
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
        // Weak flow elements:
        // Weakness | Element
        //    0     | weak colbreak
        //    1     | weak fractional spacing
        //    2     | weak spacing
        //    3     | generated weak spacing
        //    4     | generated weak fractional spacing
        //    5     | par spacing

        match content {
            Content::Parbreak => {}
            Content::Colbreak { weak } => {
                if *weak {
                    self.0.weak(FlowChild::Colbreak, styles, 0);
                } else {
                    self.0.destructive(FlowChild::Colbreak, styles);
                }
            }
            &Content::Vertical { amount, weak, generated } => {
                let child = FlowChild::Spacing(amount);
                let frac = amount.is_fractional();
                if weak {
                    let weakness = 1 + u8::from(frac) + 2 * u8::from(generated);
                    self.0.weak(child, styles, weakness);
                } else if frac {
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

    fn par(&mut self, par: ParNode, styles: StyleChain<'a>, indent: bool) {
        let amount = if indent && !styles.get(ParNode::SPACING_AND_INDENT) {
            styles.get(ParNode::LEADING).into()
        } else {
            styles.get(ParNode::SPACING).into()
        };

        self.0.weak(FlowChild::Spacing(amount), styles, 5);
        self.0.supportive(FlowChild::Node(par.pack()), styles);
        self.0.weak(FlowChild::Spacing(amount), styles, 5);
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
        // Weak par elements:
        // Weakness | Element
        //    0     | weak fractional spacing
        //    1     | weak spacing
        //    2     | space

        match content {
            Content::Space => {
                self.0.weak(ParChild::Text(' '.into()), styles, 2);
            }
            &Content::Linebreak { justify } => {
                let c = if justify { '\u{2028}' } else { '\n' };
                self.0.destructive(ParChild::Text(c.into()), styles);
            }
            &Content::Horizontal { amount, weak } => {
                let child = ParChild::Spacing(amount);
                let frac = amount.is_fractional();
                if weak {
                    let weakness = u8::from(!frac);
                    self.0.weak(child, styles, weakness);
                } else if frac {
                    self.0.destructive(child, styles);
                } else {
                    self.0.ignorant(child, styles);
                }
            }
            &Content::Quote { double } => {
                self.0.supportive(ParChild::Quote { double }, styles);
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

    fn finish(self, parent: &mut Builder<'a>) {
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
                    ParChild::Text(_) | ParChild::Quote { .. } => Some(true),
                    ParChild::Node(_) => Some(false),
                })
                .unwrap_or_default()
            && parent
                .flow
                .0
                .items()
                .rev()
                .find_map(|child| match child {
                    FlowChild::Spacing(_) => None,
                    FlowChild::Node(node) => Some(node.is::<ParNode>()),
                    FlowChild::Colbreak => Some(false),
                })
                .unwrap_or_default()
        {
            children.push_front(ParChild::Spacing(indent.into()));
        }

        parent.flow.par(ParNode(children), shared, !indent.is_zero());
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
    /// Whether the list can be attached.
    attachable: bool,
    /// Trailing content for which it is unclear whether it is part of the list.
    staged: Vec<(&'a Content, StyleChain<'a>)>,
}

impl<'a> ListBuilder<'a> {
    fn accept(&mut self, content: &'a Content, styles: StyleChain<'a>) -> bool {
        if self.items.is_empty() {
            match content {
                Content::Space => {}
                Content::Item(_) => {}
                Content::Parbreak => self.attachable = false,
                _ => self.attachable = true,
            }
        }

        match content {
            Content::Item(item)
                if self
                    .items
                    .items()
                    .next()
                    .map_or(true, |first| item.kind() == first.kind()) =>
            {
                self.items.push(item.clone(), styles);
                self.tight &= self.staged.drain(..).all(|(t, _)| *t != Content::Parbreak);
            }
            Content::Space | Content::Parbreak if !self.items.is_empty() => {
                self.staged.push((content, styles));
            }
            _ => return false,
        }

        true
    }

    fn finish(self, parent: &mut Builder<'a>) -> SourceResult<()> {
        let (items, shared) = self.items.finish();
        let kind = match items.items().next() {
            Some(item) => item.kind(),
            None => return Ok(()),
        };

        let tight = self.tight;
        let attached = tight && self.attachable;
        let content = match kind {
            LIST => Content::show(ListNode::<LIST> { tight, attached, items }),
            ENUM => Content::show(ListNode::<ENUM> { tight, attached, items }),
            DESC | _ => Content::show(ListNode::<DESC> { tight, attached, items }),
        };

        let stored = parent.scratch.templates.alloc(content);
        parent.accept(stored, shared)?;

        for (content, styles) in self.staged {
            parent.accept(content, styles)?;
        }

        parent.list.attachable = true;

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
            attachable: true,
            staged: vec![],
        }
    }
}
