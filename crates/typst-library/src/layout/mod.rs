//! Composable layouts.

mod align;
mod columns;
mod container;
#[path = "enum.rs"]
mod enum_;
mod flow;
mod fragment;
mod grid;
mod hide;
mod list;
#[path = "measure.rs"]
mod measure_;
mod pad;
mod page;
mod par;
mod place;
mod regions;
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
pub use self::fragment::*;
pub use self::grid::*;
pub use self::hide::*;
pub use self::list::*;
pub use self::measure_::*;
pub use self::pad::*;
pub use self::page::*;
pub use self::par::*;
pub use self::place::*;
pub use self::regions::*;
pub use self::repeat::*;
pub use self::spacing::*;
pub use self::stack::*;
pub use self::table::*;
pub use self::terms::*;
pub use self::transform::*;

use std::mem;

use typed_arena::Arena;
use typst::diag::SourceResult;
use typst::eval::Tracer;
use typst::model::DelayedErrors;
use typst::model::{applicable, realize, StyleVecBuilder};

use crate::math::{EquationElem, LayoutMath};
use crate::meta::DocumentElem;
use crate::prelude::*;
use crate::shared::BehavedBuilder;
use crate::text::{LinebreakElem, SmartquoteElem, SpaceElem, TextElem};
use crate::visualize::{
    CircleElem, EllipseElem, ImageElem, LineElem, PathElem, PolygonElem, RectElem,
    SquareElem,
};

/// Hook up all layout definitions.
pub(super) fn define(global: &mut Scope) {
    global.category("layout");
    global.define_type::<Length>();
    global.define_type::<Angle>();
    global.define_type::<Ratio>();
    global.define_type::<Rel<Length>>();
    global.define_type::<Fr>();
    global.define_type::<Dir>();
    global.define_type::<Align>();
    global.define_elem::<PageElem>();
    global.define_elem::<PagebreakElem>();
    global.define_elem::<VElem>();
    global.define_elem::<ParElem>();
    global.define_elem::<ParbreakElem>();
    global.define_elem::<HElem>();
    global.define_elem::<BoxElem>();
    global.define_elem::<BlockElem>();
    global.define_elem::<ListElem>();
    global.define_elem::<EnumElem>();
    global.define_elem::<TermsElem>();
    global.define_elem::<TableElem>();
    global.define_elem::<StackElem>();
    global.define_elem::<GridElem>();
    global.define_elem::<ColumnsElem>();
    global.define_elem::<ColbreakElem>();
    global.define_elem::<PlaceElem>();
    global.define_elem::<AlignElem>();
    global.define_elem::<PadElem>();
    global.define_elem::<RepeatElem>();
    global.define_elem::<MoveElem>();
    global.define_elem::<ScaleElem>();
    global.define_elem::<RotateElem>();
    global.define_elem::<HideElem>();
    global.define_func::<measure>();
}

/// Root-level layout.
pub trait LayoutRoot {
    /// Layout into one frame per page.
    fn layout_root(&self, vt: &mut Vt, styles: StyleChain) -> SourceResult<Document>;
}

impl LayoutRoot for Content {
    #[tracing::instrument(name = "Content::layout_root", skip_all)]
    fn layout_root(&self, vt: &mut Vt, styles: StyleChain) -> SourceResult<Document> {
        #[comemo::memoize]
        fn cached(
            content: &Content,
            world: Tracked<dyn World + '_>,
            introspector: Tracked<Introspector>,
            locator: Tracked<Locator>,
            delayed: TrackedMut<DelayedErrors>,
            tracer: TrackedMut<Tracer>,
            styles: StyleChain,
        ) -> SourceResult<Document> {
            let mut locator = Locator::chained(locator);
            let mut vt = Vt {
                world,
                introspector,
                locator: &mut locator,
                delayed,
                tracer,
            };
            let scratch = Scratch::default();
            let (realized, styles) = realize_root(&mut vt, &scratch, content, styles)?;
            realized
                .with::<dyn LayoutRoot>()
                .unwrap()
                .layout_root(&mut vt, styles)
        }

        tracing::info!("Starting layout");
        cached(
            self,
            vt.world,
            vt.introspector,
            vt.locator.track(),
            TrackedMut::reborrow_mut(&mut vt.delayed),
            TrackedMut::reborrow_mut(&mut vt.tracer),
            styles,
        )
    }
}

/// Layout into regions.
pub trait Layout {
    /// Layout into one frame per region.
    fn layout(
        &self,
        vt: &mut Vt,
        styles: StyleChain,
        regions: Regions,
    ) -> SourceResult<Fragment>;

    /// Layout without side effects.
    ///
    /// This element must be layouted again in the same order for the results to
    /// be valid.
    #[tracing::instrument(name = "Layout::measure", skip_all)]
    fn measure(
        &self,
        vt: &mut Vt,
        styles: StyleChain,
        regions: Regions,
    ) -> SourceResult<Fragment> {
        let mut locator = Locator::chained(vt.locator.track());
        let mut vt = Vt {
            world: vt.world,
            introspector: vt.introspector,
            locator: &mut locator,
            tracer: TrackedMut::reborrow_mut(&mut vt.tracer),
            delayed: TrackedMut::reborrow_mut(&mut vt.delayed),
        };
        self.layout(&mut vt, styles, regions)
    }
}

impl Layout for Content {
    #[tracing::instrument(name = "Content::layout", skip_all)]
    fn layout(
        &self,
        vt: &mut Vt,
        styles: StyleChain,
        regions: Regions,
    ) -> SourceResult<Fragment> {
        #[allow(clippy::too_many_arguments)]
        #[comemo::memoize]
        fn cached(
            content: &Content,
            world: Tracked<dyn World + '_>,
            introspector: Tracked<Introspector>,
            locator: Tracked<Locator>,
            delayed: TrackedMut<DelayedErrors>,
            tracer: TrackedMut<Tracer>,
            styles: StyleChain,
            regions: Regions,
        ) -> SourceResult<Fragment> {
            let mut locator = Locator::chained(locator);
            let mut vt = Vt {
                world,
                introspector,
                locator: &mut locator,
                delayed,
                tracer,
            };
            let scratch = Scratch::default();
            let (realized, styles) = realize_block(&mut vt, &scratch, content, styles)?;
            realized
                .with::<dyn Layout>()
                .unwrap()
                .layout(&mut vt, styles, regions)
        }

        tracing::info!("Layouting `Content`");

        let fragment = cached(
            self,
            vt.world,
            vt.introspector,
            vt.locator.track(),
            TrackedMut::reborrow_mut(&mut vt.delayed),
            TrackedMut::reborrow_mut(&mut vt.tracer),
            styles,
            regions,
        )?;

        vt.locator.visit_frames(&fragment);
        Ok(fragment)
    }
}

/// Realize into an element that is capable of root-level layout.
#[tracing::instrument(skip_all)]
fn realize_root<'a>(
    vt: &mut Vt,
    scratch: &'a Scratch<'a>,
    content: &'a Content,
    styles: StyleChain<'a>,
) -> SourceResult<(Content, StyleChain<'a>)> {
    if content.can::<dyn LayoutRoot>() && !applicable(content, styles) {
        return Ok((content.clone(), styles));
    }

    let mut builder = Builder::new(vt, scratch, true);
    builder.accept(content, styles)?;
    builder.interrupt_page(Some(styles), true)?;
    let (pages, shared) = builder.doc.unwrap().pages.finish();
    Ok((DocumentElem::new(pages.to_vec()).pack(), shared))
}

/// Realize into an element that is capable of block-level layout.
#[tracing::instrument(skip_all)]
fn realize_block<'a>(
    vt: &mut Vt,
    scratch: &'a Scratch<'a>,
    content: &'a Content,
    styles: StyleChain<'a>,
) -> SourceResult<(Content, StyleChain<'a>)> {
    // These elements implement `Layout` but still require a flow for
    // proper layout.
    if content.can::<dyn Layout>()
        && !content.is::<BoxElem>()
        && !content.is::<LineElem>()
        && !content.is::<RectElem>()
        && !content.is::<SquareElem>()
        && !content.is::<EllipseElem>()
        && !content.is::<CircleElem>()
        && !content.is::<ImageElem>()
        && !content.is::<PolygonElem>()
        && !content.is::<PathElem>()
        && !content.is::<PlaceElem>()
        && !applicable(content, styles)
    {
        return Ok((content.clone(), styles));
    }

    let mut builder = Builder::new(vt, scratch, false);
    builder.accept(content, styles)?;
    builder.interrupt_par()?;
    let (children, shared) = builder.flow.0.finish();
    Ok((FlowElem::new(children.to_vec()).pack(), shared))
}

/// Builds a document or a flow element from content.
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
            doc: top.then(DocBuilder::default),
            flow: FlowBuilder::default(),
            par: ParBuilder::default(),
            list: ListBuilder::default(),
        }
    }

    fn accept(
        &mut self,
        mut content: &'a Content,
        styles: StyleChain<'a>,
    ) -> SourceResult<()> {
        if content.can::<dyn LayoutMath>() && !content.is::<EquationElem>() {
            content =
                self.scratch.content.alloc(EquationElem::new(content.clone()).pack());
        }

        if let Some(realized) = realize(self.vt, content, styles)? {
            let stored = self.scratch.content.alloc(realized);
            return self.accept(stored, styles);
        }

        if let Some((elem, local)) = content.to_styled() {
            return self.styled(elem, local, styles);
        }

        if let Some(children) = content.to_sequence() {
            for elem in children {
                self.accept(elem, styles)?;
            }
            return Ok(());
        }

        if self.list.accept(content, styles) {
            return Ok(());
        }

        self.interrupt_list()?;

        if self.list.accept(content, styles) {
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
            .to::<PagebreakElem>()
            .map_or(false, |pagebreak| !pagebreak.weak(styles));

        self.interrupt_page(keep.then_some(styles), false)?;

        if let Some(doc) = &mut self.doc {
            if doc.accept(content, styles) {
                return Ok(());
            }
        }

        if content.is::<PagebreakElem>() {
            bail!(content.span(), "pagebreaks are not allowed inside of containers");
        } else {
            bail!(content.span(), "{} is not allowed here", content.func().name());
        }
    }

    fn styled(
        &mut self,
        elem: &'a Content,
        map: &'a Styles,
        styles: StyleChain<'a>,
    ) -> SourceResult<()> {
        let stored = self.scratch.styles.alloc(styles);
        let styles = stored.chain(map);
        self.interrupt_style(map, None)?;
        self.accept(elem, styles)?;
        self.interrupt_style(map, Some(styles))?;
        Ok(())
    }

    fn interrupt_style(
        &mut self,
        local: &Styles,
        outer: Option<StyleChain<'a>>,
    ) -> SourceResult<()> {
        if let Some(Some(span)) = local.interruption::<DocumentElem>() {
            if self.doc.is_none() {
                bail!(span, "document set rules are not allowed inside of containers");
            }
            if outer.is_none()
                && (!self.flow.0.is_empty()
                    || !self.par.0.is_empty()
                    || !self.list.items.is_empty())
            {
                bail!(span, "document set rules must appear before any content");
            }
        } else if let Some(Some(span)) = local.interruption::<PageElem>() {
            if self.doc.is_none() {
                bail!(span, "page configuration is not allowed inside of containers");
            }
            self.interrupt_page(outer, false)?;
        } else if local.interruption::<ParElem>().is_some()
            || local.interruption::<AlignElem>().is_some()
        {
            self.interrupt_par()?;
        } else if local.interruption::<ListElem>().is_some()
            || local.interruption::<EnumElem>().is_some()
            || local.interruption::<TermsElem>().is_some()
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

    fn interrupt_page(
        &mut self,
        styles: Option<StyleChain<'a>>,
        last: bool,
    ) -> SourceResult<()> {
        self.interrupt_par()?;
        let Some(doc) = &mut self.doc else { return Ok(()) };
        if (doc.keep_next && styles.is_some()) || self.flow.0.has_strong_elements(last) {
            let (flow, shared) = mem::take(&mut self.flow).0.finish();
            let styles = if shared == StyleChain::default() {
                styles.unwrap_or_default()
            } else {
                shared
            };
            let page = PageElem::new(FlowElem::new(flow.to_vec()).pack());
            let stored = self.scratch.content.alloc(page.pack());
            self.accept(stored, styles)?;
        }
        Ok(())
    }
}

/// Accepts pagebreaks and pages.
struct DocBuilder<'a> {
    /// The page runs built so far.
    pages: StyleVecBuilder<'a, Content>,
    /// Whether to keep a following page even if it is empty.
    keep_next: bool,
    /// Whether the next page should be cleared to an even or odd number.
    clear_next: Option<Parity>,
}

impl<'a> DocBuilder<'a> {
    fn accept(&mut self, content: &Content, styles: StyleChain<'a>) -> bool {
        if let Some(pagebreak) = content.to::<PagebreakElem>() {
            self.keep_next = !pagebreak.weak(styles);
            self.clear_next = pagebreak.to(styles);
            return true;
        }

        if let Some(page) = content.to::<PageElem>() {
            let elem = if let Some(clear_to) = self.clear_next.take() {
                let mut page = page.clone();
                page.push_clear_to(Some(clear_to));
                page.pack()
            } else {
                content.clone()
            };

            self.pages.push(elem, styles);
            self.keep_next = false;
            return true;
        }

        false
    }
}

impl Default for DocBuilder<'_> {
    fn default() -> Self {
        Self {
            pages: StyleVecBuilder::new(),
            keep_next: true,
            clear_next: None,
        }
    }
}

/// Accepts flow content.
#[derive(Default)]
struct FlowBuilder<'a>(BehavedBuilder<'a>, bool);

impl<'a> FlowBuilder<'a> {
    fn accept(&mut self, content: &'a Content, styles: StyleChain<'a>) -> bool {
        if content.is::<ParbreakElem>() {
            self.1 = true;
            return true;
        }

        let last_was_parbreak = self.1;
        self.1 = false;

        if content.is::<VElem>()
            || content.is::<ColbreakElem>()
            || content.is::<MetaElem>()
            || content.is::<PlaceElem>()
        {
            self.0.push(content.clone(), styles);
            return true;
        }

        if content.can::<dyn Layout>() || content.is::<ParElem>() {
            let is_tight_list = if let Some(elem) = content.to::<ListElem>() {
                elem.tight(styles)
            } else if let Some(elem) = content.to::<EnumElem>() {
                elem.tight(styles)
            } else if let Some(elem) = content.to::<TermsElem>() {
                elem.tight(styles)
            } else {
                false
            };

            if !last_was_parbreak && is_tight_list {
                let leading = ParElem::leading_in(styles);
                let spacing = VElem::list_attach(leading.into());
                self.0.push(spacing.pack(), styles);
            }

            let (above, below) = if let Some(block) = content.to::<BlockElem>() {
                (block.above(styles), block.below(styles))
            } else {
                (BlockElem::above_in(styles), BlockElem::below_in(styles))
            };

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
        if content.is::<MetaElem>() {
            if self.0.has_strong_elements(false) {
                self.0.push(content.clone(), styles);
                return true;
            }
        } else if content.is::<SpaceElem>()
            || content.is::<TextElem>()
            || content.is::<HElem>()
            || content.is::<LinebreakElem>()
            || content.is::<SmartquoteElem>()
            || content.to::<EquationElem>().map_or(false, |elem| !elem.block(styles))
            || content.is::<BoxElem>()
        {
            self.0.push(content.clone(), styles);
            return true;
        }

        false
    }

    fn finish(self) -> (Content, StyleChain<'a>) {
        let (children, shared) = self.0.finish();
        (ParElem::new(children.to_vec()).pack(), shared)
    }
}

/// Accepts list / enum items, spaces, paragraph breaks.
struct ListBuilder<'a> {
    /// The list items collected so far.
    items: StyleVecBuilder<'a, Content>,
    /// Whether the list contains no paragraph breaks.
    tight: bool,
    /// Trailing content for which it is unclear whether it is part of the list.
    staged: Vec<(&'a Content, StyleChain<'a>)>,
}

impl<'a> ListBuilder<'a> {
    fn accept(&mut self, content: &'a Content, styles: StyleChain<'a>) -> bool {
        if !self.items.is_empty()
            && (content.is::<SpaceElem>() || content.is::<ParbreakElem>())
        {
            self.staged.push((content, styles));
            return true;
        }

        if (content.is::<ListItem>()
            || content.is::<EnumItem>()
            || content.is::<TermItem>())
            && self
                .items
                .elems()
                .next()
                .map_or(true, |first| first.func() == content.func())
        {
            self.items.push(content.clone(), styles);
            self.tight &= self.staged.drain(..).all(|(t, _)| !t.is::<ParbreakElem>());
            return true;
        }

        false
    }

    fn finish(self) -> (Content, StyleChain<'a>) {
        let (items, shared) = self.items.finish();
        let item = items.items().next().unwrap();
        let output = if item.is::<ListItem>() {
            ListElem::new(
                items
                    .iter()
                    .map(|(item, local)| {
                        let item = item.to::<ListItem>().unwrap();
                        item.clone().with_body(item.body().styled_with_map(local.clone()))
                    })
                    .collect::<Vec<_>>(),
            )
            .with_tight(self.tight)
            .pack()
        } else if item.is::<EnumItem>() {
            EnumElem::new(
                items
                    .iter()
                    .map(|(item, local)| {
                        let item = item.to::<EnumItem>().unwrap();
                        item.clone().with_body(item.body().styled_with_map(local.clone()))
                    })
                    .collect::<Vec<_>>(),
            )
            .with_tight(self.tight)
            .pack()
        } else if item.is::<TermItem>() {
            TermsElem::new(
                items
                    .iter()
                    .map(|(item, local)| {
                        let item = item.to::<TermItem>().unwrap();
                        item.clone()
                            .with_term(item.term().styled_with_map(local.clone()))
                            .with_description(
                                item.description().styled_with_map(local.clone()),
                            )
                    })
                    .collect::<Vec<_>>(),
            )
            .with_tight(self.tight)
            .pack()
        } else {
            unreachable!()
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
