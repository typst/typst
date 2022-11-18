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

pub use self::align::*;
pub use self::columns::*;
pub use self::container::*;
pub use self::flow::*;
pub use self::grid::*;
pub use self::pad::*;
pub use self::page::*;
pub use self::place::*;
pub use self::spacing::*;
pub use self::stack::*;
pub use self::transform::*;

use std::mem;

use comemo::Tracked;
use typed_arena::Arena;
use typst::diag::SourceResult;
use typst::frame::Frame;
use typst::geom::*;
use typst::model::{
    capability, Content, Node, SequenceNode, Show, Style, StyleChain, StyleVecBuilder,
    StyledNode,
};
use typst::World;

use crate::core::BehavedBuilder;
use crate::prelude::*;
use crate::structure::{
    DescNode, DocNode, EnumNode, ListItem, ListNode, DESC, ENUM, LIST,
};
use crate::text::{
    LinebreakNode, ParNode, ParbreakNode, SmartQuoteNode, SpaceNode, TextNode,
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
        if !self.has::<dyn Show>() || !styles.applicable(self) {
            if let Some(node) = self.to::<dyn LayoutBlock>() {
                let barrier = Style::Barrier(self.id());
                let styles = barrier.chain(&styles);
                return node.layout_block(world, regions, styles);
            }
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
    ) -> SourceResult<Frame>;
}

impl LayoutInline for Content {
    #[comemo::memoize]
    fn layout_inline(
        &self,
        world: Tracked<dyn World>,
        regions: &Regions,
        styles: StyleChain,
    ) -> SourceResult<Frame> {
        assert!(regions.backlog.is_empty());
        assert!(regions.last.is_none());

        if !self.has::<dyn Show>() || !styles.applicable(self) {
            if let Some(node) = self.to::<dyn LayoutInline>() {
                let barrier = Style::Barrier(self.id());
                let styles = barrier.chain(&styles);
                return node.layout_inline(world, regions, styles);
            }

            if let Some(node) = self.to::<dyn LayoutBlock>() {
                let barrier = Style::Barrier(self.id());
                let styles = barrier.chain(&styles);
                return Ok(node.layout_block(world, regions, styles)?.remove(0));
            }
        }

        let scratch = Scratch::default();
        let mut builder = Builder::new(world, &scratch, false);
        builder.accept(self, styles)?;
        let (flow, shared) = builder.into_flow(styles)?;
        Ok(flow.layout_block(world, regions, shared)?.remove(0))
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
    content: Arena<Content>,
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
        if content.is::<TextNode>() {
            if let Some(realized) = styles.apply(self.world, content)? {
                let stored = self.scratch.content.alloc(realized);
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

    fn show(&mut self, content: &Content, styles: StyleChain<'a>) -> SourceResult<bool> {
        let Some(realized) = styles.apply(self.world, content)? else {
            return Ok(false);
        };

        let stored = self.scratch.content.alloc(realized);
        self.accept(stored, styles)?;

        Ok(true)
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
    fn accept(&mut self, content: &Content, styles: StyleChain<'a>) {
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
        Self { pages: StyleVecBuilder::new(), keep_next: true }
    }
}

/// Accepts flow content.
#[derive(Default)]
struct FlowBuilder<'a>(BehavedBuilder<'a>, bool);

impl<'a> FlowBuilder<'a> {
    fn accept(&mut self, content: &Content, styles: StyleChain<'a>) -> bool {
        let last_was_parbreak = std::mem::replace(&mut self.1, false);

        if content.is::<ParbreakNode>() {
            self.1 = true;
            return true;
        } else if content.is::<VNode>() || content.is::<ColbreakNode>() {
            self.0.push(content.clone(), styles);
            return true;
        } else if content.has::<dyn LayoutBlock>() {
            if !last_was_parbreak {
                let tight = if let Some(node) = content.downcast::<ListNode>() {
                    node.tight
                } else if let Some(node) = content.downcast::<EnumNode>() {
                    node.tight
                } else if let Some(node) = content.downcast::<DescNode>() {
                    node.tight
                } else {
                    false
                };

                if tight {
                    let leading = styles.get(ParNode::LEADING);
                    let spacing = VNode::list_attach(leading.into());
                    self.0.push(spacing.pack(), styles);
                }
            }

            let above = styles.get(BlockNode::ABOVE);
            let below = styles.get(BlockNode::BELOW);
            self.0.push(above.pack(), styles);
            self.0.push(content.clone(), styles);
            self.0.push(below.pack(), styles);
            return true;
        }

        false
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
struct ParBuilder<'a>(BehavedBuilder<'a>);

impl<'a> ParBuilder<'a> {
    fn accept(&mut self, content: &Content, styles: StyleChain<'a>) -> bool {
        if content.is::<SpaceNode>()
            || content.is::<LinebreakNode>()
            || content.is::<HNode>()
            || content.is::<SmartQuoteNode>()
            || content.is::<TextNode>()
            || content.has::<dyn LayoutInline>()
        {
            self.0.push(content.clone(), styles);
            return true;
        }

        false
    }

    fn finish(self, parent: &mut Builder<'a>) {
        let (children, shared) = self.0.finish();
        if !children.is_empty() {
            parent.flow.accept(&ParNode(children).pack(), shared);
        }
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
        if !self.items.is_empty()
            && (content.is::<SpaceNode>() || content.is::<ParbreakNode>())
        {
            self.staged.push((content, styles));
        } else if let Some(item) = content.downcast::<ListItem>() {
            if self
                .items
                .items()
                .next()
                .map_or(false, |first| item.kind() != first.kind())
            {
                return false;
            }

            self.items.push(item.clone(), styles);
            self.tight &= self.staged.drain(..).all(|(t, _)| !t.is::<ParbreakNode>());
        } else {
            return false;
        }

        true
    }

    fn finish(self, parent: &mut Builder<'a>) -> SourceResult<()> {
        let (items, shared) = self.items.finish();
        if let Some(item) = items.items().next() {
            let tight = self.tight;
            let content = match item.kind() {
                LIST => ListNode::<LIST> { tight, items }.pack(),
                ENUM => ListNode::<ENUM> { tight, items }.pack(),
                DESC | _ => ListNode::<DESC> { tight, items }.pack(),
            };

            let stored = parent.scratch.content.alloc(content);
            parent.accept(stored, shared)?;
        }

        for (content, styles) in self.staged {
            parent.accept(content, styles)?;
        }

        parent.list.tight = true;

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
