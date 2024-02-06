//! Realization of content.

mod behave;

pub use self::behave::BehavedBuilder;

use std::borrow::Cow;
use std::cell::OnceCell;
use std::mem;

use smallvec::smallvec;
use typed_arena::Arena;

use crate::diag::{bail, SourceResult};
use crate::engine::{Engine, Route};
use crate::foundations::{
    Behave, Behaviour, Content, NativeElement, Packed, Recipe, RecipeIndex, Regex,
    Selector, Show, ShowSet, Style, StyleChain, StyleVec, StyleVecBuilder, Styles,
    Synthesize, Transformation,
};
use crate::introspection::{Locatable, Meta, MetaElem};
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
use crate::util::{hash128, BitSet};

/// Realize into an element that is capable of root-level layout.
#[typst_macros::time(name = "realize root")]
pub fn realize_root<'a>(
    engine: &mut Engine,
    scratch: &'a Scratch<'a>,
    content: &'a Content,
    styles: StyleChain<'a>,
) -> SourceResult<(Packed<DocumentElem>, StyleChain<'a>)> {
    let mut builder = Builder::new(engine, scratch, true);
    builder.accept(content, styles)?;
    builder.interrupt_page(Some(styles), true)?;
    let (pages, shared) = builder.doc.unwrap().pages.finish();
    let span = first_span(&pages);
    Ok((Packed::new(DocumentElem::new(pages.to_vec())).spanned(span), shared))
}

/// Realize into an element that is capable of block-level layout.
#[typst_macros::time(name = "realize block")]
pub fn realize_block<'a>(
    engine: &mut Engine,
    scratch: &'a Scratch<'a>,
    content: &'a Content,
    styles: StyleChain<'a>,
) -> SourceResult<(Cow<'a, Content>, StyleChain<'a>)> {
    // These elements implement `Layout` but still require a flow for
    // proper layout.
    if content.can::<dyn LayoutMultiple>() && verdict(engine, content, styles).is_none() {
        return Ok((Cow::Borrowed(content), styles));
    }

    let mut builder = Builder::new(engine, scratch, false);
    builder.accept(content, styles)?;
    builder.interrupt_par()?;

    let (children, shared) = builder.flow.0.finish();
    let span = first_span(&children);
    Ok((Cow::Owned(FlowElem::new(children.to_vec()).pack().spanned(span)), shared))
}

/// Apply the show rules in the given style chain to a target element.
pub fn realize(
    engine: &mut Engine,
    target: &Content,
    styles: StyleChain,
) -> SourceResult<Option<Content>> {
    let Some(Verdict { prepared, mut map, step }) = verdict(engine, target, styles)
    else {
        return Ok(None);
    };

    // Create a fresh copy that we can mutate.
    let mut target = target.clone();

    // If the element isn't yet prepared (we're seeing it for the first time),
    // prepare it.
    let mut meta = None;
    if !prepared {
        meta = prepare(engine, &mut target, &mut map, styles)?;
    }

    // Apply a step, if there is one.
    let mut output = match step {
        Some(step) => {
            // Errors in show rules don't terminate compilation immediately. We
            // just continue with empty content for them and show all errors
            // together, if they remain by the end of the introspection loop.
            //
            // This way, we can ignore errors that only occur in earlier
            // iterations and also show more useful errors at once.
            engine.delayed(|engine| show(engine, target, step, styles.chain(&map)))
        }
        None => target,
    };

    // If necessary, apply metadata generated in the preparation.
    if let Some(meta) = meta {
        output += meta.pack();
    }

    Ok(Some(output.styled_with_map(map)))
}

/// What to do with an element when encountering it during realization.
struct Verdict<'a> {
    /// Whether the element is already prepated (i.e. things that should only
    /// happen once have happened).
    prepared: bool,
    /// A map of styles to apply to the element.
    map: Styles,
    /// An optional show rule transformation to apply to the element.
    step: Option<ShowStep<'a>>,
}

/// An optional show rule transformation to apply to the element.
enum ShowStep<'a> {
    /// A user-defined transformational show rule.
    Recipe(&'a Recipe, RecipeIndex),
    /// The built-in show rule.
    Builtin,
}

/// Inspects a target element and the current styles and determines how to
/// proceed with the styling.
fn verdict<'a>(
    engine: &mut Engine,
    target: &'a Content,
    styles: StyleChain<'a>,
) -> Option<Verdict<'a>> {
    let mut target = target;
    let mut map = Styles::new();
    let mut revoked = BitSet::new();
    let mut step = None;
    let mut slot;

    let depth = OnceCell::new();
    let prepared = target.is_prepared();

    // Do pre-synthesis on a cloned element to be able to match on synthesized
    // fields before real synthesis runs (during preparation). It's really
    // unfortunate that we have to do this, but otherwise
    // `show figure.where(kind: table)` won't work :(
    if !prepared && target.can::<dyn Synthesize>() {
        slot = target.clone();
        slot.with_mut::<dyn Synthesize>()
            .unwrap()
            .synthesize(engine, styles)
            .ok();
        target = &slot;
    }

    let mut r = 0;
    for entry in styles.entries() {
        let recipe = match entry {
            Style::Recipe(recipe) => recipe,
            Style::Property(_) => continue,
            Style::Revocation(index) => {
                revoked.insert(index.0);
                continue;
            }
        };

        // We're not interested in recipes that don't match.
        if !recipe.applicable(target, styles) {
            r += 1;
            continue;
        }

        if let Transformation::Style(transform) = &recipe.transform {
            // If this is a show-set for an unprepared element, we need to apply
            // it.
            if !prepared {
                map.apply(transform.clone());
            }
        } else if step.is_none() {
            // Lazily compute the total number of recipes in the style chain. We
            // need it to determine whether a particular show rule was already
            // applied to the `target` previously. For this purpose, show rules
            // are indexed from the top of the chain as the chain might grow to
            // the bottom.
            let depth =
                *depth.get_or_init(|| styles.entries().filter_map(Style::recipe).count());
            let index = RecipeIndex(depth - r);

            if !target.is_guarded(index) && !revoked.contains(index.0) {
                // If we find a matching, unguarded replacement show rule,
                // remember it, but still continue searching for potential
                // show-set styles that might change the verdict.
                step = Some(ShowStep::Recipe(recipe, index));

                // If we found a show rule and are already prepared, there is
                // nothing else to do, so we can just break.
                if prepared {
                    break;
                }
            }
        }

        r += 1;
    }

    // If we found no user-defined rule, also consider the built-in show rule.
    if step.is_none() && target.can::<dyn Show>() {
        step = Some(ShowStep::Builtin);
    }

    // If there's no nothing to do, there is also no verdict.
    if step.is_none()
        && map.is_empty()
        && (prepared || {
            target.label().is_none()
                && !target.can::<dyn ShowSet>()
                && !target.can::<dyn Locatable>()
                && !target.can::<dyn Synthesize>()
        })
    {
        return None;
    }

    Some(Verdict { prepared, map, step })
}

/// This is only executed the first time an element is visited.
fn prepare(
    engine: &mut Engine,
    target: &mut Content,
    map: &mut Styles,
    styles: StyleChain,
) -> SourceResult<Option<Packed<MetaElem>>> {
    // Generate a location for the element, which uniquely identifies it in
    // the document. This has some overhead, so we only do it for elements
    // that are explicitly marked as locatable and labelled elements.
    if target.can::<dyn Locatable>() || target.label().is_some() {
        let location = engine.locator.locate(hash128(&target));
        target.set_location(location);
    }

    // Apply built-in show-set rules. User-defined show-set rules are already
    // considered in the map built while determining the verdict.
    if let Some(show_settable) = target.with::<dyn ShowSet>() {
        map.apply(show_settable.show_set(styles));
    }

    // If necessary, generated "synthesized" fields (which are derived from
    // other fields or queries). Do this after show-set so that show-set styles
    // are respected.
    if let Some(synthesizable) = target.with_mut::<dyn Synthesize>() {
        synthesizable.synthesize(engine, styles.chain(map))?;
    }

    // Copy style chain fields into the element itself, so that they are
    // available in rules.
    target.materialize(styles.chain(map));

    // Ensure that this preparation only runs once by marking the element as
    // prepared.
    target.mark_prepared();

    // Apply metadata be able to find the element in the frames.
    // Do this after synthesis, so that it includes the synthesized fields.
    if target.location().is_some() {
        // Add a style to the whole element's subtree identifying it as
        // belonging to the element.
        map.set(MetaElem::set_data(smallvec![Meta::Elem(target.clone())]));

        // Return an extra meta elem that will be attached so that the metadata
        // styles are not lost in case the element's show rule results in
        // nothing.
        return Ok(Some(Packed::new(MetaElem::new()).spanned(target.span())));
    }

    Ok(None)
}

/// Apply a step.
fn show(
    engine: &mut Engine,
    target: Content,
    step: ShowStep,
    styles: StyleChain,
) -> SourceResult<Content> {
    match step {
        // Apply a user-defined show rule.
        ShowStep::Recipe(recipe, guard) => match &recipe.selector {
            // If the selector is a regex, the `target` is guaranteed to be a
            // text element. This invokes special regex handling.
            Some(Selector::Regex(regex)) => {
                let text = target.into_packed::<TextElem>().unwrap();
                show_regex(engine, &text, regex, recipe, guard)
            }

            // Just apply the recipe.
            _ => recipe.apply(engine, target.guarded(guard)),
        },

        // If the verdict picks this step, the `target` is guaranteed to have a
        // built-in show rule.
        ShowStep::Builtin => target.with::<dyn Show>().unwrap().show(engine, styles),
    }
}

/// Apply a regex show rule recipe to a target.
fn show_regex(
    engine: &mut Engine,
    elem: &Packed<TextElem>,
    regex: &Regex,
    recipe: &Recipe,
    index: RecipeIndex,
) -> SourceResult<Content> {
    let make = |s: &str| {
        let mut fresh = elem.clone();
        fresh.push_text(s.into());
        fresh.pack()
    };

    let mut result = vec![];
    let mut cursor = 0;

    let text = elem.text();

    for m in regex.find_iter(elem.text()) {
        let start = m.start();
        if cursor < start {
            result.push(make(&text[cursor..start]));
        }

        let piece = make(m.as_str());
        let transformed = recipe.apply(engine, piece)?;
        result.push(transformed);
        cursor = m.end();
    }

    if cursor < text.len() {
        result.push(make(&text[cursor..]));
    }

    // In contrast to normal elements, which are guarded individually, for text
    // show rules, we fully revoke the rule. This means that we can replace text
    // with other text that rematches without running into infinite recursion
    // problems.
    //
    // We do _not_ do this for all content because revoking e.g. a list show
    // rule for all content resulting from that rule would be wrong: The list
    // might contain nested lists. Moreover, replacing a normal element with one
    // that rematches is bad practice: It can for instance also lead to
    // surprising query results, so it's better to let the user deal with it.
    // All these problems don't exist for text, so it's fine here.
    Ok(Content::sequence(result).styled(Style::Revocation(index)))
}

/// Builds a document or a flow element from content.
struct Builder<'a, 'v, 't> {
    /// The engine.
    engine: &'v mut Engine<'t>,
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
    /// The current citation grouping state.
    cites: CiteGroupBuilder<'a>,
}

/// Temporary storage arenas for building.
#[derive(Default)]
pub struct Scratch<'a> {
    /// An arena where intermediate style chains are stored.
    styles: Arena<StyleChain<'a>>,
    /// An arena where intermediate content resulting from show rules is stored.
    content: Arena<Content>,
}

impl<'a, 'v, 't> Builder<'a, 'v, 't> {
    fn new(engine: &'v mut Engine<'t>, scratch: &'a Scratch<'a>, top: bool) -> Self {
        Self {
            engine,
            scratch,
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
                .scratch
                .content
                .alloc(EquationElem::new(content.clone()).pack().spanned(content.span()));
        }

        if let Some(realized) = realize(self.engine, content, styles)? {
            self.engine.route.increase();
            if !self.engine.route.within(Route::MAX_SHOW_RULE_DEPTH) {
                bail!(
                    content.span(), "maximum show rule depth exceeded";
                    hint: "check whether the show rule matches its own output"
                );
            }
            let stored = self.scratch.content.alloc(realized);
            let v = self.accept(stored, styles);
            self.engine.route.decrease();
            return v;
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

        if self.flow.accept(content, styles) {
            return Ok(());
        }

        let keep = content
            .to_packed::<PagebreakElem>()
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

    fn interrupt_cites(&mut self) -> SourceResult<()> {
        if !self.cites.items.is_empty() {
            let staged = mem::take(&mut self.cites.staged);
            let (group, styles) = mem::take(&mut self.cites).finish();
            let stored = self.scratch.content.alloc(group);
            self.accept(stored, styles)?;
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
            let (children, shared) = mem::take(&mut self.flow).0.finish();
            let styles = if shared == StyleChain::default() {
                styles.unwrap_or_default()
            } else {
                shared
            };
            let span = first_span(&children);
            let flow = FlowElem::new(children.to_vec());
            let page = PageElem::new(flow.pack().spanned(span));
            let stored = self.scratch.content.alloc(page.pack().spanned(span));
            self.accept(stored, styles)?;
        }
        Ok(())
    }
}

/// Accepts pagebreaks and pages.
struct DocBuilder<'a> {
    /// The page runs built so far.
    pages: StyleVecBuilder<'a, Cow<'a, Content>>,
    /// Whether to keep a following page even if it is empty.
    keep_next: bool,
    /// Whether the next page should be cleared to an even or odd number.
    clear_next: Option<Parity>,
}

impl<'a> DocBuilder<'a> {
    fn accept(&mut self, content: &'a Content, styles: StyleChain<'a>) -> bool {
        if let Some(pagebreak) = content.to_packed::<PagebreakElem>() {
            self.keep_next = !pagebreak.weak(styles);
            self.clear_next = pagebreak.to(styles);
            return true;
        }

        if let Some(page) = content.to_packed::<PageElem>() {
            let elem = if let Some(clear_to) = self.clear_next.take() {
                let mut page = page.clone();
                page.push_clear_to(Some(clear_to));
                Cow::Owned(page.pack())
            } else {
                Cow::Borrowed(content)
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
            self.0.push(Cow::Borrowed(content), styles);
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
                self.0.push(Cow::Owned(spacing.pack()), styles);
            }

            let (above, below) = if let Some(block) = content.to_packed::<BlockElem>() {
                (block.above(styles), block.below(styles))
            } else {
                (BlockElem::above_in(styles), BlockElem::below_in(styles))
            };

            self.0.push(Cow::Owned(above.pack()), styles);
            self.0.push(Cow::Borrowed(content), styles);
            self.0.push(Cow::Owned(below.pack()), styles);
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
                self.0.push(Cow::Borrowed(content), styles);
                return true;
            }
        } else if content.is::<SpaceElem>()
            || content.is::<TextElem>()
            || content.is::<HElem>()
            || content.is::<LinebreakElem>()
            || content.is::<SmartQuoteElem>()
            || content
                .to_packed::<EquationElem>()
                .map_or(false, |elem| !elem.block(styles))
            || content.is::<BoxElem>()
        {
            self.0.push(Cow::Borrowed(content), styles);
            return true;
        }

        false
    }

    fn finish(self) -> (Content, StyleChain<'a>) {
        let (children, shared) = self.0.finish();
        let span = first_span(&children);
        (ParElem::new(children.to_vec()).pack().spanned(span), shared)
    }
}

/// Accepts list / enum items, spaces, paragraph breaks.
struct ListBuilder<'a> {
    /// The list items collected so far.
    items: StyleVecBuilder<'a, Cow<'a, Content>>,
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
            self.items.push(Cow::Borrowed(content), styles);
            self.tight &= self.staged.drain(..).all(|(t, _)| !t.is::<ParbreakElem>());
            return true;
        }

        false
    }

    fn finish(self) -> (Content, StyleChain<'a>) {
        let (items, shared) = self.items.finish();
        let span = first_span(&items);
        let item = items.items().next().unwrap();
        let output = if item.is::<ListItem>() {
            ListElem::new(
                items
                    .iter()
                    .map(|(item, local)| {
                        let mut item = item.to_packed::<ListItem>().unwrap().clone();
                        let body = item.body().clone().styled_with_map(local.clone());
                        item.push_body(body);
                        item
                    })
                    .collect::<Vec<_>>(),
            )
            .with_tight(self.tight)
            .pack()
            .spanned(span)
        } else if item.is::<EnumItem>() {
            EnumElem::new(
                items
                    .iter()
                    .map(|(item, local)| {
                        let mut item = item.to_packed::<EnumItem>().unwrap().clone();
                        let body = item.body().clone().styled_with_map(local.clone());
                        item.push_body(body);
                        item
                    })
                    .collect::<Vec<_>>(),
            )
            .with_tight(self.tight)
            .pack()
            .spanned(span)
        } else if item.is::<TermItem>() {
            TermsElem::new(
                items
                    .iter()
                    .map(|(item, local)| {
                        let mut item = item.to_packed::<TermItem>().unwrap().clone();
                        let term = item.term().clone().styled_with_map(local.clone());
                        let description =
                            item.description().clone().styled_with_map(local.clone());
                        item.push_term(term);
                        item.push_description(description);
                        item
                    })
                    .collect::<Vec<_>>(),
            )
            .with_tight(self.tight)
            .pack()
            .spanned(span)
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

    fn finish(self) -> (Content, StyleChain<'a>) {
        let span = self.items.first().map(|cite| cite.span()).unwrap_or(Span::detached());
        (CiteGroup::new(self.items).pack().spanned(span), self.styles)
    }
}

/// Find the first span that isn't detached.
fn first_span(children: &StyleVec<Cow<Content>>) -> Span {
    children
        .iter()
        .filter(|(elem, _)| {
            elem.with::<dyn Behave>()
                .map_or(true, |b| b.behaviour() != Behaviour::Invisible)
        })
        .map(|(elem, _)| elem.span())
        .find(|span| !span.is_detached())
        .unwrap_or_else(Span::detached)
}
