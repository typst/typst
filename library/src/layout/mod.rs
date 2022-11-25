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
    applicable, capability, realize, Content, Node, SequenceNode, Style, StyleChain,
    StyleVecBuilder, StyledNode,
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
pub trait LayoutRoot {
    /// Layout into one frame per page.
    fn layout_root(
        &self,
        world: Tracked<dyn World>,
        styles: StyleChain,
    ) -> SourceResult<Vec<Frame>>;
}

impl LayoutRoot for Content {
    #[comemo::memoize]
    fn layout_root(
        &self,
        world: Tracked<dyn World>,
        styles: StyleChain,
    ) -> SourceResult<Vec<Frame>> {
        let scratch = Scratch::default();
        let (realized, styles) = realize_root(world, &scratch, self, styles)?;
        realized.with::<dyn LayoutRoot>().unwrap().layout_root(world, styles)
    }
}

/// Block-level layout.
#[capability]
pub trait LayoutBlock {
    /// Layout into one frame per region.
    fn layout_block(
        &self,
        world: Tracked<dyn World>,
        styles: StyleChain,
        regions: &Regions,
    ) -> SourceResult<Vec<Frame>>;
}

impl LayoutBlock for Content {
    #[comemo::memoize]
    fn layout_block(
        &self,
        world: Tracked<dyn World>,
        styles: StyleChain,
        regions: &Regions,
    ) -> SourceResult<Vec<Frame>> {
        let scratch = Scratch::default();
        let (realized, styles) = realize_block(world, &scratch, self, styles)?;
        let barrier = Style::Barrier(realized.id());
        let styles = styles.chain_one(&barrier);
        realized
            .with::<dyn LayoutBlock>()
            .unwrap()
            .layout_block(world, styles, regions)
    }
}

/// Inline-level layout.
#[capability]
pub trait LayoutInline {
    /// Layout into a single frame.
    fn layout_inline(
        &self,
        world: Tracked<dyn World>,
        styles: StyleChain,
        regions: &Regions,
    ) -> SourceResult<Frame>;
}

impl LayoutInline for Content {
    #[comemo::memoize]
    fn layout_inline(
        &self,
        world: Tracked<dyn World>,
        styles: StyleChain,
        regions: &Regions,
    ) -> SourceResult<Frame> {
        assert!(regions.backlog.is_empty());
        assert!(regions.last.is_none());

        if self.has::<dyn LayoutInline>() && !applicable(self, styles) {
            let barrier = Style::Barrier(self.id());
            let styles = styles.chain_one(&barrier);
            return self
                .with::<dyn LayoutInline>()
                .unwrap()
                .layout_inline(world, styles, regions);
        }

        Ok(self.layout_block(world, styles, regions)?.remove(0))
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

/// Realize into a node that is capable of root-level layout.
fn realize_root<'a>(
    world: Tracked<'a, dyn World>,
    scratch: &'a Scratch<'a>,
    content: &'a Content,
    styles: StyleChain<'a>,
) -> SourceResult<(Content, StyleChain<'a>)> {
    if content.has::<dyn LayoutRoot>() && !applicable(content, styles) {
        return Ok((content.clone(), styles));
    }

    let mut builder = Builder::new(world, &scratch, true);
    builder.accept(content, styles)?;
    builder.interrupt_page(Some(styles))?;
    let (pages, shared) = builder.doc.unwrap().pages.finish();
    Ok((DocNode(pages).pack(), shared))
}

/// Realize into a node that is capable of block-level layout.
fn realize_block<'a>(
    world: Tracked<'a, dyn World>,
    scratch: &'a Scratch<'a>,
    content: &'a Content,
    styles: StyleChain<'a>,
) -> SourceResult<(Content, StyleChain<'a>)> {
    if content.has::<dyn LayoutBlock>() && !applicable(content, styles) {
        return Ok((content.clone(), styles));
    }

    let mut builder = Builder::new(world, &scratch, false);
    builder.accept(content, styles)?;
    builder.interrupt_par()?;
    let (children, shared) = builder.flow.0.finish();
    Ok((FlowNode(children).pack(), shared))
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

    fn accept(
        &mut self,
        content: &'a Content,
        styles: StyleChain<'a>,
    ) -> SourceResult<()> {
        if let Some(styled) = content.to::<StyledNode>() {
            return self.styled(styled, styles);
        }

        if let Some(seq) = content.to::<SequenceNode>() {
            for sub in &seq.0 {
                self.accept(sub, styles)?;
            }
            return Ok(());
        }

        if let Some(realized) = realize(self.world, content, styles)? {
            let stored = self.scratch.content.alloc(realized);
            return self.accept(stored, styles);
        }

        if self.list.accept(content, styles) {
            return Ok(());
        }

        self.interrupt_list()?;

        if content.is::<ListItem>() {
            self.list.accept(content, styles);
            return Ok(());
        }

        if self.par.accept(content, styles) {
            return Ok(());
        }

        self.interrupt_par()?;

        if self.flow.accept(content, styles) {
            return Ok(());
        }

        let keep = content
            .to::<PagebreakNode>()
            .map_or(false, |pagebreak| !pagebreak.weak);

        self.interrupt_page(keep.then(|| styles))?;

        if let Some(doc) = &mut self.doc {
            if doc.accept(content, styles) {
                return Ok(());
            }
        }

        Ok(())
    }

    fn styled(
        &mut self,
        styled: &'a StyledNode,
        styles: StyleChain<'a>,
    ) -> SourceResult<()> {
        let stored = self.scratch.styles.alloc(styles);
        let styles = stored.chain(&styled.map);
        self.interrupt_style(&styled.map, None)?;
        self.accept(&styled.sub, styles)?;
        self.interrupt_style(&styled.map, Some(styles))?;
        Ok(())
    }

    fn interrupt_style(
        &mut self,
        map: &StyleMap,
        styles: Option<StyleChain<'a>>,
    ) -> SourceResult<()> {
        if map.interrupts::<PageNode>() {
            self.interrupt_page(styles)?;
        } else if map.interrupts::<ParNode>() {
            self.interrupt_par()?;
        } else if map.interrupts::<ListNode>()
            || map.interrupts::<EnumNode>()
            || map.interrupts::<DescNode>()
        {
            self.interrupt_list()?;
        }
        Ok(())
    }

    fn interrupt_list(&mut self) -> SourceResult<()> {
        if !self.list.items.is_empty() {
            let staged = mem::take(&mut self.list.staged);
            let (list, styles) = mem::take(&mut self.list).finish();
            let stored = self.scratch.content.alloc(list);
            self.accept(stored, styles)?;
            for (content, styles) in staged {
                self.accept(content, styles)?;
            }
        }
        Ok(())
    }

    fn interrupt_par(&mut self) -> SourceResult<()> {
        self.interrupt_list()?;
        if !self.par.0.is_empty() {
            let (par, styles) = mem::take(&mut self.par).finish();
            let stored = self.scratch.content.alloc(par);
            self.accept(stored, styles)?;
        }

        Ok(())
    }

    fn interrupt_page(&mut self, styles: Option<StyleChain<'a>>) -> SourceResult<()> {
        self.interrupt_par()?;
        let Some(doc) = &mut self.doc else { return Ok(()) };
        if !self.flow.0.is_empty() || (doc.keep_next && styles.is_some()) {
            let (flow, shared) = mem::take(&mut self.flow).finish();
            let styles =
                if shared == StyleChain::default() { styles.unwrap() } else { shared };
            let page = PageNode(flow).pack();
            let stored = self.scratch.content.alloc(page);
            self.accept(stored, styles)?;
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
    fn accept(&mut self, content: &Content, styles: StyleChain<'a>) -> bool {
        if let Some(pagebreak) = content.to::<PagebreakNode>() {
            self.keep_next = !pagebreak.weak;
            return true;
        }

        if let Some(page) = content.to::<PageNode>() {
            self.pages.push(page.clone(), styles);
            self.keep_next = false;
            return true;
        }

        false
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
    fn accept(&mut self, content: &'a Content, styles: StyleChain<'a>) -> bool {
        if content.is::<ParbreakNode>() {
            self.1 = true;
            return true;
        }

        let last_was_parbreak = self.1;
        self.1 = false;

        if content.is::<VNode>() || content.is::<ColbreakNode>() {
            self.0.push(content.clone(), styles);
            return true;
        }

        if content.has::<dyn LayoutBlock>() {
            let is_tight_list = if let Some(node) = content.to::<ListNode>() {
                node.tight
            } else if let Some(node) = content.to::<EnumNode>() {
                node.tight
            } else if let Some(node) = content.to::<DescNode>() {
                node.tight
            } else {
                false
            };

            if !last_was_parbreak && is_tight_list {
                let leading = styles.get(ParNode::LEADING);
                let spacing = VNode::list_attach(leading.into());
                self.0.push(spacing.pack(), styles);
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

    fn finish(self) -> (Content, StyleChain<'a>) {
        let (flow, shared) = self.0.finish();
        (FlowNode(flow).pack(), shared)
    }
}

/// Accepts paragraph content.
#[derive(Default)]
struct ParBuilder<'a>(BehavedBuilder<'a>);

impl<'a> ParBuilder<'a> {
    fn accept(&mut self, content: &'a Content, styles: StyleChain<'a>) -> bool {
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

    fn finish(self) -> (Content, StyleChain<'a>) {
        let (children, shared) = self.0.finish();
        (ParNode(children).pack(), shared)
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
            return true;
        }

        if let Some(item) = content.to::<ListItem>() {
            if self
                .items
                .items()
                .next()
                .map_or(true, |first| item.kind() == first.kind())
            {
                self.items.push(item.clone(), styles);
                self.tight &= self.staged.drain(..).all(|(t, _)| !t.is::<ParbreakNode>());
                return true;
            }
        }

        false
    }

    fn finish(self) -> (Content, StyleChain<'a>) {
        let (items, shared) = self.items.finish();
        let item = items.items().next().unwrap();
        let output = match item.kind() {
            LIST => ListNode::<LIST> { tight: self.tight, items }.pack(),
            ENUM => ListNode::<ENUM> { tight: self.tight, items }.pack(),
            DESC | _ => ListNode::<DESC> { tight: self.tight, items }.pack(),
        };
        (output, shared)
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
