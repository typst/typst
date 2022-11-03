//! Composable layouts.

mod align;
mod columns;
mod container;
mod flow;
mod grid;
mod pad;
mod page;
mod place;
mod spacing;
mod stack;
mod transform;

pub use align::*;
pub use columns::*;
pub use container::*;
pub use flow::*;
pub use grid::*;
pub use pad::*;
pub use page::*;
pub use place::*;
pub use spacing::*;
pub use stack::*;
pub use transform::*;

use std::mem;

use comemo::Tracked;
use typed_arena::Arena;
use typst::diag::SourceResult;
use typst::frame::Frame;
use typst::geom::*;
use typst::model::{
    capability, Barrier, Content, Node, SequenceNode, Show, StyleChain, StyleEntry,
    StyleMap, StyleVec, StyleVecBuilder, StyledNode, Target,
};
use typst::World;

use crate::structure::{
    DescNode, DocNode, EnumNode, ListItem, ListNode, DESC, ENUM, LIST,
};
use crate::text::{
    LinebreakNode, ParChild, ParNode, ParbreakNode, SmartQuoteNode, SpaceNode, TextNode,
};

/// Root-level layout.
#[capability]
pub trait LayoutRoot: 'static + Sync + Send {
    /// Layout into one frame per page.
    fn layout_root(&self, world: Tracked<dyn World>) -> SourceResult<Vec<Frame>>;
}

impl LayoutRoot for Content {
    #[comemo::memoize]
    fn layout_root(&self, world: Tracked<dyn World>) -> SourceResult<Vec<Frame>> {
        let styles = StyleChain::with_root(&world.config().styles);
        let scratch = Scratch::default();

        let mut builder = Builder::new(world, &scratch, true);
        builder.accept(self, styles)?;

        let (doc, shared) = builder.into_doc(styles)?;
        doc.layout(world, shared)
    }
}

/// Block-level layout.
#[capability]
pub trait LayoutBlock: 'static + Sync + Send {
    /// Layout into one frame per region.
    fn layout_block(
        &self,
        world: Tracked<dyn World>,
        regions: &Regions,
        styles: StyleChain,
    ) -> SourceResult<Vec<Frame>>;
}

impl LayoutBlock for Content {
    #[comemo::memoize]
    fn layout_block(
        &self,
        world: Tracked<dyn World>,
        regions: &Regions,
        styles: StyleChain,
    ) -> SourceResult<Vec<Frame>> {
        if let Some(node) = self.to::<dyn LayoutBlock>() {
            let barrier = StyleEntry::Barrier(Barrier::new(self.id()));
            let styles = barrier.chain(&styles);
            return node.layout_block(world, regions, styles);
        }

        let scratch = Scratch::default();
        let mut builder = Builder::new(world, &scratch, false);
        builder.accept(self, styles)?;
        let (flow, shared) = builder.into_flow(styles)?;
        flow.layout_block(world, regions, shared)
    }
}

/// Inline-level layout.
#[capability]
pub trait LayoutInline: 'static + Sync + Send {
    /// Layout into a single frame.
    fn layout_inline(
        &self,
        world: Tracked<dyn World>,
        regions: &Regions,
        styles: StyleChain,
    ) -> SourceResult<Vec<Frame>>;
}

impl LayoutInline for Content {
    #[comemo::memoize]
    fn layout_inline(
        &self,
        world: Tracked<dyn World>,
        regions: &Regions,
        styles: StyleChain,
    ) -> SourceResult<Vec<Frame>> {
        if let Some(node) = self.to::<dyn LayoutInline>() {
            let barrier = StyleEntry::Barrier(Barrier::new(self.id()));
            let styles = barrier.chain(&styles);
            return node.layout_inline(world, regions, styles);
        }

        if let Some(node) = self.to::<dyn LayoutBlock>() {
            let barrier = StyleEntry::Barrier(Barrier::new(self.id()));
            let styles = barrier.chain(&styles);
            return node.layout_block(world, regions, styles);
        }

        let scratch = Scratch::default();
        let mut builder = Builder::new(world, &scratch, false);
        builder.accept(self, styles)?;
        let (flow, shared) = builder.into_flow(styles)?;
        flow.layout_block(world, regions, shared)
    }
}

/// A sequence of regions to layout into.
#[derive(Debug, Clone, Hash)]
pub struct Regions {
    /// The (remaining) size of the first region.
    pub first: Size,
    /// The base size for relative sizing.
    pub base: Size,
    /// The height of followup regions. The width is the same for all regions.
    pub backlog: Vec<Abs>,
    /// The height of the final region that is repeated once the backlog is
    /// drained. The width is the same for all regions.
    pub last: Option<Abs>,
    /// Whether nodes should expand to fill the regions instead of shrinking to
    /// fit the content.
    pub expand: Axes<bool>,
}

impl Regions {
    /// Create a new region sequence with exactly one region.
    pub fn one(size: Size, base: Size, expand: Axes<bool>) -> Self {
        Self {
            first: size,
            base,
            backlog: vec![],
            last: None,
            expand,
        }
    }

    /// Create a new sequence of same-size regions that repeats indefinitely.
    pub fn repeat(size: Size, base: Size, expand: Axes<bool>) -> Self {
        Self {
            first: size,
            base,
            backlog: vec![],
            last: Some(size.y),
            expand,
        }
    }

    /// Create new regions where all sizes are mapped with `f`.
    ///
    /// Note that since all regions must have the same width, the width returned
    /// by `f` is ignored for the backlog and the final region.
    pub fn map<F>(&self, mut f: F) -> Self
    where
        F: FnMut(Size) -> Size,
    {
        let x = self.first.x;
        Self {
            first: f(self.first),
            base: f(self.base),
            backlog: self.backlog.iter().map(|&y| f(Size::new(x, y)).y).collect(),
            last: self.last.map(|y| f(Size::new(x, y)).y),
            expand: self.expand,
        }
    }

    /// Whether the first region is full and a region break is called for.
    pub fn is_full(&self) -> bool {
        Abs::zero().fits(self.first.y) && !self.in_last()
    }

    /// Whether the first region is the last usable region.
    ///
    /// If this is true, calling `next()` will have no effect.
    pub fn in_last(&self) -> bool {
        self.backlog.is_empty() && self.last.map_or(true, |height| self.first.y == height)
    }

    /// Advance to the next region if there is any.
    pub fn next(&mut self) {
        if let Some(height) = (!self.backlog.is_empty())
            .then(|| self.backlog.remove(0))
            .or(self.last)
        {
            self.first.y = height;
            self.base.y = height;
        }
    }

    /// An iterator that returns the sizes of the first and all following
    /// regions, equivalently to what would be produced by calling
    /// [`next()`](Self::next) repeatedly until all regions are exhausted.
    /// This iterater may be infinite.
    pub fn iter(&self) -> impl Iterator<Item = Size> + '_ {
        let first = std::iter::once(self.first);
        let backlog = self.backlog.iter();
        let last = self.last.iter().cycle();
        first.chain(backlog.chain(last).map(|&h| Size::new(self.first.x, h)))
    }
}

/// Builds a document or a flow node from content.
struct Builder<'a> {
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
struct Scratch<'a> {
    /// An arena where intermediate style chains are stored.
    styles: Arena<StyleChain<'a>>,
    /// An arena where intermediate content resulting from show rules is stored.
    templates: Arena<Content>,
}

/// Determines whether a style could interrupt some composable structure.
#[derive(Debug, Copy, Clone, Eq, PartialEq, Ord, PartialOrd)]
enum Interruption {
    /// The style forces a list break.
    List,
    /// The style forces a paragraph break.
    Par,
    /// The style forces a page break.
    Page,
}

impl<'a> Builder<'a> {
    fn new(world: Tracked<'a, dyn World>, scratch: &'a Scratch<'a>, top: bool) -> Self {
        Self {
            world,
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
    ) -> SourceResult<(DocNode, StyleChain<'a>)> {
        self.interrupt(Interruption::Page, styles, true)?;
        let (pages, shared) = self.doc.unwrap().pages.finish();
        Ok((DocNode(pages), shared))
    }

    fn into_flow(
        mut self,
        styles: StyleChain<'a>,
    ) -> SourceResult<(FlowNode, StyleChain<'a>)> {
        self.interrupt(Interruption::Par, styles, false)?;
        let (children, shared) = self.flow.0.finish();
        Ok((FlowNode(children), shared))
    }

    fn accept(
        &mut self,
        content: &'a Content,
        styles: StyleChain<'a>,
    ) -> SourceResult<()> {
        if let Some(text) = content.downcast::<TextNode>() {
            if let Some(realized) = styles.apply(self.world, Target::Text(&text.0))? {
                let stored = self.scratch.templates.alloc(realized);
                return self.accept(stored, styles);
            }
        } else if let Some(styled) = content.downcast::<StyledNode>() {
            return self.styled(styled, styles);
        } else if let Some(seq) = content.downcast::<SequenceNode>() {
            return self.sequence(seq, styles);
        } else if content.has::<dyn Show>() && self.show(content, styles)? {
            return Ok(());
        }

        if self.list.accept(content, styles) {
            return Ok(());
        }

        self.interrupt(Interruption::List, styles, false)?;

        if content.is::<ListItem>() {
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

        let keep = content
            .downcast::<PagebreakNode>()
            .map_or(false, |pagebreak| !pagebreak.weak);
        self.interrupt(Interruption::Page, styles, keep)?;

        if let Some(doc) = &mut self.doc {
            doc.accept(content, styles);
        }

        // We might want to issue a warning or error for content that wasn't
        // handled (e.g. a pagebreak in a flow building process). However, we
        // don't have the spans here at the moment.
        Ok(())
    }

    fn show(
        &mut self,
        content: &'a Content,
        styles: StyleChain<'a>,
    ) -> SourceResult<bool> {
        if let Some(mut realized) = styles.apply(self.world, Target::Node(content))? {
            let mut map = StyleMap::new();
            let barrier = Barrier::new(content.id());
            map.push(StyleEntry::Barrier(barrier));
            map.push(StyleEntry::Barrier(barrier));
            realized = realized.styled_with_map(map);
            let stored = self.scratch.templates.alloc(realized);
            self.accept(stored, styles)?;
            Ok(true)
        } else {
            Ok(false)
        }
    }

    fn styled(
        &mut self,
        styled: &'a StyledNode,
        styles: StyleChain<'a>,
    ) -> SourceResult<()> {
        let stored = self.scratch.styles.alloc(styles);
        let styles = styled.map.chain(stored);

        let intr = if styled.map.interrupts::<PageNode>() {
            Some(Interruption::Page)
        } else if styled.map.interrupts::<ParNode>() {
            Some(Interruption::Par)
        } else if styled.map.interrupts::<ListNode>()
            || styled.map.interrupts::<EnumNode>()
            || styled.map.interrupts::<DescNode>()
        {
            Some(Interruption::List)
        } else {
            None
        };

        if let Some(intr) = intr {
            self.interrupt(intr, styles, false)?;
        }

        self.accept(&styled.sub, styles)?;

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

        if intr >= Interruption::Par && !self.par.is_empty() {
            mem::take(&mut self.par).finish(self);
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
        seq: &'a SequenceNode,
        styles: StyleChain<'a>,
    ) -> SourceResult<()> {
        for content in &seq.0 {
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
        if let Some(pagebreak) = content.downcast::<PagebreakNode>() {
            self.keep_next = !pagebreak.weak;
        }

        if let Some(page) = content.downcast::<PageNode>() {
            self.pages.push(page.clone(), styles);
            self.keep_next = false;
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

        if content.is::<ParbreakNode>() {
            /* Nothing to do */
        } else if let Some(colbreak) = content.downcast::<ColbreakNode>() {
            if colbreak.weak {
                self.0.weak(FlowChild::Colbreak, styles, 0);
            } else {
                self.0.destructive(FlowChild::Colbreak, styles);
            }
        } else if let Some(vertical) = content.downcast::<VNode>() {
            let child = FlowChild::Spacing(vertical.amount);
            let frac = vertical.amount.is_fractional();
            if vertical.weak {
                let weakness = 1 + u8::from(frac) + 2 * u8::from(vertical.generated);
                self.0.weak(child, styles, weakness);
            } else if frac {
                self.0.destructive(child, styles);
            } else {
                self.0.ignorant(child, styles);
            }
        } else if content.has::<dyn LayoutBlock>() {
            let child = FlowChild::Block(content.clone());
            if content.is::<PlaceNode>() {
                self.0.ignorant(child, styles);
            } else {
                self.0.supportive(child, styles);
            }
        } else {
            return false;
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
        self.0.supportive(FlowChild::Block(par.pack()), styles);
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

        if content.is::<SpaceNode>() {
            self.0.weak(ParChild::Text(' '.into()), styles, 2);
        } else if let Some(linebreak) = content.downcast::<LinebreakNode>() {
            let c = if linebreak.justify { '\u{2028}' } else { '\n' };
            self.0.destructive(ParChild::Text(c.into()), styles);
        } else if let Some(horizontal) = content.downcast::<HNode>() {
            let child = ParChild::Spacing(horizontal.amount);
            let frac = horizontal.amount.is_fractional();
            if horizontal.weak {
                let weakness = u8::from(!frac);
                self.0.weak(child, styles, weakness);
            } else if frac {
                self.0.destructive(child, styles);
            } else {
                self.0.ignorant(child, styles);
            }
        } else if let Some(quote) = content.downcast::<SmartQuoteNode>() {
            self.0.supportive(ParChild::Quote { double: quote.double }, styles);
        } else if let Some(text) = content.downcast::<TextNode>() {
            self.0.supportive(ParChild::Text(text.0.clone()), styles);
        } else if content.has::<dyn LayoutInline>() {
            self.0.supportive(ParChild::Inline(content.clone()), styles);
        } else {
            return false;
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
                    ParChild::Inline(_) => Some(false),
                })
                .unwrap_or_default()
            && parent
                .flow
                .0
                .items()
                .rev()
                .find_map(|child| match child {
                    FlowChild::Spacing(_) => None,
                    FlowChild::Block(content) => Some(content.is::<ParNode>()),
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
            if content.is::<ParbreakNode>() {
                self.attachable = false;
            } else if !content.is::<SpaceNode>() && !content.is::<ListItem>() {
                self.attachable = true;
            }
        }

        if let Some(item) = content.downcast::<ListItem>() {
            if self
                .items
                .items()
                .next()
                .map_or(true, |first| item.kind() == first.kind())
            {
                self.items.push(item.clone(), styles);
                self.tight &= self.staged.drain(..).all(|(t, _)| !t.is::<ParbreakNode>());
            } else {
                return false;
            }
        } else if !self.items.is_empty()
            && (content.is::<SpaceNode>() || content.is::<ParbreakNode>())
        {
            self.staged.push((content, styles));
        } else {
            return false;
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
            LIST => ListNode::<LIST> { tight, attached, items }.pack(),
            ENUM => ListNode::<ENUM> { tight, attached, items }.pack(),
            DESC | _ => ListNode::<DESC> { tight, attached, items }.pack(),
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

/// A wrapper around a [`StyleVecBuilder`] that allows to collapse items.
struct CollapsingBuilder<'a, T> {
    /// The internal builder.
    builder: StyleVecBuilder<'a, T>,
    /// Staged weak and ignorant items that we can't yet commit to the builder.
    /// The option is `Some(_)` for weak items and `None` for ignorant items.
    staged: Vec<(T, StyleChain<'a>, Option<u8>)>,
    /// What the last non-ignorant item was.
    last: Last,
}

/// What the last non-ignorant item was.
#[derive(Debug, Copy, Clone, Eq, PartialEq)]
enum Last {
    Weak,
    Destructive,
    Supportive,
}

impl<'a, T> CollapsingBuilder<'a, T> {
    /// Create a new style-vec builder.
    fn new() -> Self {
        Self {
            builder: StyleVecBuilder::new(),
            staged: vec![],
            last: Last::Destructive,
        }
    }

    /// Whether the builder is empty.
    fn is_empty(&self) -> bool {
        self.builder.is_empty() && self.staged.is_empty()
    }

    /// Can only exist when there is at least one supportive item to its left
    /// and to its right, with no destructive items in between. There may be
    /// ignorant items in between in both directions.
    ///
    /// Between weak items, there may be at least one per layer and among the
    /// candidates the strongest one (smallest `weakness`) wins. When tied,
    /// the one that compares larger through `PartialOrd` wins.
    fn weak(&mut self, item: T, styles: StyleChain<'a>, weakness: u8)
    where
        T: PartialOrd,
    {
        if self.last == Last::Destructive {
            return;
        }

        if self.last == Last::Weak {
            if let Some(i) =
                self.staged.iter().position(|(prev_item, _, prev_weakness)| {
                    prev_weakness.map_or(false, |prev_weakness| {
                        weakness < prev_weakness
                            || (weakness == prev_weakness && item > *prev_item)
                    })
                })
            {
                self.staged.remove(i);
            } else {
                return;
            }
        }

        self.staged.push((item, styles, Some(weakness)));
        self.last = Last::Weak;
    }

    /// Forces nearby weak items to collapse.
    fn destructive(&mut self, item: T, styles: StyleChain<'a>) {
        self.flush(false);
        self.builder.push(item, styles);
        self.last = Last::Destructive;
    }

    /// Allows nearby weak items to exist.
    fn supportive(&mut self, item: T, styles: StyleChain<'a>) {
        self.flush(true);
        self.builder.push(item, styles);
        self.last = Last::Supportive;
    }

    /// Has no influence on other items.
    fn ignorant(&mut self, item: T, styles: StyleChain<'a>) {
        self.staged.push((item, styles, None));
    }

    /// Iterate over the contained items.
    fn items(&self) -> impl DoubleEndedIterator<Item = &T> {
        self.builder.items().chain(self.staged.iter().map(|(item, ..)| item))
    }

    /// Return the finish style vec and the common prefix chain.
    fn finish(mut self) -> (StyleVec<T>, StyleChain<'a>) {
        self.flush(false);
        self.builder.finish()
    }

    /// Push the staged items, filtering out weak items if `supportive` is
    /// false.
    fn flush(&mut self, supportive: bool) {
        for (item, styles, meta) in self.staged.drain(..) {
            if supportive || meta.is_none() {
                self.builder.push(item, styles);
            }
        }
    }
}

impl<'a, T> Default for CollapsingBuilder<'a, T> {
    fn default() -> Self {
        Self::new()
    }
}
