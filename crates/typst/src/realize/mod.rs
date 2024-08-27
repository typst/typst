//! Realization of content.
//!
//! *Realization* is the process of recursively applying styling and, in
//! particular, show rules to produce well-known elements that can be laid out.

mod arenas;
mod behaviour;
mod process;

use once_cell::unsync::Lazy;

pub use self::arenas::Arenas;
pub use self::behaviour::{Behave, BehavedBuilder, Behaviour};
pub use self::process::process;

use std::mem;

use crate::diag::{bail, At, SourceResult};
use crate::engine::Engine;
use crate::foundations::{
    Content, ContextElem, NativeElement, Packed, SequenceElem, Smart, StyleChain,
    StyleVec, StyledElem, Styles,
};
use crate::introspection::{SplitLocator, TagElem, TagKind};
use crate::layout::{
    AlignElem, BlockElem, BoxElem, ColbreakElem, FlushElem, HElem, InlineElem, PageElem,
    PagebreakElem, PlaceElem, VElem,
};
use crate::math::{EquationElem, LayoutMath};
use crate::model::{
    CiteElem, CiteGroup, DocumentElem, DocumentInfo, EnumElem, EnumItem, ListElem,
    ListItem, ParElem, ParbreakElem, TermItem, TermsElem,
};
use crate::syntax::Span;
use crate::text::{LinebreakElem, SmartQuoteElem, SpaceElem, TextElem};
use crate::utils::SliceExt;

/// A pair of content and a style chain that applies to it.
pub type Pair<'a> = (&'a Content, StyleChain<'a>);

/// Realize at the root level.
#[typst_macros::time(name = "realize")]
pub fn realize_root<'a>(
    engine: &mut Engine<'a>,
    locator: &mut SplitLocator<'a>,
    arenas: &'a Arenas<'a>,
    content: &'a Content,
    styles: StyleChain<'a>,
) -> SourceResult<(Vec<Pair<'a>>, DocumentInfo)> {
    let mut builder = Builder::new(engine, locator, arenas, true);
    builder.accept(content, styles)?;
    builder.interrupt_par()?;
    Ok((builder.sink.finish(), builder.doc_info.unwrap()))
}

/// Realize at the container level.
#[typst_macros::time(name = "realize")]
pub fn realizer_container<'a>(
    engine: &mut Engine<'a>,
    locator: &mut SplitLocator<'a>,
    arenas: &'a Arenas<'a>,
    content: &'a Content,
    styles: StyleChain<'a>,
) -> SourceResult<Vec<Pair<'a>>> {
    let mut builder = Builder::new(engine, locator, arenas, false);
    builder.accept(content, styles)?;
    builder.interrupt_par()?;
    Ok(builder.sink.finish())
}

/// Realizes content into a flat list of well-known, styled elements.
struct Builder<'a, 'v> {
    /// The engine.
    engine: &'v mut Engine<'a>,
    /// Assigns unique locations to elements.
    locator: &'v mut SplitLocator<'a>,
    /// Scratch arenas for building.
    arenas: &'a Arenas<'a>,

    /// The output elements of well-known types collected by the builder.
    sink: BehavedBuilder<'a>,
    /// Document metadata we have collected from `set document` rules. If this
    /// is `None`, we are in a container.
    doc_info: Option<DocumentInfo>,

    /// A builder for a paragraph that might be under construction.
    par: ParBuilder<'a>,
    /// A builder for a list that might be under construction.
    list: ListBuilder<'a>,
    /// A builder for a citation group that might be under construction.
    cites: CiteGroupBuilder<'a>,

    /// Whether we are currently not within any container or show rule output.
    /// This is used to determine page styles during layout.
    outside: bool,
    /// Whether the last item that we visited was a paragraph (with no parbreak
    /// in between). This is used for attach spacing.
    last_was_par: bool,
}

impl<'a, 'v> Builder<'a, 'v> {
    /// Creates a new builder.
    fn new(
        engine: &'v mut Engine<'a>,
        locator: &'v mut SplitLocator<'a>,
        arenas: &'a Arenas<'a>,
        root: bool,
    ) -> Self {
        Self {
            engine,
            locator,
            arenas,
            sink: BehavedBuilder::default(),
            doc_info: root.then(DocumentInfo::default),
            par: ParBuilder::default(),
            list: ListBuilder::default(),
            cites: CiteGroupBuilder::default(),
            outside: root,
            last_was_par: false,
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
        if let Some((tag, realized)) =
            process(self.engine, self.locator, content, styles)?
        {
            self.engine.route.increase();
            self.engine.route.check_show_depth().at(content.span())?;

            if let Some(tag) = &tag {
                self.accept(self.arenas.store(TagElem::packed(tag.clone())), styles)?;
            }

            let prev_outside = self.outside;
            self.outside &= content.is::<ContextElem>();
            self.accept(self.arenas.store(realized), styles)?;
            self.outside = prev_outside;

            if let Some(tag) = tag {
                let end = tag.with_kind(TagKind::End);
                self.accept(self.arenas.store(TagElem::packed(end)), styles)?;
            }

            self.engine.route.decrease();
            return Ok(());
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
        // (cite group, list, or par).

        if self.cites.accept(content, styles) {
            return Ok(());
        }

        self.interrupt_cites()?;

        if self.list.accept(content, styles) {
            return Ok(());
        }

        self.interrupt_list()?;

        // Try again because it could be another kind of list.
        if self.list.accept(content, styles) {
            return Ok(());
        }

        if self.par.accept(content, styles) {
            return Ok(());
        }

        self.interrupt_par()?;

        self.save(content, styles)
    }

    /// Tries to save a piece of content into the sink.
    fn save(&mut self, content: &'a Content, styles: StyleChain<'a>) -> SourceResult<()> {
        let last_was_par = std::mem::replace(&mut self.last_was_par, false);
        let par_spacing = Lazy::new(|| {
            self.arenas
                .store(VElem::par_spacing(ParElem::spacing_in(styles).into()).pack())
        });

        if content.is::<TagElem>()
            || content.is::<PlaceElem>()
            || content.is::<FlushElem>()
            || content.is::<ColbreakElem>()
        {
            self.sink.push(content, styles);
        } else if content.is::<PagebreakElem>() {
            if self.doc_info.is_none() {
                bail!(content.span(), "pagebreaks are not allowed inside of containers");
            }
            self.sink.push(content, styles);
        } else if let Some(elem) = content.to_packed::<VElem>() {
            if !elem.attach(styles) || last_was_par {
                self.sink.push(content, styles);
            }
        } else if content.is::<ParbreakElem>() {
            // It's only a boundary, so we can ignore it.
        } else if content.is::<ParElem>() {
            self.sink.push(*par_spacing, styles);
            self.sink.push(content, styles);
            self.sink.push(*par_spacing, styles);
            self.last_was_par = true;
        } else if let Some(elem) = content.to_packed::<BlockElem>() {
            let above = match elem.above(styles) {
                Smart::Auto => *par_spacing,
                Smart::Custom(above) => {
                    self.arenas.store(VElem::block_spacing(above).pack())
                }
            };

            let below = match elem.below(styles) {
                Smart::Auto => *par_spacing,
                Smart::Custom(below) => {
                    self.arenas.store(VElem::block_spacing(below).pack())
                }
            };

            self.sink.push(above, styles);
            self.sink.push(content, styles);
            self.sink.push(below, styles);
        } else {
            bail!(content.span(), "{} is not allowed here", content.func().name());
        }

        Ok(())
    }

    /// Handles a styled element.
    fn styled(
        &mut self,
        styled: &'a StyledElem,
        styles: StyleChain<'a>,
    ) -> SourceResult<()> {
        if let Some(span) = styled.styles.interruption::<DocumentElem>() {
            let Some(info) = &mut self.doc_info else {
                bail!(span, "document set rules are not allowed inside of containers");
            };
            info.populate(&styled.styles);
        }

        let page_interruption = styled.styles.interruption::<PageElem>();
        if let Some(span) = page_interruption {
            if self.doc_info.is_none() {
                bail!(span, "page configuration is not allowed inside of containers");
            }

            // When there are page styles, we "break free" from our show rule
            // cage.
            self.outside = true;
        }

        // If we are not within a container or show rule, mark the styles as
        // "outside". This will allow them to be lifted to the page level.
        let outer = self.arenas.store(styles);
        let local = if self.outside {
            self.arenas.store(styled.styles.clone().outside())
        } else {
            &styled.styles
        };

        if page_interruption.is_some() {
            // For the starting pagebreak we only want the styles before and
            // including the interruptions, not trailing styles that happen to
            // be in the same `Styles` list.
            let relevant = local
                .as_slice()
                .trim_end_matches(|style| style.interruption::<PageElem>().is_none());
            self.accept(PagebreakElem::shared_weak(), outer.chain(relevant))?;
        }

        self.interrupt_styles(local)?;
        self.accept(&styled.child, outer.chain(local))?;
        self.interrupt_styles(local)?;

        if page_interruption.is_some() {
            // For the ending pagebreak, the styles don't really matter because
            // the styles of a "boundary" pagebreak are ignored during layout.
            self.accept(PagebreakElem::shared_boundary(), *outer)?;
        }

        Ok(())
    }

    /// Inspects the styles and dispatches to the different interruption
    /// handlers.
    fn interrupt_styles(&mut self, local: &Styles) -> SourceResult<()> {
        if local.interruption::<ParElem>().is_some()
            || local.interruption::<AlignElem>().is_some()
        {
            self.interrupt_par()?;
        } else if local.interruption::<ListElem>().is_some()
            || local.interruption::<EnumElem>().is_some()
            || local.interruption::<TermsElem>().is_some()
        {
            self.interrupt_list()?;
        } else if local.interruption::<CiteElem>().is_some() {
            self.interrupt_cites()?;
        }
        Ok(())
    }

    /// Interrupts paragraph building and adds the resulting paragraph element
    /// to the builder.
    fn interrupt_par(&mut self) -> SourceResult<()> {
        self.interrupt_list()?;
        if !self.par.0.is_empty() {
            mem::take(&mut self.par).finish(self)?;
        }
        Ok(())
    }

    /// Interrupts list building and adds the resulting list element to the
    /// builder.
    fn interrupt_list(&mut self) -> SourceResult<()> {
        self.interrupt_cites()?;
        if !self.list.0.is_empty() {
            mem::take(&mut self.list).finish(self)?;
        }
        Ok(())
    }

    /// Interrupts citation grouping and adds the resulting citation group to
    /// the builder.
    fn interrupt_cites(&mut self) -> SourceResult<()> {
        if !self.cites.0.is_empty() {
            mem::take(&mut self.cites).finish(self)?;
        }
        Ok(())
    }
}

/// Builds a [paragraph][ParElem] from paragraph content.
#[derive(Default)]
struct ParBuilder<'a>(BehavedBuilder<'a>);

impl<'a> ParBuilder<'a> {
    /// Tries to accept a piece of content.
    ///
    /// Returns true if this content could be merged into the paragraph. If this
    /// function returns false, then the content could not be merged, and
    /// paragraph building should be interrupted so that the content can be
    /// added elsewhere.
    fn accept(&mut self, content: &'a Content, styles: StyleChain<'a>) -> bool {
        if Self::is_primary(content) || (!self.0.is_empty() && Self::is_inner(content)) {
            self.0.push(content, styles);
            return true;
        }

        false
    }

    /// Whether this content is of interest to the builder.
    fn is_primary(content: &'a Content) -> bool {
        content.is::<SpaceElem>()
            || content.is::<TextElem>()
            || content.is::<HElem>()
            || content.is::<LinebreakElem>()
            || content.is::<SmartQuoteElem>()
            || content.is::<InlineElem>()
            || content.is::<BoxElem>()
    }

    /// Whether this content can merely exist in between interesting items.
    fn is_inner(content: &'a Content) -> bool {
        content.is::<TagElem>()
    }

    /// Turns this builder into the resulting list, along with
    /// its [style chain][StyleChain].
    fn finish(self, builder: &mut Builder<'a, '_>) -> SourceResult<()> {
        let buf = self.0.finish();
        let trimmed = buf.trim_end_matches(|(c, _)| c.is::<TagElem>());
        let staged = &buf[trimmed.len()..];

        let span = first_span(trimmed);
        let (children, trunk) = StyleVec::create(trimmed);
        let elem = Packed::new(ParElem::new(children)).spanned(span);
        builder.accept(builder.arenas.store(elem.pack()), trunk)?;

        for &(tag, styles) in staged {
            builder.accept(tag, styles)?;
        }

        Ok(())
    }
}

/// Builds a list (either [`ListElem`], [`EnumElem`], or [`TermsElem`]) from
/// list or enum items, spaces, and paragraph breaks.
#[derive(Default)]
struct ListBuilder<'a>(Vec<Pair<'a>>);

impl<'a> ListBuilder<'a> {
    /// Tries to accept a piece of content.
    ///
    /// Returns true if this content could be merged into the list. If this
    /// function returns false, then the content could not be merged, and list
    /// building should be interrupted so that the content can be added
    /// elsewhere.
    fn accept(&mut self, content: &'a Content, styles: StyleChain<'a>) -> bool {
        if (Self::is_primary(content) && self.is_compatible(content))
            || (!self.0.is_empty() && Self::is_inner(content))
        {
            self.0.push((content, styles));
            return true;
        }

        false
    }

    /// Whether this content is of interest to the builder.
    fn is_primary(content: &'a Content) -> bool {
        content.is::<ListItem>() || content.is::<EnumItem>() || content.is::<TermItem>()
    }

    /// Whether this content can merely exist in between interesting items.
    fn is_inner(content: &'a Content) -> bool {
        content.is::<TagElem>()
            || content.is::<SpaceElem>()
            || content.is::<ParbreakElem>()
    }

    /// Whether this kind of list items is compatible with the builder's type.
    fn is_compatible(&self, content: &'a Content) -> bool {
        self.0
            .first()
            .map_or(true, |(first, _)| first.func() == content.func())
    }

    /// Turns this builder into the resulting list, along with
    /// its [style chain][StyleChain].
    fn finish(self, builder: &mut Builder<'a, '_>) -> SourceResult<()> {
        let trimmed = self.0.trim_end_matches(|(c, _)| Self::is_inner(c));
        let tags = trimmed.iter().filter(|(c, _)| c.is::<TagElem>());
        let staged = &self.0[trimmed.len()..];
        let items = trimmed.iter().copied().filter(|(c, _)| Self::is_primary(c));
        let first = items.clone().next().unwrap().0;
        let tight = !trimmed.iter().any(|(c, _)| c.is::<ParbreakElem>());

        // Determine the styles that are shared by all items. These will be
        // used for the list itself.
        let trunk = StyleChain::trunk(items.clone().map(|(_, s)| s)).unwrap();
        let depth = trunk.links().count();

        // Builder the correct element.
        let iter = items.map(|(c, s)| (c, s.suffix(depth)));
        let elem = if first.is::<ListItem>() {
            let children = iter
                .map(|(item, local)| {
                    item.to_packed::<ListItem>().unwrap().clone().styled(local)
                })
                .collect();
            ListElem::new(children).with_tight(tight).pack()
        } else if first.is::<EnumItem>() {
            let children = iter
                .map(|(item, local)| {
                    item.to_packed::<EnumItem>().unwrap().clone().styled(local)
                })
                .collect();
            EnumElem::new(children).with_tight(tight).pack()
        } else if first.is::<TermItem>() {
            let children = iter
                .map(|(item, local)| {
                    item.to_packed::<TermItem>().unwrap().clone().styled(local)
                })
                .collect();
            TermsElem::new(children).with_tight(tight).pack()
        } else {
            unreachable!()
        };

        // Add the list to the builder.
        let span = first_span(&self.0);
        let stored = builder.arenas.store(elem.spanned(span));
        builder.accept(stored, trunk)?;

        // Add the tags and staged elements to the builder.
        for &(content, styles) in tags.chain(staged) {
            builder.accept(content, styles)?;
        }

        Ok(())
    }
}

/// Builds a [citation group][CiteGroup] from citations.
#[derive(Default)]
struct CiteGroupBuilder<'a>(Vec<Pair<'a>>);

impl<'a> CiteGroupBuilder<'a> {
    /// Tries to accept a piece of content.
    ///
    /// Returns true if this content could be merged into the citation group. If
    /// this function returns false, then the content could not be merged, and
    /// citation grouping should be interrupted so that the content can be added
    /// elsewhere.
    fn accept(&mut self, content: &'a Content, styles: StyleChain<'a>) -> bool {
        if Self::is_primary(content) || (!self.0.is_empty() && Self::is_inner(content)) {
            self.0.push((content, styles));
            return true;
        }

        false
    }

    /// Whether this content is of interest to the builder.
    fn is_primary(content: &'a Content) -> bool {
        content.is::<CiteElem>()
    }

    /// Whether this content can merely exist in between interesting items.
    fn is_inner(content: &'a Content) -> bool {
        content.is::<TagElem>() || content.is::<SpaceElem>()
    }

    /// Turns this builder into the resulting citation group, along with
    /// its [style chain][StyleChain].
    fn finish(self, builder: &mut Builder<'a, '_>) -> SourceResult<()> {
        let trimmed = self.0.trim_end_matches(|(c, _)| Self::is_inner(c));
        let tags = trimmed.iter().filter(|(c, _)| c.is::<TagElem>());
        let staged = &self.0[trimmed.len()..];
        let trunk = trimmed[0].1;
        let children = trimmed
            .iter()
            .filter_map(|(c, _)| c.to_packed::<CiteElem>())
            .cloned()
            .collect();

        // Add the citation group to the builder.
        let span = first_span(&self.0);
        let elem = CiteGroup::new(children).pack();
        let stored = builder.arenas.store(elem.spanned(span));
        builder.accept(stored, trunk)?;

        // Add the tags and staged elements to the builder.
        for &(content, styles) in tags.chain(staged) {
            builder.accept(content, styles)?;
        }

        Ok(())
    }
}

/// Determine a span for the built collection.
pub fn first_span(children: &[(&Content, StyleChain)]) -> Span {
    children
        .iter()
        .map(|(c, _)| c.span())
        .find(|span| !span.is_detached())
        .unwrap_or(Span::detached())
}
