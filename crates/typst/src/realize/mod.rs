//! Realization of content.

mod arenas;
mod behaviour;
mod process;

pub use self::arenas::Arenas;
pub use self::behaviour::{Behave, BehavedBuilder, Behaviour};
pub use self::process::{process, processable};

use std::borrow::Cow;

use std::mem;

use crate::diag::{bail, SourceResult};
use crate::engine::{Engine, Route};
use crate::foundations::{
    Content, NativeElement, Packed, SequenceElem, StyleChain, StyledElem, Styles,
};
use crate::introspection::MetaElem;
use crate::layout::{
    AlignElem, BlockElem, BoxElem, ColbreakElem, FlowElem, HElem, LayoutMultiple,
    LayoutSingle, PageElem, PagebreakElem, Parity, PlaceElem, VElem,
};
use crate::math::{EquationElem, LayoutMath};
use crate::model::{
    CiteElem, CiteGroup, DocumentElem, EnumElem, EnumItem, ListElem, ListItem, ParElem,
    ParbreakElem, TermItem, TermsElem,
};
use crate::syntax::Span;
use crate::text::{LinebreakElem, SmartQuoteElem, SpaceElem, TextElem};

/// Realize into an element that is capable of root-level layout.
#[typst_macros::time(name = "realize root")]
pub fn realize_root<'a>(
    engine: &mut Engine,
    arenas: &'a Arenas<'a>,
    content: &'a Content,
    styles: StyleChain<'a>,
) -> SourceResult<(Packed<DocumentElem>, StyleChain<'a>)> {
    let mut builder = Builder::new(engine, arenas, true);
    builder.accept(content, styles)?;
    builder.interrupt_page(Some(styles), true)?;
    let (doc, trunk) = builder.doc.unwrap().finish();
    Ok((doc, trunk))
}

/// Realize into an element that is capable of block-level layout.
#[typst_macros::time(name = "realize block")]
pub fn realize_block<'a>(
    engine: &mut Engine,
    arenas: &'a Arenas<'a>,
    content: &'a Content,
    styles: StyleChain<'a>,
) -> SourceResult<(Cow<'a, Content>, StyleChain<'a>)> {
    // These elements implement `Layout` but still require a flow for
    // proper layout.
    if content.can::<dyn LayoutMultiple>() && !processable(engine, content, styles) {
        return Ok((Cow::Borrowed(content), styles));
    }

    let mut builder = Builder::new(engine, arenas, false);
    builder.accept(content, styles)?;
    builder.interrupt_par()?;

    let (flow, trunk) = builder.flow.finish();
    Ok((Cow::Owned(flow.pack()), trunk))
}

/// Builds a document or a flow element from content.
struct Builder<'a, 'v, 't> {
    /// The engine.
    engine: &'v mut Engine<'t>,
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

impl<'a, 'v, 't> Builder<'a, 'v, 't> {
    fn new(engine: &'v mut Engine<'t>, arenas: &'a Arenas<'a>, top: bool) -> Self {
        Self {
            engine,
            arenas,
            doc: top.then(DocBuilder::default),
            flow: FlowBuilder::default(),
            par: ParBuilder::default(),
            list: ListBuilder::default(),
            cites: CiteGroupBuilder::default(),
        }
    }

    fn accept(
        &mut self,
        mut content: &'a Content,
        styles: StyleChain<'a>,
    ) -> SourceResult<()> {
        if content.can::<dyn LayoutMath>() && !content.is::<EquationElem>() {
            content = self
                .arenas
                .store(EquationElem::new(content.clone()).pack().spanned(content.span()));
        }

        if let Some(realized) = process(self.engine, content, styles)? {
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
        let stored = self.arenas.store(styles);
        let styles = stored.chain(&styled.styles);
        self.interrupt_style(&styled.styles, None)?;
        self.accept(&styled.child, styles)?;
        self.interrupt_style(&styled.styles, Some(styles))?;
        Ok(())
    }

    fn interrupt_style(
        &mut self,
        local: &Styles,
        outer: Option<StyleChain<'a>>,
    ) -> SourceResult<()> {
        if let Some(Some(span)) = local.interruption::<DocumentElem>() {
            let Some(doc) = &self.doc else {
                bail!(span, "document set rules are not allowed inside of containers");
            };
            if outer.is_none()
                && (!doc.pages.is_empty()
                    || !self.flow.0.is_empty()
                    || !self.par.0.is_empty()
                    || !self.list.items.is_empty()
                    || !self.cites.items.is_empty())
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

    fn interrupt_par(&mut self) -> SourceResult<()> {
        self.interrupt_list()?;
        if !self.par.0.is_empty() {
            let (par, styles) = mem::take(&mut self.par).finish();
            self.accept(self.arenas.store(par.pack()), styles)?;
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

/// Accepts pagebreaks and pages.
struct DocBuilder<'a> {
    /// The page runs built so far.
    pages: BehavedBuilder<'a>,
    /// Whether to keep a following page even if it is empty.
    keep_next: bool,
    /// Whether the next page should be cleared to an even or odd number.
    clear_next: Option<Parity>,
}

impl<'a> DocBuilder<'a> {
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

    fn finish(self) -> (Packed<DocumentElem>, StyleChain<'a>) {
        let (children, trunk, span) = self.pages.finish();
        (Packed::new(DocumentElem::new(children)).spanned(span), trunk)
    }
}

impl Default for DocBuilder<'_> {
    fn default() -> Self {
        Self {
            pages: BehavedBuilder::new(),
            keep_next: true,
            clear_next: None,
        }
    }
}

/// Accepts flow content.
#[derive(Default)]
struct FlowBuilder<'a>(BehavedBuilder<'a>, bool);

impl<'a> FlowBuilder<'a> {
    fn accept(
        &mut self,
        arenas: &'a Arenas<'a>,
        content: &'a Content,
        styles: StyleChain<'a>,
    ) -> bool {
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
            self.0.push(content, styles);
            return true;
        }

        if content.can::<dyn LayoutSingle>()
            || content.can::<dyn LayoutMultiple>()
            || content.is::<ParElem>()
        {
            let is_tight_list = if let Some(elem) = content.to_packed::<ListElem>() {
                elem.tight(styles)
            } else if let Some(elem) = content.to_packed::<EnumElem>() {
                elem.tight(styles)
            } else if let Some(elem) = content.to_packed::<TermsElem>() {
                elem.tight(styles)
            } else {
                false
            };

            if !last_was_parbreak && is_tight_list {
                let leading = ParElem::leading_in(styles);
                let spacing = VElem::list_attach(leading.into());
                self.0.push(arenas.store(spacing.pack()), styles);
            }

            let (above, below) = if let Some(block) = content.to_packed::<BlockElem>() {
                (block.above(styles), block.below(styles))
            } else {
                (BlockElem::above_in(styles), BlockElem::below_in(styles))
            };

            self.0.push(arenas.store(above.pack()), styles);
            self.0.push(content, styles);
            self.0.push(arenas.store(below.pack()), styles);
            return true;
        }

        false
    }

    fn finish(self) -> (Packed<FlowElem>, StyleChain<'a>) {
        let (children, trunk, span) = self.0.finish();
        (Packed::new(FlowElem::new(children)).spanned(span), trunk)
    }
}

/// Accepts paragraph content.
#[derive(Default)]
struct ParBuilder<'a>(BehavedBuilder<'a>);

impl<'a> ParBuilder<'a> {
    fn accept(&mut self, content: &'a Content, styles: StyleChain<'a>) -> bool {
        if content.is::<MetaElem>() {
            if !self.0.is_empty() {
                self.0.push(content, styles);
                return true;
            }
        } else if content.is::<SpaceElem>()
            || content.is::<TextElem>()
            || content.is::<HElem>()
            || content.is::<LinebreakElem>()
            || content.is::<SmartQuoteElem>()
            || content
                .to_packed::<EquationElem>()
                .is_some_and(|elem| !elem.block(styles))
            || content.is::<BoxElem>()
        {
            self.0.push(content, styles);
            return true;
        }

        false
    }

    fn finish(self) -> (Packed<ParElem>, StyleChain<'a>) {
        let (children, trunk, span) = self.0.finish();
        (Packed::new(ParElem::new(children)).spanned(span), trunk)
    }
}

/// Accepts list / enum items, spaces, paragraph breaks.
struct ListBuilder<'a> {
    /// The list items collected so far.
    items: BehavedBuilder<'a>,
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
                .items()
                .next()
                .map_or(true, |first| first.func() == content.func())
        {
            self.items.push(content, styles);
            self.tight &= self.staged.drain(..).all(|(t, _)| !t.is::<ParbreakElem>());
            return true;
        }

        false
    }

    fn finish(self) -> (Content, StyleChain<'a>) {
        let (items, trunk, span) = self.items.finish_iter();
        let mut items = items.peekable();
        let (first, _) = items.peek().unwrap();
        let output = if first.is::<ListItem>() {
            ListElem::new(
                items
                    .map(|(item, local)| {
                        let mut item = item.to_packed::<ListItem>().unwrap().clone();
                        let body = item.body().clone().styled_with_map(local);
                        item.push_body(body);
                        item
                    })
                    .collect(),
            )
            .with_tight(self.tight)
            .pack()
            .spanned(span)
        } else if first.is::<EnumItem>() {
            EnumElem::new(
                items
                    .map(|(item, local)| {
                        let mut item = item.to_packed::<EnumItem>().unwrap().clone();
                        let body = item.body().clone().styled_with_map(local);
                        item.push_body(body);
                        item
                    })
                    .collect(),
            )
            .with_tight(self.tight)
            .pack()
            .spanned(span)
        } else if first.is::<TermItem>() {
            TermsElem::new(
                items
                    .map(|(item, local)| {
                        let mut item = item.to_packed::<TermItem>().unwrap().clone();
                        let term = item.term().clone().styled_with_map(local.clone());
                        let description =
                            item.description().clone().styled_with_map(local);
                        item.push_term(term);
                        item.push_description(description);
                        item
                    })
                    .collect(),
            )
            .with_tight(self.tight)
            .pack()
            .spanned(span)
        } else {
            unreachable!()
        };
        (output, trunk)
    }
}

impl Default for ListBuilder<'_> {
    fn default() -> Self {
        Self {
            items: BehavedBuilder::default(),
            tight: true,
            staged: vec![],
        }
    }
}

/// Accepts citations.
#[derive(Default)]
struct CiteGroupBuilder<'a> {
    /// The styles.
    styles: StyleChain<'a>,
    /// The citations.
    items: Vec<Packed<CiteElem>>,
    /// Trailing content for which it is unclear whether it is part of the list.
    staged: Vec<(&'a Content, StyleChain<'a>)>,
}

impl<'a> CiteGroupBuilder<'a> {
    fn accept(&mut self, content: &'a Content, styles: StyleChain<'a>) -> bool {
        if !self.items.is_empty()
            && (content.is::<SpaceElem>() || content.is::<MetaElem>())
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

    fn finish(self) -> (Packed<CiteGroup>, StyleChain<'a>) {
        let span = self.items.first().map(|cite| cite.span()).unwrap_or(Span::detached());
        (Packed::new(CiteGroup::new(self.items)).spanned(span), self.styles)
    }
}
