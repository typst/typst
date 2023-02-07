//! Composable layouts.

mod align;
mod columns;
mod container;
#[path = "enum.rs"]
mod enum_;
mod flow;
mod grid;
mod hide;
mod list;
mod pad;
mod page;
mod par;
mod place;
mod repeat;
mod spacing;
mod stack;
mod table;
mod terms;
mod transform;

pub use self::align::*;
pub use self::columns::*;
pub use self::container::*;
pub use self::enum_::*;
pub use self::flow::*;
pub use self::grid::*;
pub use self::hide::*;
pub use self::list::*;
pub use self::pad::*;
pub use self::page::*;
pub use self::par::*;
pub use self::place::*;
pub use self::repeat::*;
pub use self::spacing::*;
pub use self::stack::*;
pub use self::table::*;
pub use self::terms::*;
pub use self::transform::*;

use std::mem;

use typed_arena::Arena;
use typst::diag::SourceResult;
use typst::geom::*;
use typst::model::{
    applicable, capability, realize, Content, Node, SequenceNode, Style, StyleChain,
    StyleVecBuilder, StyledNode,
};

use crate::math::{FormulaNode, LayoutMath};
use crate::meta::DocumentNode;
use crate::prelude::*;
use crate::shared::BehavedBuilder;
use crate::text::{LinebreakNode, SmartQuoteNode, SpaceNode, TextNode};

/// Root-level layout.
#[capability]
pub trait LayoutRoot {
    /// Layout into one frame per page.
    fn layout_root(&self, vt: &mut Vt, styles: StyleChain) -> SourceResult<Document>;
}

impl LayoutRoot for Content {
    fn layout_root(&self, vt: &mut Vt, styles: StyleChain) -> SourceResult<Document> {
        #[comemo::memoize]
        fn cached(
            node: &Content,
            world: Tracked<dyn World>,
            provider: TrackedMut<StabilityProvider>,
            introspector: Tracked<Introspector>,
            styles: StyleChain,
        ) -> SourceResult<Document> {
            let mut vt = Vt { world, provider, introspector };
            let scratch = Scratch::default();
            let (realized, styles) = realize_root(&mut vt, &scratch, node, styles)?;
            realized
                .with::<dyn LayoutRoot>()
                .unwrap()
                .layout_root(&mut vt, styles)
        }

        cached(
            self,
            vt.world,
            TrackedMut::reborrow_mut(&mut vt.provider),
            vt.introspector,
            styles,
        )
    }
}

/// Layout into regions.
#[capability]
pub trait Layout {
    /// Layout into one frame per region.
    fn layout(
        &self,
        vt: &mut Vt,
        styles: StyleChain,
        regions: Regions,
    ) -> SourceResult<Fragment>;
}

impl Layout for Content {
    fn layout(
        &self,
        vt: &mut Vt,
        styles: StyleChain,
        regions: Regions,
    ) -> SourceResult<Fragment> {
        #[comemo::memoize]
        fn cached(
            node: &Content,
            world: Tracked<dyn World>,
            provider: TrackedMut<StabilityProvider>,
            introspector: Tracked<Introspector>,
            styles: StyleChain,
            regions: Regions,
        ) -> SourceResult<Fragment> {
            let mut vt = Vt { world, provider, introspector };
            let scratch = Scratch::default();
            let (realized, styles) = realize_block(&mut vt, &scratch, node, styles)?;
            let barrier = Style::Barrier(realized.id());
            let styles = styles.chain_one(&barrier);
            realized
                .with::<dyn Layout>()
                .unwrap()
                .layout(&mut vt, styles, regions)
        }

        cached(
            self,
            vt.world,
            TrackedMut::reborrow_mut(&mut vt.provider),
            vt.introspector,
            styles,
            regions,
        )
    }
}

/// Inline-level layout.
#[capability]
pub trait Inline: Layout {}

/// A sequence of regions to layout into.
#[derive(Debug, Copy, Clone, Hash)]
pub struct Regions<'a> {
    /// The (remaining) size of the first region.
    pub first: Size,
    /// The base size for relative sizing.
    pub base: Size,
    /// The height of followup regions. The width is the same for all regions.
    pub backlog: &'a [Abs],
    /// The height of the final region that is repeated once the backlog is
    /// drained. The width is the same for all regions.
    pub last: Option<Abs>,
    /// Whether nodes should expand to fill the regions instead of shrinking to
    /// fit the content.
    pub expand: Axes<bool>,
}

impl<'a> Regions<'a> {
    /// Create a new region sequence with exactly one region.
    pub fn one(size: Size, base: Size, expand: Axes<bool>) -> Self {
        Self {
            first: size,
            base,
            backlog: &[],
            last: None,
            expand,
        }
    }

    /// Create a new sequence of same-size regions that repeats indefinitely.
    pub fn repeat(size: Size, base: Size, expand: Axes<bool>) -> Self {
        Self {
            first: size,
            base,
            backlog: &[],
            last: Some(size.y),
            expand,
        }
    }

    /// Create new regions where all sizes are mapped with `f`.
    ///
    /// Note that since all regions must have the same width, the width returned
    /// by `f` is ignored for the backlog and the final region.
    pub fn map<'v, F>(&self, backlog: &'v mut Vec<Abs>, mut f: F) -> Regions<'v>
    where
        F: FnMut(Size) -> Size,
    {
        let x = self.first.x;
        backlog.clear();
        backlog.extend(self.backlog.iter().map(|&y| f(Size::new(x, y)).y));
        Regions {
            first: f(self.first),
            base: f(self.base),
            backlog,
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
        if let Some(height) = self
            .backlog
            .split_first()
            .map(|(first, tail)| {
                self.backlog = tail;
                *first
            })
            .or(self.last)
        {
            self.first.y = height;
            self.base.y = height;
        }
    }

    /// An iterator that returns the sizes of the first and all following
    /// regions, equivalently to what would be produced by calling
    /// [`next()`](Self::next) repeatedly until all regions are exhausted.
    /// This iterator may be infinite.
    pub fn iter(&self) -> impl Iterator<Item = Size> + '_ {
        let first = std::iter::once(self.first);
        let backlog = self.backlog.iter();
        let last = self.last.iter().cycle();
        first.chain(backlog.chain(last).map(|&h| Size::new(self.first.x, h)))
    }
}

/// Realize into a node that is capable of root-level layout.
fn realize_root<'a>(
    vt: &mut Vt,
    scratch: &'a Scratch<'a>,
    content: &'a Content,
    styles: StyleChain<'a>,
) -> SourceResult<(Content, StyleChain<'a>)> {
    if content.has::<dyn LayoutRoot>() && !applicable(content, styles) {
        return Ok((content.clone(), styles));
    }

    let mut builder = Builder::new(vt, &scratch, true);
    builder.accept(content, styles)?;
    builder.interrupt_page(Some(styles))?;
    let (pages, shared) = builder.doc.unwrap().pages.finish();
    Ok((DocumentNode(pages).pack(), shared))
}

/// Realize into a node that is capable of block-level layout.
fn realize_block<'a>(
    vt: &mut Vt,
    scratch: &'a Scratch<'a>,
    content: &'a Content,
    styles: StyleChain<'a>,
) -> SourceResult<(Content, StyleChain<'a>)> {
    if content.has::<dyn Layout>() && !applicable(content, styles) {
        return Ok((content.clone(), styles));
    }

    let mut builder = Builder::new(vt, &scratch, false);
    builder.accept(content, styles)?;
    builder.interrupt_par()?;
    let (children, shared) = builder.flow.0.finish();
    Ok((FlowNode(children, false).pack(), shared))
}

/// Builds a document or a flow node from content.
struct Builder<'a, 'v, 't> {
    /// The virtual typesetter.
    vt: &'v mut Vt<'t>,
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

impl<'a, 'v, 't> Builder<'a, 'v, 't> {
    fn new(vt: &'v mut Vt<'t>, scratch: &'a Scratch<'a>, top: bool) -> Self {
        Self {
            vt,
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
        // Prepare only if this is the first application for this node.
        if let Some(node) = content.with::<dyn Prepare>() {
            if !content.is_prepared() {
                let prepared =
                    node.prepare(self.vt, content.clone().prepared(), styles)?;
                let stored = self.scratch.content.alloc(prepared);
                return self.accept(stored, styles);
            }
        }

        if let Some(styled) = content.to::<StyledNode>() {
            return self.styled(styled, styles);
        }

        if let Some(seq) = content.to::<SequenceNode>() {
            for sub in &seq.0 {
                self.accept(sub, styles)?;
            }
            return Ok(());
        }

        if let Some(realized) = realize(self.vt, content, styles)? {
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

        if let Some(span) = content.span() {
            bail!(span, "not allowed here");
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
        if let Some(Some(span)) = map.interruption::<DocumentNode>() {
            if self.doc.is_none() {
                bail!(span, "not allowed here");
            }
            if !self.flow.0.is_empty()
                || !self.par.0.is_empty()
                || !self.list.items.is_empty()
            {
                bail!(span, "must appear before any content");
            }
        } else if let Some(Some(span)) = map.interruption::<PageNode>() {
            if self.doc.is_none() {
                bail!(span, "not allowed here");
            }
            self.interrupt_page(styles)?;
        } else if map.interruption::<ParNode>().is_some()
            || map.interruption::<AlignNode>().is_some()
        {
            self.interrupt_par()?;
        } else if map.interruption::<ListNode>().is_some()
            || map.interruption::<EnumNode>().is_some()
            || map.interruption::<TermsNode>().is_some()
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
            let (flow, shared) = mem::take(&mut self.flow).0.finish();
            let styles =
                if shared == StyleChain::default() { styles.unwrap() } else { shared };
            let page = PageNode(FlowNode(flow, true).pack()).pack();
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

        if content.has::<dyn Layout>() || content.is::<ParNode>() {
            let is_tight_list = if let Some(node) = content.to::<ListNode>() {
                node.tight
            } else if let Some(node) = content.to::<EnumNode>() {
                node.tight
            } else if let Some(node) = content.to::<TermsNode>() {
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
            || content.is::<FormulaNode>()
            || content.has::<dyn Inline>()
        {
            self.0.push(content.clone(), styles);
            return true;
        }

        if content.has::<dyn LayoutMath>() {
            let formula = FormulaNode { body: content.clone(), block: false }.pack();
            self.0.push(formula, styles);
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
            if self.items.items().next().map_or(true, |first| {
                std::mem::discriminant(item) == std::mem::discriminant(first)
            }) {
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
        let output = match item {
            ListItem::List(_) => ListNode {
                tight: self.tight,
                items: items.map(|item| match item {
                    ListItem::List(item) => item.clone(),
                    _ => panic!("wrong list item"),
                }),
            }
            .pack(),
            ListItem::Enum(..) => EnumNode {
                tight: self.tight,
                items: items.map(|item| match item {
                    ListItem::Enum(number, body) => (*number, body.clone()),
                    _ => panic!("wrong list item"),
                }),
            }
            .pack(),
            ListItem::Term(_) => TermsNode {
                tight: self.tight,
                items: items.map(|item| match item {
                    ListItem::Term(item) => item.clone(),
                    _ => panic!("wrong list item"),
                }),
            }
            .pack(),
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

/// An item in a list.
#[capable]
#[derive(Debug, Clone, Hash)]
pub enum ListItem {
    /// An item of a bullet list.
    List(Content),
    /// An item of a numbered list.
    Enum(Option<NonZeroUsize>, Content),
    /// An item of a term list.
    Term(TermItem),
}

#[node]
impl ListItem {}
