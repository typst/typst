//! Realization of content.
//!
//! *Realization* is the process of applying show rules to produce
//! something that can be laid out directly.
//!
//! Currently, there are issues with the realization process, and
//! it is subject to changes in the future.

mod arenas;
mod behaviour;
mod process;

use once_cell::unsync::Lazy;

pub use self::arenas::Arenas;
pub use self::behaviour::{Behave, BehavedBuilder, Behaviour, StyleVec};
pub use self::process::process;

use std::mem;

use crate::diag::{bail, SourceResult};
use crate::engine::{Engine, Route};
use crate::foundations::{
    Content, NativeElement, Packed, SequenceElem, Smart, StyleChain, StyledElem, Styles,
};
use crate::introspection::{SplitLocator, TagElem};
use crate::layout::{
    AlignElem, BlockElem, BoxElem, ColbreakElem, FlowElem, FlushElem, HElem, InlineElem,
    PageElem, PagebreakElem, Parity, PlaceElem, VElem,
};
use crate::math::{EquationElem, LayoutMath};
use crate::model::{
    CiteElem, CiteGroup, DocumentElem, DocumentInfo, EnumElem, EnumItem, ListElem,
    ListItem, ParElem, ParbreakElem, TermItem, TermsElem,
};
use crate::syntax::Span;
use crate::text::{LinebreakElem, SmartQuoteElem, SpaceElem, TextElem};

/// A pair of content and a style chain that applies to it.
type Pair<'a> = (&'a Content, StyleChain<'a>);

/// Realize at the root-level.
#[typst_macros::time(name = "realize root")]
pub fn realize_root<'a>(
    engine: &mut Engine<'a>,
    locator: &mut SplitLocator<'a>,
    arenas: &'a Arenas<'a>,
    content: &'a Content,
    styles: StyleChain<'a>,
) -> SourceResult<(StyleVec, StyleChain<'a>, DocumentInfo)> {
    let mut builder = Builder::new(engine, locator, arenas, true);
    builder.accept(content, styles)?;
    builder.interrupt_page(Some(styles), true)?;
    Ok(builder.doc.unwrap().finish())
}

/// Realize into a `FlowElem`, an element that is capable of block-level layout.
#[typst_macros::time(name = "realize flow")]
pub fn realize_flow<'a>(
    engine: &mut Engine<'a>,
    locator: &mut SplitLocator<'a>,
    arenas: &'a Arenas<'a>,
    content: &'a Content,
    styles: StyleChain<'a>,
) -> SourceResult<(Packed<FlowElem>, StyleChain<'a>)> {
    let mut builder = Builder::new(engine, locator, arenas, false);
    builder.accept(content, styles)?;
    builder.interrupt_par()?;
    Ok(builder.flow.finish())
}

/// Builds a document or a flow element from content.
struct Builder<'a, 'v> {
    /// The engine.
    engine: &'v mut Engine<'a>,
    /// Assigns unique locations to elements.
    locator: &'v mut SplitLocator<'a>,
    /// Scratch arenas for building.
    arenas: &'a Arenas<'a>,
    /// The current document building state.
    doc: Option<DocBuilder<'a>>,
    /// The current flow building state.
    flow: FlowBuilder<'a>,
    /// The current paragraph building state.
    par: ParBuilder<'a>,
    /// The current list building state.
    list: ListBuilder<'a>,
    /// The current citation grouping state.
    cites: CiteGroupBuilder<'a>,
}

impl<'a, 'v> Builder<'a, 'v> {
    fn new(
        engine: &'v mut Engine<'a>,
        locator: &'v mut SplitLocator<'a>,
        arenas: &'a Arenas<'a>,
        top: bool,
    ) -> Self {
        Self {
            engine,
            locator,
            arenas,
            doc: top.then(DocBuilder::default),
            flow: FlowBuilder::default(),
            par: ParBuilder::default(),
            list: ListBuilder::default(),
            cites: CiteGroupBuilder::default(),
        }
    }

    /// Adds a piece of content to this builder.
    fn accept(
        &mut self,
        mut content: &'a Content,
        styles: StyleChain<'a>,
    ) -> SourceResult<()> {
        // Implicitly wrap math content in an equation if needed
        if content.can::<dyn LayoutMath>() && !content.is::<EquationElem>() {
            content = self
                .arenas
                .store(EquationElem::new(content.clone()).pack().spanned(content.span()));
        }

        // Styled elements and sequences can (at least currently) also have
        // labels, so this needs to happen before they are handled.
        if let Some(realized) = process(self.engine, self.locator, content, styles)? {
            self.engine.route.increase();
            if !self.engine.route.within(Route::MAX_SHOW_RULE_DEPTH) {
                bail!(
                    content.span(), "maximum show rule depth exceeded";
                    hint: "check whether the show rule matches its own output"
                );
            }
            let result = self.accept(self.arenas.store(realized), styles);
            self.engine.route.decrease();
            return result;
        }

        if let Some(styled) = content.to_packed::<StyledElem>() {
            return self.styled(styled, styles);
        }

        if let Some(sequence) = content.to_packed::<SequenceElem>() {
            for elem in &sequence.children {
                self.accept(elem, styles)?;
            }
            return Ok(());
        }

        // Try to merge `content` with an element under construction

        if self.cites.accept(content, styles) {
            return Ok(());
        }

        self.interrupt_cites()?;

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

        if self.flow.accept(self.arenas, content, styles) {
            return Ok(());
        }

        let keep = content
            .to_packed::<PagebreakElem>()
            .is_some_and(|pagebreak| !pagebreak.weak(styles));

        self.interrupt_page(keep.then_some(styles), false)?;

        if let Some(doc) = &mut self.doc {
            if doc.accept(self.arenas, content, styles) {
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
        styled: &'a StyledElem,
        styles: StyleChain<'a>,
    ) -> SourceResult<()> {
        let local = &styled.styles;
        let stored = self.arenas.store(styles);
        let styles = stored.chain(local);

        if let Some(Some(span)) = local.interruption::<DocumentElem>() {
            let Some(doc) = &mut self.doc else {
                bail!(span, "document set rules are not allowed inside of containers");
            };
            doc.info.populate(local);
        }

        self.interrupt_style(local, None)?;
        self.accept(&styled.child, styles)?;
        self.interrupt_style(local, Some(styles))?;

        Ok(())
    }

    fn interrupt_style(
        &mut self,
        local: &Styles,
        outer: Option<StyleChain<'a>>,
    ) -> SourceResult<()> {
        if let Some(Some(span)) = local.interruption::<PageElem>() {
            if self.doc.is_none() {
                bail!(span, "page configuration is not allowed inside of containers");
            }
            self.interrupt_page(outer, false)?;
        }
        if local.interruption::<ParElem>().is_some()
            || local.interruption::<AlignElem>().is_some()
        {
            self.interrupt_par()?;
        }
        if local.interruption::<ListElem>().is_some()
            || local.interruption::<EnumElem>().is_some()
            || local.interruption::<TermsElem>().is_some()
        {
            self.interrupt_list()?;
        }
        Ok(())
    }

    /// Interrupts citation grouping and adds the resulting citation group to the builder.
    fn interrupt_cites(&mut self) -> SourceResult<()> {
        if !self.cites.items.is_empty() {
            let staged = mem::take(&mut self.cites.staged);
            let (group, styles) = mem::take(&mut self.cites).finish();
            self.accept(self.arenas.store(group.pack()), styles)?;
            for (content, styles) in staged {
                self.accept(content, styles)?;
            }
        }
        Ok(())
    }

    /// Interrupts list building and adds the resulting list element to the builder.
    fn interrupt_list(&mut self) -> SourceResult<()> {
        self.interrupt_cites()?;
        if !self.list.items.is_empty() {
            let staged = mem::take(&mut self.list.staged);
            let (list, styles) = mem::take(&mut self.list).finish();
            self.accept(self.arenas.store(list), styles)?;
            for (content, styles) in staged {
                self.accept(content, styles)?;
            }
        }
        Ok(())
    }

    /// Interrupts paragraph building and adds the resulting paragraph element to the builder.
    fn interrupt_par(&mut self) -> SourceResult<()> {
        self.interrupt_list()?;
        if !self.par.0.is_empty() {
            let (par, styles) = mem::take(&mut self.par).finish();
            self.accept(self.arenas.store(par.pack()), styles)?;
        }

        Ok(())
    }

    /// Interrupts page building and adds the resulting page element to the builder.
    fn interrupt_page(
        &mut self,
        styles: Option<StyleChain<'a>>,
        last: bool,
    ) -> SourceResult<()> {
        self.interrupt_par()?;
        let Some(doc) = &mut self.doc else { return Ok(()) };
        if (doc.keep_next && styles.is_some()) || self.flow.0.has_strong_elements(last) {
            let (flow, trunk) = mem::take(&mut self.flow).finish();
            let span = flow.span();
            let styles = if trunk == StyleChain::default() {
                styles.unwrap_or_default()
            } else {
                trunk
            };
            let page = PageElem::new(flow.pack()).pack().spanned(span);
            self.accept(self.arenas.store(page), styles)?;
        }
        Ok(())
    }
}

/// Builds a [document][DocumentElem] from pagebreaks and pages.
struct DocBuilder<'a> {
    /// The page runs built so far.
    pages: BehavedBuilder<'a>,
    /// Whether to keep a following page even if it is empty.
    keep_next: bool,
    /// Whether the next page should be cleared to an even or odd number.
    clear_next: Option<Parity>,
    /// Details about the document.
    info: DocumentInfo,
}

impl<'a> DocBuilder<'a> {
    /// Tries to accept a piece of content.
    ///
    /// Returns true if this content could be merged into the document.
    /// If this function returns false, then the
    /// content could not be merged, and document building should be
    /// interrupted so that the content can be added elsewhere.
    fn accept(
        &mut self,
        arenas: &'a Arenas<'a>,
        content: &'a Content,
        styles: StyleChain<'a>,
    ) -> bool {
        if let Some(pagebreak) = content.to_packed::<PagebreakElem>() {
            self.keep_next = !pagebreak.weak(styles);
            self.clear_next = pagebreak.to(styles);
            return true;
        }

        if let Some(page) = content.to_packed::<PageElem>() {
            let elem = if let Some(clear_to) = self.clear_next.take() {
                let mut page = page.clone();
                page.push_clear_to(Some(clear_to));
                arenas.store(page.pack())
            } else {
                content
            };

            self.pages.push(elem, styles);
            self.keep_next = false;
            return true;
        }

        false
    }

    /// Turns this builder into the resulting page runs, along with
    /// its [style chain][StyleChain].
    fn finish(self) -> (StyleVec, StyleChain<'a>, DocumentInfo) {
        let buf = self.pages.finish();
        let (children, trunk) = StyleVec::create(&buf);
        (children, trunk, self.info)
    }
}

impl Default for DocBuilder<'_> {
    fn default() -> Self {
        Self {
            pages: BehavedBuilder::new(),
            keep_next: true,
            clear_next: None,
            info: DocumentInfo::default(),
        }
    }
}

/// Builds a [flow][FlowElem] from flow content.
#[derive(Default)]
struct FlowBuilder<'a>(BehavedBuilder<'a>, bool);

impl<'a> FlowBuilder<'a> {
    /// Tries to accept a piece of content.
    ///
    /// Returns true if this content could be merged into the flow.
    /// If this function returns false, then the
    /// content could not be merged, and flow building should be
    /// interrupted so that the content can be added elsewhere.
    fn accept(
        &mut self,
        arenas: &'a Arenas<'a>,
        content: &'a Content,
        styles: StyleChain<'a>,
    ) -> bool {
        let last_was_par = self.1;
        self.1 = false;

        if content.is::<ParbreakElem>() {
            return true;
        }

        if let Some(elem) = content.to_packed::<VElem>() {
            if !elem.attach(styles) || last_was_par {
                self.0.push(content, styles);
            }
            return true;
        }

        if content.is::<ColbreakElem>()
            || content.is::<TagElem>()
            || content.is::<PlaceElem>()
            || content.is::<FlushElem>()
        {
            self.0.push(content, styles);
            return true;
        }

        let par_spacing = Lazy::new(|| {
            arenas.store(VElem::par_spacing(ParElem::spacing_in(styles).into()).pack())
        });

        if let Some(elem) = content.to_packed::<BlockElem>() {
            let above = match elem.above(styles) {
                Smart::Auto => *par_spacing,
                Smart::Custom(above) => arenas.store(VElem::block_spacing(above).pack()),
            };

            let below = match elem.below(styles) {
                Smart::Auto => *par_spacing,
                Smart::Custom(below) => arenas.store(VElem::block_spacing(below).pack()),
            };

            self.0.push(above, styles);
            self.0.push(content, styles);
            self.0.push(below, styles);
            return true;
        }

        if content.is::<ParElem>() {
            self.0.push(*par_spacing, styles);
            self.0.push(content, styles);
            self.0.push(*par_spacing, styles);
            self.1 = true;
            return true;
        }

        false
    }

    /// Turns this builder into the resulting flow, along with
    /// its [style chain][StyleChain].
    fn finish(self) -> (Packed<FlowElem>, StyleChain<'a>) {
        let buf = self.0.finish();
        let span = determine_span(&buf);
        let (children, trunk) = StyleVec::create(&buf);
        (Packed::new(FlowElem::new(children)).spanned(span), trunk)
    }
}

/// Builds a [paragraph][ParElem] from paragraph content.
#[derive(Default)]
struct ParBuilder<'a>(BehavedBuilder<'a>);

impl<'a> ParBuilder<'a> {
    /// Tries to accept a piece of content.
    ///
    /// Returns true if this content could be merged into the paragraph.
    /// If this function returns false, then the
    /// content could not be merged, and paragraph building should be
    /// interrupted so that the content can be added elsewhere.
    fn accept(&mut self, content: &'a Content, styles: StyleChain<'a>) -> bool {
        if content.is::<TagElem>() {
            if !self.0.is_empty() {
                self.0.push(content, styles);
                return true;
            }
        } else if content.is::<SpaceElem>()
            || content.is::<TextElem>()
            || content.is::<HElem>()
            || content.is::<LinebreakElem>()
            || content.is::<SmartQuoteElem>()
            || content.is::<InlineElem>()
            || content.is::<BoxElem>()
        {
            self.0.push(content, styles);
            return true;
        }

        false
    }

    /// Turns this builder into the resulting paragraph, along with
    /// its [style chain][StyleChain].
    fn finish(self) -> (Packed<ParElem>, StyleChain<'a>) {
        let buf = self.0.finish();
        let span = determine_span(&buf);
        let (children, trunk) = StyleVec::create(&buf);
        (Packed::new(ParElem::new(children)).spanned(span), trunk)
    }
}

/// Builds a list (either [`ListElem`], [`EnumElem`], or [`TermsElem`])
/// from list or enum items, spaces, and paragraph breaks.
struct ListBuilder<'a> {
    /// The list items collected so far.
    items: Vec<Pair<'a>>,
    /// Trailing content for which it is unclear whether it is part of the list.
    staged: Vec<Pair<'a>>,
    /// Whether the list contains no paragraph breaks.
    tight: bool,
}

impl<'a> ListBuilder<'a> {
    /// Tries to accept a piece of content.
    ///
    /// Returns true if this content could be merged into the list.
    /// If this function returns false, then the
    /// content could not be merged, and list building should be
    /// interrupted so that the content can be added elsewhere.
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
                .first()
                .map_or(true, |(first, _)| first.func() == content.func())
        {
            self.items.push((content, styles));
            self.tight &= self.staged.drain(..).all(|(t, _)| !t.is::<ParbreakElem>());
            return true;
        }

        false
    }

    /// Turns this builder into the resulting list, along with
    /// its [style chain][StyleChain].
    fn finish(self) -> (Content, StyleChain<'a>) {
        let span = determine_span(&self.items);
        let (children, trunk) = StyleVec::create(&self.items);

        let mut iter = children.into_iter().peekable();
        let (first, _) = iter.peek().unwrap();
        let output = if first.is::<ListItem>() {
            let children = iter
                .map(|(item, local)| {
                    item.into_packed::<ListItem>().unwrap().styled(local)
                })
                .collect();
            ListElem::new(children).with_tight(self.tight).pack().spanned(span)
        } else if first.is::<EnumItem>() {
            let children = iter
                .map(|(item, local)| {
                    item.into_packed::<EnumItem>().unwrap().styled(local)
                })
                .collect();
            EnumElem::new(children).with_tight(self.tight).pack().spanned(span)
        } else if first.is::<TermItem>() {
            let children = iter
                .map(|(item, local)| {
                    item.into_packed::<TermItem>().unwrap().styled(local)
                })
                .collect();
            TermsElem::new(children).with_tight(self.tight).pack().spanned(span)
        } else {
            unreachable!()
        };

        (output, trunk)
    }
}

impl Default for ListBuilder<'_> {
    fn default() -> Self {
        Self { items: vec![], staged: vec![], tight: true }
    }
}

/// Builds a [citation group][CiteGroup] from citations.
#[derive(Default)]
struct CiteGroupBuilder<'a> {
    /// The citations.
    items: Vec<Packed<CiteElem>>,
    /// Trailing content for which it is unclear whether it is part of the list.
    staged: Vec<Pair<'a>>,
    /// The styles.
    styles: StyleChain<'a>,
}

impl<'a> CiteGroupBuilder<'a> {
    /// Tries to accept a piece of content.
    ///
    /// Returns true if this content could be merged into the citation
    /// group. If this function returns false, then the
    /// content could not be merged, and citation grouping should be
    /// interrupted so that the content can be added elsewhere.
    fn accept(&mut self, content: &'a Content, styles: StyleChain<'a>) -> bool {
        if !self.items.is_empty()
            && (content.is::<SpaceElem>() || content.is::<TagElem>())
        {
            self.staged.push((content, styles));
            return true;
        }

        if let Some(citation) = content.to_packed::<CiteElem>() {
            if self.items.is_empty() {
                self.styles = styles;
            }
            self.staged.retain(|(elem, _)| !elem.is::<SpaceElem>());
            self.items.push(citation.clone());
            return true;
        }

        false
    }

    /// Turns this builder into the resulting citation group, along with
    /// its [style chain][StyleChain].
    fn finish(self) -> (Packed<CiteGroup>, StyleChain<'a>) {
        let span = self.items.first().map(|cite| cite.span()).unwrap_or(Span::detached());
        (Packed::new(CiteGroup::new(self.items)).spanned(span), self.styles)
    }
}

/// Determine a span for the built collection.
fn determine_span(buf: &[(&Content, StyleChain)]) -> Span {
    let mut span = Span::detached();
    for &(content, _) in buf {
        span = content.span();
        if !span.is_detached() {
            break;
        }
    }
    span
}
