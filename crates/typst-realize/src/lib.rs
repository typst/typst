//! Typst's realization subsystem.
//!
//! *Realization* is the process of recursively applying styling and, in
//! particular, show rules to produce well-known elements that can be processed
//! further.

use std::borrow::Cow;
use std::cell::LazyCell;

use arrayvec::ArrayVec;
use bumpalo::Bump;
use bumpalo::collections::{CollectIn, String as BumpString, Vec as BumpVec};
use comemo::Track;
use ecow::EcoString;
use typst_library::diag::{At, SourceResult, bail};
use typst_library::engine::Engine;
use typst_library::foundations::{
    Content, Context, ContextElem, Element, NativeElement, NativeShowRule, Packed,
    Recipe, RecipeIndex, Selector, SequenceElem, ShowSet, Style, StyleChain, StyledElem,
    Styles, SymbolElem, Synthesize, TargetElem, Transformation,
};
use typst_library::introspection::{Locatable, LocationKey, SplitLocator, Tag, TagElem};
use typst_library::layout::{
    AlignElem, BoxElem, HElem, InlineElem, PageElem, PagebreakElem, VElem,
};
use typst_library::math::{EquationElem, Mathy};
use typst_library::model::{
    CiteElem, CiteGroup, DocumentElem, EnumElem, ListElem, ListItemLike, ListLike,
    ParElem, ParbreakElem, TermsElem,
};
use typst_library::routines::{Arenas, FragmentKind, Pair, RealizationKind};
use typst_library::text::{LinebreakElem, SmartQuoteElem, SpaceElem, TextElem};
use typst_syntax::Span;
use typst_utils::{ListSet, SliceExt, SmallBitSet};

/// Realize content into a flat list of well-known, styled items.
#[typst_macros::time(name = "realize")]
pub fn realize<'a>(
    kind: RealizationKind,
    engine: &mut Engine,
    locator: &mut SplitLocator,
    arenas: &'a Arenas,
    content: &'a Content,
    styles: StyleChain<'a>,
) -> SourceResult<Vec<Pair<'a>>> {
    let mut s = State {
        engine,
        locator,
        arenas,
        rules: match kind {
            RealizationKind::LayoutDocument { .. } => LAYOUT_RULES,
            RealizationKind::LayoutFragment { .. } => LAYOUT_RULES,
            RealizationKind::LayoutPar => LAYOUT_PAR_RULES,
            RealizationKind::HtmlDocument { .. } => HTML_DOCUMENT_RULES,
            RealizationKind::HtmlFragment { .. } => HTML_FRAGMENT_RULES,
            RealizationKind::Math => MATH_RULES,
        },
        sink: vec![],
        groupings: ArrayVec::new(),
        outside: kind.is_document(),
        may_attach: false,
        saw_parbreak: false,
        kind,
    };

    visit(&mut s, content, styles)?;
    finish(&mut s)?;

    Ok(s.sink)
}

/// Mutable state for realization.
///
/// Sadly, we need that many lifetimes because &mut references are invariant and
/// it would force the lifetimes of e.g. engine and locator to be equal if they
/// shared a lifetime. We can get around it by enforcing the lifetimes on
/// `fn realize`, but that makes it less flexible on the call site, which isn't
/// worth it.
///
/// The only interesting lifetime is 'a, which is that of the content that comes
/// in and goes out. It's the same 'a as on `fn realize`.
struct State<'a, 'x, 'y, 'z> {
    /// Defines what kind of realization we are performing.
    kind: RealizationKind<'x>,
    /// The engine.
    engine: &'x mut Engine<'y>,
    /// Assigns unique locations to elements.
    locator: &'x mut SplitLocator<'z>,
    /// Temporary storage arenas for lifetime extension during realization.
    arenas: &'a Arenas,
    /// The output elements of well-known types.
    sink: Vec<Pair<'a>>,
    /// Grouping rules used for realization.
    rules: &'x [&'x GroupingRule],
    /// Currently active groupings.
    groupings: ArrayVec<Grouping<'x>, MAX_GROUP_NESTING>,
    /// Whether we are currently not within any container or show rule output.
    /// This is used to determine page styles during layout.
    outside: bool,
    /// Whether now following attach spacing can survive.
    may_attach: bool,
    /// Whether we visited any paragraph breaks.
    saw_parbreak: bool,
}

/// Defines a rule for how certain elements shall be grouped during realization.
struct GroupingRule {
    /// When an element is visited that matches a rule with higher priority
    /// than one that is currently grouped, we start a nested group.
    priority: u8,
    /// Whether the grouping handles tags itself. If this is set to `false`,
    /// realization will transparently take care of tags and they will not
    /// be visible to `finish`.
    tags: bool,
    /// Defines which kinds of elements start and make up this kind of grouping.
    trigger: fn(&Content, &State) -> bool,
    /// Defines elements that may appear in the interior of the grouping, but
    /// not at the edges.
    inner: fn(&Content) -> bool,
    /// Defines whether styles for this kind of element interrupt the grouping.
    interrupt: fn(Element) -> bool,
    /// Should convert the accumulated elements in `s.sink[start..]` into
    /// the grouped element.
    finish: fn(Grouped) -> SourceResult<()>,
}

/// A started grouping of some elements.
struct Grouping<'a> {
    /// The position in `s.sink` where the group starts.
    start: usize,
    /// Only applies to `PAR` grouping: Whether this paragraph group is
    /// interrupted, but not yet finished because it may be ignored due to being
    /// fully inline.
    interrupted: bool,
    /// The rule used for this grouping.
    rule: &'a GroupingRule,
}

/// The result of grouping.
struct Grouped<'a, 'x, 'y, 'z, 's> {
    /// The realization state.
    s: &'s mut State<'a, 'x, 'y, 'z>,
    /// The position in `s.sink` where the group starts.
    start: usize,
}

/// What to do with an element when encountering it during realization.
struct Verdict<'a> {
    /// Whether the element is already prepared (i.e. things that should only
    /// happen once have happened).
    prepared: bool,
    /// A map of styles to apply to the element.
    map: Styles,
    /// An optional show rule transformation to apply to the element.
    step: Option<ShowStep<'a>>,
}

/// A show rule transformation to apply to the element.
enum ShowStep<'a> {
    /// A user-defined transformational show rule.
    Recipe(&'a Recipe, RecipeIndex),
    /// The built-in show rule.
    Builtin(NativeShowRule),
}

/// A match of a regex show rule.
struct RegexMatch<'a> {
    /// The offset in the string that matched.
    offset: usize,
    /// The text that matched.
    text: EcoString,
    /// The style chain of the matching grouping.
    styles: StyleChain<'a>,
    /// The index of the recipe that matched.
    id: RecipeIndex,
    /// The recipe that matched.
    recipe: &'a Recipe,
}

/// State kept for space collapsing.
#[derive(Debug, Copy, Clone, Eq, PartialEq)]
enum SpaceState {
    /// A following space will be collapsed.
    Destructive,
    /// A following space will be kept unless a destructive element follows.
    Supportive,
    /// A space exists at this index.
    Space(usize),
}

impl<'a> State<'a, '_, '_, '_> {
    /// Lifetime-extends some content.
    fn store(&self, content: Content) -> &'a Content {
        self.arenas.content.alloc(content)
    }

    /// Lifetime-extends some pairs.
    ///
    /// By using a `BumpVec` instead of a `alloc_slice_copy` we can reuse
    /// the space if no other bump allocations have been made by the time
    /// the `BumpVec` is dropped.
    fn store_slice(&self, pairs: &[Pair<'a>]) -> BumpVec<'a, Pair<'a>> {
        let mut vec = BumpVec::new_in(&self.arenas.bump);
        vec.extend_from_slice_copy(pairs);
        vec
    }
}

impl<'a, 'x, 'y, 'z, 's> Grouped<'a, 'x, 'y, 'z, 's> {
    /// Accesses the grouped elements.
    fn get(&self) -> &[Pair<'a>] {
        &self.s.sink[self.start..]
    }

    /// Accesses the grouped elements mutably.
    fn get_mut(&mut self) -> (&mut Vec<Pair<'a>>, usize) {
        (&mut self.s.sink, self.start)
    }

    /// Removes the grouped elements from the sink and retrieves back the state
    /// with which resulting elements can be visited.
    fn end(self) -> &'s mut State<'a, 'x, 'y, 'z> {
        self.s.sink.truncate(self.start);
        self.s
    }
}

/// Handles an arbitrary piece of content during realization.
fn visit<'a>(
    s: &mut State<'a, '_, '_, '_>,
    content: &'a Content,
    styles: StyleChain<'a>,
) -> SourceResult<()> {
    // Tags can always simply be pushed.
    if content.is::<TagElem>() {
        s.sink.push((content, styles));
        return Ok(());
    }

    // Transformations for content based on the realization kind. Needs
    // to happen before show rules.
    if visit_kind_rules(s, content, styles)? {
        return Ok(());
    }

    // Apply show rules and preparation.
    if visit_show_rules(s, content, styles)? {
        return Ok(());
    }

    // Recurse into sequences. Styled elements and sequences can currently also
    // have labels, so this needs to happen before they are handled.
    if let Some(sequence) = content.to_packed::<SequenceElem>() {
        for elem in &sequence.children {
            visit(s, elem, styles)?;
        }
        return Ok(());
    }

    // Recurse into styled elements.
    if let Some(styled) = content.to_packed::<StyledElem>() {
        return visit_styled(s, &styled.child, Cow::Borrowed(&styled.styles), styles);
    }

    // Apply grouping --- where multiple elements are collected and then
    // processed together (typically being transformed into one).
    if visit_grouping_rules(s, content, styles)? {
        return Ok(());
    }

    // Some elements are skipped based on specific circumstances.
    if visit_filter_rules(s, content, styles)? {
        return Ok(());
    }

    // No further transformations to apply, so we can finally just push it to
    // the output!
    s.sink.push((content, styles));

    Ok(())
}

// Handles transformations based on the realization kind.
fn visit_kind_rules<'a>(
    s: &mut State<'a, '_, '_, '_>,
    content: &'a Content,
    styles: StyleChain<'a>,
) -> SourceResult<bool> {
    if let RealizationKind::Math = s.kind {
        // Transparently recurse into equations nested in math, so that things
        // like this work:
        // ```
        // #let my = $pi$
        // $ my r^2 $
        // ```
        if let Some(elem) = content.to_packed::<EquationElem>() {
            visit(s, &elem.body, styles)?;
            return Ok(true);
        }

        // In normal realization, we apply regex show rules to consecutive
        // textual elements via `TEXTUAL` grouping. However, in math, this is
        // not desirable, so we just do it on a per-element basis.
        if let Some(elem) = content.to_packed::<SymbolElem>() {
            if let Some(m) = find_regex_match_in_str(elem.text.as_str(), styles) {
                visit_regex_match(s, &[(content, styles)], m)?;
                return Ok(true);
            }
        } else if let Some(elem) = content.to_packed::<TextElem>()
            && let Some(m) = find_regex_match_in_str(&elem.text, styles)
        {
            visit_regex_match(s, &[(content, styles)], m)?;
            return Ok(true);
        }
    } else {
        // Transparently wrap mathy content into equations.
        if content.can::<dyn Mathy>() && !content.is::<EquationElem>() {
            let eq = EquationElem::new(content.clone()).pack().spanned(content.span());
            visit(s, s.store(eq), styles)?;
            return Ok(true);
        }

        // Symbols in non-math content transparently convert to `TextElem` so we
        // don't have to handle them in non-math layout.
        if let Some(elem) = content.to_packed::<SymbolElem>() {
            let mut text = TextElem::packed(elem.text.clone()).spanned(elem.span());
            if let Some(label) = elem.label() {
                text.set_label(label);
            }
            visit(s, s.store(text), styles)?;
            return Ok(true);
        }
    }

    Ok(false)
}

/// Tries to apply show rules to or prepare content. Returns `true` if the
/// element was handled.
fn visit_show_rules<'a>(
    s: &mut State<'a, '_, '_, '_>,
    content: &'a Content,
    styles: StyleChain<'a>,
) -> SourceResult<bool> {
    // Determines whether and how to proceed with show rule application.
    let Some(Verdict { prepared, mut map, step }) = verdict(s.engine, content, styles)
    else {
        return Ok(false);
    };

    // Create a fresh copy that we can mutate.
    let mut output = Cow::Borrowed(content);

    // If the element isn't yet prepared (we're seeing it for the first time),
    // prepare it.
    let mut tags = None;
    if !prepared {
        tags = prepare(s.engine, s.locator, output.to_mut(), &mut map, styles)?;
    }

    // Apply a show rule step, if there is one.
    if let Some(step) = step {
        let chained = styles.chain(&map);
        let result = match step {
            // Apply a user-defined show rule.
            ShowStep::Recipe(recipe, guard) => {
                let context = Context::new(output.location(), Some(chained));
                recipe.apply(
                    s.engine,
                    context.track(),
                    output.into_owned().guarded(guard),
                )
            }

            // Apply a built-in show rule.
            ShowStep::Builtin(rule) => {
                let _scope = typst_timing::TimingScope::new(output.elem().name());
                rule.apply(&output, s.engine, chained)
                    .map(|content| content.spanned(output.span()))
            }
        };

        // Errors in show rules don't terminate compilation immediately. We just
        // continue with empty content for them and show all errors together, if
        // they remain by the end of the introspection loop.
        //
        // This way, we can ignore errors that only occur in earlier iterations
        // and also show more useful errors at once.
        output = Cow::Owned(s.engine.delay(result));
    }

    // Lifetime-extend the realized content if necessary.
    let realized = match output {
        Cow::Borrowed(realized) => realized,
        Cow::Owned(realized) => s.store(realized),
    };

    // Push start tag.
    let (start, end) = tags.unzip();
    if let Some(tag) = start {
        visit(s, s.store(TagElem::packed(tag)), styles)?;
    }

    let prev_outside = s.outside;
    s.outside &= content.is::<ContextElem>();
    s.engine.route.increase();
    s.engine.route.check_show_depth().at(content.span())?;

    visit_styled(s, realized, Cow::Owned(map), styles)?;

    s.outside = prev_outside;
    s.engine.route.decrease();

    // Push end tag.
    if let Some(tag) = end {
        visit(s, s.store(TagElem::packed(tag)), styles)?;
    }

    Ok(true)
}

/// Inspects an element and the current styles and determines how to proceed
/// with the styling.
fn verdict<'a>(
    engine: &mut Engine,
    elem: &'a Content,
    styles: StyleChain<'a>,
) -> Option<Verdict<'a>> {
    let prepared = elem.is_prepared();
    let mut map = Styles::new();
    let mut step = None;

    // Do pre-synthesis on a cloned element to be able to match on synthesized
    // fields before real synthesis runs (during preparation). It's really
    // unfortunate that we have to do this, but otherwise
    // `show figure.where(kind: table)` won't work :(
    let mut elem = elem;
    let mut slot;
    if !prepared && elem.can::<dyn Synthesize>() {
        slot = elem.clone();
        slot.with_mut::<dyn Synthesize>()
            .unwrap()
            .synthesize(engine, styles)
            .ok();
        elem = &slot;
    }

    // Lazily computes the total number of recipes in the style chain. We need
    // it to determine whether a particular show rule was already applied to the
    // `elem` previously. For this purpose, show rules are indexed from the
    // top of the chain as the chain might grow to the bottom.
    let depth = LazyCell::new(|| styles.recipes().count());

    for (r, recipe) in styles.recipes().enumerate() {
        // We're not interested in recipes that don't match.
        if !recipe
            .selector()
            .is_some_and(|selector| selector.matches(elem, Some(styles)))
        {
            continue;
        }

        // Special handling for show-set rules.
        if let Transformation::Style(transform) = recipe.transform() {
            if !prepared {
                map.apply(transform.clone());
            }
            continue;
        }

        // If we already have a show step, don't look for one.
        if step.is_some() {
            continue;
        }

        // Check whether this show rule was already applied to the element.
        let index = RecipeIndex(*depth - r);
        if elem.is_guarded(index) {
            continue;
        }

        // We'll apply this recipe.
        step = Some(ShowStep::Recipe(recipe, index));

        // If we found a show rule and are already prepared, there is nothing
        // else to do, so we can just break. If we are not yet prepared,
        // continue searching for potential show-set styles.
        if prepared {
            break;
        }
    }

    // If we found no user-defined rule, also consider the built-in show rule.
    if step.is_none() {
        let target = styles.get(TargetElem::target);
        if let Some(rule) = engine.routines.rules.get(target, elem) {
            step = Some(ShowStep::Builtin(rule));
        }
    }

    // If there's no nothing to do, there is also no verdict.
    if step.is_none()
        && map.is_empty()
        && (prepared || {
            elem.label().is_none()
                && elem.location().is_none()
                && !elem.can::<dyn ShowSet>()
                && !elem.can::<dyn Locatable>()
                && !elem.can::<dyn Synthesize>()
        })
    {
        return None;
    }

    Some(Verdict { prepared, map, step })
}

/// This is only executed the first time an element is visited.
fn prepare(
    engine: &mut Engine,
    locator: &mut SplitLocator,
    elem: &mut Content,
    map: &mut Styles,
    styles: StyleChain,
) -> SourceResult<Option<(Tag, Tag)>> {
    // Generate a location for the element, which uniquely identifies it in
    // the document. This has some overhead, so we only do it for elements
    // that are explicitly marked as locatable and labelled elements.
    //
    // The element could already have a location even if it is not prepared
    // when it stems from a query.
    let key = typst_utils::hash128(&elem);
    if elem.location().is_none()
        && (elem.can::<dyn Locatable>() || elem.label().is_some())
    {
        let loc = locator.next_location(engine.introspector, key);
        elem.set_location(loc);
    }

    // Apply built-in show-set rules. User-defined show-set rules are already
    // considered in the map built while determining the verdict.
    if let Some(show_settable) = elem.with::<dyn ShowSet>() {
        map.apply(show_settable.show_set(styles));
    }

    // If necessary, generated "synthesized" fields (which are derived from
    // other fields or queries). Do this after show-set so that show-set styles
    // are respected.
    if let Some(synthesizable) = elem.with_mut::<dyn Synthesize>() {
        synthesizable.synthesize(engine, styles.chain(map))?;
    }

    // Copy style chain fields into the element itself, so that they are
    // available in rules.
    elem.materialize(styles.chain(map));

    // If the element is locatable, create start and end tags to be able to find
    // the element in the frames after layout. Do this after synthesis and
    // materialization, so that it includes the synthesized fields. Do it before
    // marking as prepared so that show-set rules will apply to this element
    // when queried.
    let tags = elem
        .location()
        .map(|loc| (Tag::Start(elem.clone()), Tag::End(loc, key)));

    // Ensure that this preparation only runs once by marking the element as
    // prepared.
    elem.mark_prepared();

    Ok(tags)
}

/// Handles a styled element.
fn visit_styled<'a>(
    s: &mut State<'a, '_, '_, '_>,
    content: &'a Content,
    mut local: Cow<'a, Styles>,
    outer: StyleChain<'a>,
) -> SourceResult<()> {
    // Nothing to do if the styles are actually empty.
    if local.is_empty() {
        return visit(s, content, outer);
    }

    // Check for document and page styles.
    let mut pagebreak = false;
    for style in local.iter() {
        let Some(elem) = style.element() else { continue };
        if elem == DocumentElem::ELEM {
            if let Some(info) = s.kind.as_document_mut() {
                info.populate(&local)
            } else {
                bail!(
                    style.span(),
                    "document set rules are not allowed inside of containers"
                );
            }
        } else if elem == PageElem::ELEM {
            if !matches!(s.kind, RealizationKind::LayoutDocument { .. }) {
                bail!(
                    style.span(),
                    "page configuration is not allowed inside of containers"
                );
            }

            // When there are page styles, we "break free" from our show rule cage.
            pagebreak = true;
            s.outside = true;
        }
    }

    // If we are not within a container or show rule, mark the styles as
    // "outside". This will allow them to be lifted to the page level.
    if s.outside {
        local = Cow::Owned(local.into_owned().outside());
    }

    // Lifetime-extend the styles if necessary.
    let outer = s.arenas.bump.alloc(outer);
    let local = match local {
        Cow::Borrowed(map) => map,
        Cow::Owned(owned) => &*s.arenas.styles.alloc(owned),
    };

    // Generate a weak pagebreak if there is a page interruption. For the
    // starting pagebreak we only want the styles before and including the
    // interruptions, not trailing styles that happen to be in the same `Styles`
    // list, so we trim the local styles.
    if pagebreak {
        let relevant = local
            .as_slice()
            .trim_end_matches(|style| style.element() != Some(PageElem::ELEM));
        visit(s, PagebreakElem::shared_weak(), outer.chain(relevant))?;
    }

    finish_interrupted(s, local)?;
    visit(s, content, outer.chain(local))?;
    finish_interrupted(s, local)?;

    // Generate a weak "boundary" pagebreak at the end. In comparison to a
    // normal weak pagebreak, the styles of this are ignored during layout, so
    // it doesn't really matter what we use here.
    if pagebreak {
        visit(s, PagebreakElem::shared_boundary(), *outer)?;
    }

    Ok(())
}

/// Tries to group the content in an active group or start a new one if any
/// grouping rule matches. Returns `true` if the element was grouped.
fn visit_grouping_rules<'a>(
    s: &mut State<'a, '_, '_, '_>,
    content: &'a Content,
    styles: StyleChain<'a>,
) -> SourceResult<bool> {
    let matching = s.rules.iter().find(|&rule| (rule.trigger)(content, s));

    // Try to continue or finish an existing grouping.
    let mut i = 0;
    while let Some(active) = s.groupings.last() {
        // Start a nested group if a rule with higher priority matches.
        if matching.is_some_and(|rule| rule.priority > active.rule.priority) {
            break;
        }

        // If the element can be added to the active grouping, do it.
        if !active.interrupted
            && ((active.rule.trigger)(content, s) || (active.rule.inner)(content))
        {
            s.sink.push((content, styles));
            return Ok(true);
        }

        finish_innermost_grouping(s)?;
        i += 1;
        if i > 512 {
            // It seems like this case is only hit when there is a cycle between
            // a show rule and a grouping rule. The show rule produces content
            // that is matched by a grouping rule, which is then again processed
            // by the show rule, and so on. The two must be at an equilibrium,
            // otherwise either the "maximum show rule depth" or "maximum
            // grouping depth" errors are triggered.
            bail!(content.span(), "maximum grouping depth exceeded");
        }
    }

    // Start a new grouping.
    if let Some(rule) = matching {
        let start = s.sink.len();
        s.groupings.push(Grouping { start, rule, interrupted: false });
        s.sink.push((content, styles));
        return Ok(true);
    }

    Ok(false)
}

/// Some elements don't make it to the sink depending on the realization kind
/// and current state.
fn visit_filter_rules<'a>(
    s: &mut State<'a, '_, '_, '_>,
    content: &'a Content,
    styles: StyleChain<'a>,
) -> SourceResult<bool> {
    if matches!(s.kind, RealizationKind::LayoutPar | RealizationKind::Math) {
        return Ok(false);
    }

    if content.is::<SpaceElem>() {
        // Outside of maths and paragraph realization, spaces that were not
        // collected by the paragraph grouper don't interest us.
        return Ok(true);
    } else if content.is::<ParbreakElem>() {
        // Paragraph breaks are only a boundary for paragraph grouping, we don't
        // need to store them.
        s.may_attach = false;
        s.saw_parbreak = true;
        return Ok(true);
    } else if !s.may_attach
        && content
            .to_packed::<VElem>()
            .is_some_and(|elem| elem.attach.get(styles))
    {
        // Attach spacing collapses if not immediately following a paragraph.
        return Ok(true);
    }

    // Remember whether following attach spacing can survive.
    s.may_attach = content.is::<ParElem>();

    Ok(false)
}

/// Finishes all grouping.
fn finish(s: &mut State) -> SourceResult<()> {
    finish_grouping_while(s, |s| {
        // If this is a fragment realization and all we've got is inline
        // content, don't turn it into a paragraph.
        if is_fully_inline(s) {
            *s.kind.as_fragment_mut().unwrap() = FragmentKind::Inline;
            s.groupings.pop();
            collapse_spaces(&mut s.sink, 0);
            false
        } else {
            !s.groupings.is_empty()
        }
    })?;

    // In paragraph and math realization, spaces are top-level.
    if matches!(s.kind, RealizationKind::LayoutPar | RealizationKind::Math) {
        collapse_spaces(&mut s.sink, 0);
    }

    Ok(())
}

/// Finishes groupings while any active group is interrupted by the styles.
fn finish_interrupted(s: &mut State, local: &Styles) -> SourceResult<()> {
    let mut last = None;
    for elem in local.iter().filter_map(|style| style.element()) {
        if last == Some(elem) {
            continue;
        }
        finish_grouping_while(s, |s| {
            s.groupings.iter().any(|grouping| (grouping.rule.interrupt)(elem))
                && if is_fully_inline(s) {
                    s.groupings[0].interrupted = true;
                    false
                } else {
                    true
                }
        })?;
        last = Some(elem);
    }
    Ok(())
}

/// Finishes groupings while `f` returns `true`.
fn finish_grouping_while<F>(s: &mut State, mut f: F) -> SourceResult<()>
where
    F: FnMut(&mut State) -> bool,
{
    // Finishing of a group may result in new content and new grouping. This
    // can, in theory, go on for a bit. To prevent it from becoming an infinite
    // loop, we keep track of the iteration count.
    let mut i = 0;
    while f(s) {
        finish_innermost_grouping(s)?;
        i += 1;
        if i > 512 {
            bail!(Span::detached(), "maximum grouping depth exceeded");
        }
    }
    Ok(())
}

/// Finishes the currently innermost grouping.
fn finish_innermost_grouping(s: &mut State) -> SourceResult<()> {
    // The grouping we are interrupting.
    let Grouping { mut start, rule, .. } = s.groupings.pop().unwrap();

    // Trim trailing non-trigger elements. At the start, they are already not
    // included precisely because they are not triggers.
    let trimmed = s.sink[start..].trim_end_matches(|(c, _)| !(rule.trigger)(c, s));
    let mut end = start + trimmed.len();

    // Tags that are opened within or at the start boundary of the grouping
    // should have their closing tag included if it is at the end boundary.
    // Similarly, tags that are closed within or at the end boundary should have
    // their opening tag included if it is at the start boundary. Finally, tags
    // that are sandwiched between an opening tag with a matching closing tag
    // should also be included.
    if rule.tags {
        // The trailing part of the sink can contain a mix of inner elements and
        // tags. If there is a closing tag with a matching start tag, but there
        // is an inner element in between, that's in principle a situation with
        // overlapping tags. However, if the inner element would immediately be
        // destructed anyways, there isn't really a problem. So we try to
        // anticipate that and destruct it eagerly.
        if std::ptr::eq(rule, &PAR) {
            for _ in s.sink.extract_if(end.., |(c, _)| c.is::<SpaceElem>()) {}
        }

        // Find tags before, within, and after the grouping range.
        let bump = &s.arenas.bump;
        let before = tag_set(bump, s.sink[..start].iter().rev().map_while(to_tag));
        let within = tag_set(bump, s.sink[start..end].iter().filter_map(to_tag));
        let after = tag_set(bump, s.sink[end..].iter().map_while(to_tag));

        // Include all tags at the start that are closed within or after.
        for (k, (c, _)) in s.sink[..start].iter().enumerate().rev() {
            let Some(elem) = c.to_packed::<TagElem>() else { break };
            let key = elem.tag.location().into();
            if within.contains(&key) || after.contains(&key) {
                start = k;
            }
        }

        // Include all tags at the end that are opened within or before.
        for (k, (c, _)) in s.sink.iter().enumerate().skip(end) {
            let Some(elem) = c.to_packed::<TagElem>() else { break };
            let key = elem.tag.location().into();
            if within.contains(&key) || before.contains(&key) {
                end = k + 1;
            }
        }
    }

    let tail = s.store_slice(&s.sink[end..]);
    s.sink.truncate(end);

    // If the grouping is not interested in tags, remove and collect them.
    let mut tags = BumpVec::<Pair>::new_in(&s.arenas.bump);
    if !rule.tags {
        let mut k = start;
        for i in start..end {
            if s.sink[i].0.is::<TagElem>() {
                tags.push(s.sink[i]);
                continue;
            }

            if k < i {
                s.sink[k] = s.sink[i];
            }
            k += 1;
        }
        s.sink.truncate(k);
    }

    // Execute the grouping's finisher rule.
    (rule.finish)(Grouped { s, start })?;

    // Visit the tags and staged elements again.
    for &(content, styles) in tags.iter().chain(&tail) {
        visit(s, content, styles)?;
    }

    Ok(())
}

/// Extracts the locations of all tags in the given `list` into a bump-allocated
/// set.
fn tag_set<'a>(
    bump: &'a Bump,
    iter: impl IntoIterator<Item = &'a Packed<TagElem>>,
) -> ListSet<BumpVec<'a, LocationKey>> {
    ListSet::new(
        iter.into_iter()
            .map(|elem| LocationKey::new(elem.tag.location()))
            .collect_in::<BumpVec<_>>(bump),
    )
}

/// Tries to convert a pair to a tag.
fn to_tag<'a>((c, _): &Pair<'a>) -> Option<&'a Packed<TagElem>> {
    c.to_packed::<TagElem>()
}

/// The maximum number of nested groups that are possible. Corresponds to the
/// number of unique priority levels.
const MAX_GROUP_NESTING: usize = 3;

/// Grouping rules used in layout realization.
static LAYOUT_RULES: &[&GroupingRule] = &[&TEXTUAL, &PAR, &CITES, &LIST, &ENUM, &TERMS];

/// Grouping rules used in paragraph layout realization.
static LAYOUT_PAR_RULES: &[&GroupingRule] = &[&TEXTUAL, &CITES, &LIST, &ENUM, &TERMS];

/// Grouping rules used in HTML root realization.
static HTML_DOCUMENT_RULES: &[&GroupingRule] =
    &[&TEXTUAL, &PAR, &CITES, &LIST, &ENUM, &TERMS];

/// Grouping rules used in HTML fragment realization.
static HTML_FRAGMENT_RULES: &[&GroupingRule] =
    &[&TEXTUAL, &PAR, &CITES, &LIST, &ENUM, &TERMS];

/// Grouping rules used in math realization.
static MATH_RULES: &[&GroupingRule] = &[&CITES, &LIST, &ENUM, &TERMS];

/// Groups adjacent textual elements for text show rule application.
static TEXTUAL: GroupingRule = GroupingRule {
    priority: 3,
    tags: true,
    trigger: |content, _| {
        let elem = content.elem();
        // Note that `SymbolElem` converts into `TextElem` before textual show
        // rules run, and we apply textual rules to elements manually during
        // math realization, so we don't check for it here.
        elem == TextElem::ELEM
            || elem == LinebreakElem::ELEM
            || elem == SmartQuoteElem::ELEM
    },
    inner: |content| content.elem() == SpaceElem::ELEM,
    // Any kind of style interrupts this kind of grouping since regex show
    // rules cannot match over style changes anyway.
    interrupt: |_| true,
    finish: finish_textual,
};

/// Collects inline-level elements into a `ParElem`.
static PAR: GroupingRule = GroupingRule {
    priority: 1,
    tags: true,
    trigger: |content, state| {
        let elem = content.elem();
        elem == TextElem::ELEM
            || elem == HElem::ELEM
            || elem == LinebreakElem::ELEM
            || elem == SmartQuoteElem::ELEM
            || elem == InlineElem::ELEM
            || elem == BoxElem::ELEM
            || match state.kind {
                RealizationKind::HtmlDocument { is_inline, .. }
                | RealizationKind::HtmlFragment { is_inline, .. } => is_inline(content),
                _ => false,
            }
    },
    inner: |content| content.elem() == SpaceElem::ELEM,
    interrupt: |elem| elem == ParElem::ELEM || elem == AlignElem::ELEM,
    finish: finish_par,
};

/// Collects `CiteElem`s into `CiteGroup`s.
static CITES: GroupingRule = GroupingRule {
    priority: 2,
    tags: false,
    trigger: |content, _| content.elem() == CiteElem::ELEM,
    inner: |content| content.elem() == SpaceElem::ELEM,
    interrupt: |elem| {
        elem == CiteGroup::ELEM || elem == ParElem::ELEM || elem == AlignElem::ELEM
    },
    finish: finish_cites,
};

/// Builds a `ListElem` from grouped `ListItems`s.
static LIST: GroupingRule = list_like_grouping::<ListElem>();

/// Builds an `EnumElem` from grouped `EnumItem`s.
static ENUM: GroupingRule = list_like_grouping::<EnumElem>();

/// Builds a `TermsElem` from grouped `TermItem`s.
static TERMS: GroupingRule = list_like_grouping::<TermsElem>();

/// Collects `ListItemLike` elements into a `ListLike` element.
const fn list_like_grouping<T: ListLike>() -> GroupingRule {
    GroupingRule {
        priority: 2,
        tags: false,
        trigger: |content, _| content.elem() == T::Item::ELEM,
        inner: |content| {
            let elem = content.elem();
            elem == SpaceElem::ELEM || elem == ParbreakElem::ELEM
        },
        interrupt: |elem| elem == T::ELEM || elem == AlignElem::ELEM,
        finish: finish_list_like::<T>,
    }
}

/// Processes grouped textual elements.
///
/// Specifically, it searches for regex matches in grouped textual elements and
/// - if there was a match, visits the results recursively,
/// - if there was no match, tries to simply implicitly use the grouped elements
///   as part of a paragraph grouping,
/// - if that's not possible because another grouping is active, temporarily
///   disables textual grouping and revisits the elements.
fn finish_textual(Grouped { s, mut start }: Grouped) -> SourceResult<()> {
    // Try to find a regex match in the grouped textual elements. Returns early
    // if there is one.
    if visit_textual(s, start)? {
        return Ok(());
    }

    // There was no regex match, so we need to collect the text into a paragraph
    // grouping. To do that, we first terminate all non-paragraph groupings.
    if in_non_par_grouping(s) {
        let elems = s.store_slice(&s.sink[start..]);
        s.sink.truncate(start);
        finish_grouping_while(s, in_non_par_grouping)?;
        start = s.sink.len();
        s.sink.extend(elems);
    }

    // Now, there are only two options:
    // 1. We are already in a paragraph group. In this case, the elements just
    //    transparently become part of it.
    // 2. There is no group at all. In this case, we create one.
    if s.groupings.is_empty() && s.rules.iter().any(|&rule| std::ptr::eq(rule, &PAR)) {
        s.groupings.push(Grouping { start, rule: &PAR, interrupted: false });
    }

    Ok(())
}

/// Whether there is an active grouping, but it is not a `PAR` grouping.
fn in_non_par_grouping(s: &mut State) -> bool {
    s.groupings.last().is_some_and(|grouping| {
        !std::ptr::eq(grouping.rule, &PAR) || grouping.interrupted
    })
}

/// Whether there is exactly one active grouping, it is a `PAR` grouping, and it
/// spans the whole sink (with the exception of leading tags).
fn is_fully_inline(s: &State) -> bool {
    s.kind.is_fragment()
        && !s.saw_parbreak
        && match s.groupings.as_slice() {
            [grouping] => {
                std::ptr::eq(grouping.rule, &PAR)
                    && s.sink[..grouping.start].iter().all(|(c, _)| c.is::<TagElem>())
            }
            _ => false,
        }
}

/// Builds the `ParElem` from inline-level elements.
fn finish_par(mut grouped: Grouped) -> SourceResult<()> {
    // Collapse unsupported spaces in-place.
    let (sink, start) = grouped.get_mut();
    collapse_spaces(sink, start);

    // Collect the children.
    let elems = grouped.get();
    let span = select_span(elems);
    let (body, trunk) = repack(elems);

    // Create and visit the paragraph.
    let s = grouped.end();
    let elem = ParElem::new(body).pack().spanned(span);
    visit(s, s.store(elem), trunk)
}

/// Builds the `CiteGroup` from `CiteElem`s.
fn finish_cites(grouped: Grouped) -> SourceResult<()> {
    // Collect the children.
    let elems = grouped.get();
    let span = select_span(elems);
    let trunk = elems[0].1;
    let children = elems
        .iter()
        .filter_map(|(c, _)| c.to_packed::<CiteElem>())
        .cloned()
        .collect();

    // Create and visit the citation group.
    let s = grouped.end();
    let elem = CiteGroup::new(children).pack().spanned(span);
    visit(s, s.store(elem), trunk)
}

/// Builds the `ListLike` element from `ListItemLike` elements.
fn finish_list_like<T: ListLike>(grouped: Grouped) -> SourceResult<()> {
    // Collect the children.
    let elems = grouped.get();
    let span = select_span(elems);
    let tight = !elems.iter().any(|(c, _)| c.is::<ParbreakElem>());
    let styles = elems.iter().filter(|(c, _)| c.is::<T::Item>()).map(|&(_, s)| s);
    let trunk = StyleChain::trunk(styles).unwrap();
    let trunk_depth = trunk.links().count();
    let children = elems
        .iter()
        .copied()
        .filter_map(|(c, s)| {
            let item = c.to_packed::<T::Item>()?.clone();
            let local = s.suffix(trunk_depth);
            Some(T::Item::styled(item, local))
        })
        .collect();

    // Create and visit the list.
    let s = grouped.end();
    let elem = T::create(children, tight).pack().spanned(span);
    visit(s, s.store(elem), trunk)
}

/// Visit textual elements in `s.sink[start..]` and apply regex show rules to
/// them.
fn visit_textual(s: &mut State, start: usize) -> SourceResult<bool> {
    // Try to find a regex match in the grouped textual elements.
    if let Some(m) = find_regex_match_in_elems(s, &s.sink[start..]) {
        collapse_spaces(&mut s.sink, start);
        let elems = s.store_slice(&s.sink[start..]);
        s.sink.truncate(start);
        visit_regex_match(s, &elems, m)?;
        return Ok(true);
    }

    Ok(false)
}

/// Finds the leftmost regex match for this style chain in the given textual
/// elements.
///
/// Collects the element's merged textual representation into the bump arena.
/// This merging also takes into account space collapsing so that we don't need
/// to call `collapse_spaces` on every textual group, performing yet another
/// linear pass. We only collapse the spaces elements themselves on the cold
/// path where there is an actual match.
fn find_regex_match_in_elems<'a>(
    s: &State,
    elems: &[Pair<'a>],
) -> Option<RegexMatch<'a>> {
    let mut buf = BumpString::new_in(&s.arenas.bump);
    let mut base = 0;
    let mut leftmost = None;
    let mut current = StyleChain::default();
    let mut space = SpaceState::Destructive;

    for &(content, styles) in elems {
        if content.is::<TagElem>() {
            continue;
        }

        let linebreak = content.is::<LinebreakElem>();
        if linebreak && let SpaceState::Space(_) = space {
            buf.pop();
        }

        if styles != current && !buf.is_empty() {
            leftmost = find_regex_match_in_str(&buf, current);
            if leftmost.is_some() {
                break;
            }
            base += buf.len();
            buf.clear();
        }

        current = styles;
        space = if content.is::<SpaceElem>() {
            if space != SpaceState::Supportive {
                continue;
            }
            buf.push(' ');
            SpaceState::Space(0)
        } else if linebreak {
            buf.push('\n');
            SpaceState::Destructive
        } else if let Some(elem) = content.to_packed::<SmartQuoteElem>() {
            buf.push(if elem.double.get(styles) { '"' } else { '\'' });
            SpaceState::Supportive
        } else if let Some(elem) = content.to_packed::<TextElem>() {
            buf.push_str(&elem.text);
            SpaceState::Supportive
        } else {
            panic!("tried to find regex match in non-textual elements");
        };
    }

    if leftmost.is_none() {
        leftmost = find_regex_match_in_str(&buf, current);
    }

    leftmost.map(|m| RegexMatch { offset: base + m.offset, ..m })
}

/// Finds the leftmost regex match for this style chain in the given text.
fn find_regex_match_in_str<'a>(
    text: &str,
    styles: StyleChain<'a>,
) -> Option<RegexMatch<'a>> {
    let mut r = 0;
    let mut revoked = SmallBitSet::new();
    let mut leftmost: Option<(regex::Match, RecipeIndex, &Recipe)> = None;

    let depth = LazyCell::new(|| styles.recipes().count());

    for entry in styles.entries() {
        let recipe = match &**entry {
            Style::Recipe(recipe) => recipe,
            Style::Property(_) => continue,
            Style::Revocation(index) => {
                revoked.insert(index.0);
                continue;
            }
        };
        r += 1;

        let Some(Selector::Regex(regex)) = recipe.selector() else { continue };
        let Some(m) = regex.find(text) else { continue };

        // Make sure we don't get any empty matches.
        if m.range().is_empty() {
            continue;
        }

        // If we already have a match that is equally or more to the left, we're
        // not interested in this new match.
        if leftmost.is_some_and(|(p, ..)| p.start() <= m.start()) {
            continue;
        }

        // Check whether the rule is already revoked. Do it only now to not
        // compute the depth unnecessarily. We subtract 1 from r because we
        // already incremented it.
        let index = RecipeIndex(*depth - (r - 1));
        if revoked.contains(index.0) {
            continue;
        }

        leftmost = Some((m, index, recipe));
    }

    leftmost.map(|(m, id, recipe)| RegexMatch {
        offset: m.start(),
        text: m.as_str().into(),
        id,
        recipe,
        styles,
    })
}

/// Visit a match of a regular expression.
///
/// This first revisits all elements before the match, potentially slicing up
/// a text element, then the transformed match, and then the remaining elements
/// after the match.
fn visit_regex_match<'a>(
    s: &mut State<'a, '_, '_, '_>,
    elems: &[Pair<'a>],
    m: RegexMatch<'a>,
) -> SourceResult<()> {
    let match_range = m.offset..m.offset + m.text.len();

    // Replace with the correct intuitive element kind: if matching against a
    // lone symbol, return a `SymbolElem`, otherwise return a newly composed
    // `TextElem`. We should only match against a `SymbolElem` during math
    // realization (`RealizationKind::Math`).
    let piece = match elems {
        &[(lone, _)] if lone.is::<SymbolElem>() => lone.clone(),
        _ => TextElem::packed(m.text),
    };

    let context = Context::new(None, Some(m.styles));
    let output = m.recipe.apply(s.engine, context.track(), piece)?;

    let mut cursor = 0;
    let mut output = Some(output);
    let mut visit_unconsumed_match = |s: &mut State<'a, '_, '_, '_>| -> SourceResult<()> {
        if let Some(output) = output.take() {
            let revocation = Style::Revocation(m.id).into();
            let outer = s.arenas.bump.alloc(m.styles);
            let chained = outer.chain(s.arenas.styles.alloc(revocation));
            visit(s, s.store(output), chained)?;
        }
        Ok(())
    };

    for &(content, styles) in elems {
        // Just forward tags.
        if content.is::<TagElem>() {
            visit(s, content, styles)?;
            continue;
        }

        // At this point, we can have a `TextElem`, `SymbolElem`, `SpaceElem`,
        // `LinebreakElem`, or `SmartQuoteElem`. We now determine the range of
        // the element.
        let len = if let Some(elem) = content.to_packed::<TextElem>() {
            elem.text.len()
        } else if let Some(elem) = content.to_packed::<SymbolElem>() {
            elem.text.len()
        } else {
            1 // The rest are Ascii, so just one byte.
        };
        let elem_range = cursor..cursor + len;

        // If the element starts before the start of match, visit it fully or
        // sliced.
        if elem_range.start < match_range.start {
            if elem_range.end <= match_range.start {
                visit(s, content, styles)?;
            } else {
                let mut elem = content.to_packed::<TextElem>().unwrap().clone();
                elem.text = elem.text[..match_range.start - elem_range.start].into();
                visit(s, s.store(elem.pack()), styles)?;
            }
        }

        // When the match starts before this element ends, visit it.
        if match_range.start < elem_range.end {
            visit_unconsumed_match(s)?;
        }

        // If the element ends after the end of the match, visit if fully or
        // sliced.
        if elem_range.end > match_range.end {
            if elem_range.start >= match_range.end {
                visit(s, content, styles)?;
            } else {
                let mut elem = content.to_packed::<TextElem>().unwrap().clone();
                elem.text = elem.text[match_range.end - elem_range.start..].into();
                visit(s, s.store(elem.pack()), styles)?;
            }
        }

        cursor = elem_range.end;
    }

    // If the match wasn't consumed yet, visit it. This shouldn't really happen
    // in practice (we'd need to have an empty match at the end), but it's an
    // extra fail-safe.
    visit_unconsumed_match(s)?;

    Ok(())
}

/// Collapses all spaces within `buf[start..]` that are at the edges or in the
/// vicinity of destructive elements.
fn collapse_spaces(buf: &mut Vec<Pair>, start: usize) {
    let mut state = SpaceState::Destructive;
    let mut k = start;

    // We do one pass over the elements, backshifting everything as necessary
    // when a space collapses. The variable `i` is our cursor in the original
    // elements. The variable `k` is our cursor in the result. At all times, we
    // have `k <= i`, so we can do it in place.
    for i in start..buf.len() {
        let (content, styles) = buf[i];

        // Determine the next state.
        if content.is::<TagElem>() {
            // Nothing to do.
        } else if content.is::<SpaceElem>() {
            if state != SpaceState::Supportive {
                continue;
            }
            state = SpaceState::Space(k);
        } else if content.is::<LinebreakElem>() {
            destruct_space(buf, &mut k, &mut state);
        } else if let Some(elem) = content.to_packed::<HElem>() {
            if elem.amount.is_fractional() || elem.weak.get(styles) {
                destruct_space(buf, &mut k, &mut state);
            }
        } else {
            state = SpaceState::Supportive;
        };

        // Copy over normal elements (in place).
        if k < i {
            buf[k] = buf[i];
        }
        k += 1;
    }

    destruct_space(buf, &mut k, &mut state);

    // Delete all the excess that's left due to the gaps produced by spaces.
    buf.truncate(k);
}

/// Deletes a preceding space if any.
fn destruct_space(buf: &mut [Pair], end: &mut usize, state: &mut SpaceState) {
    if let SpaceState::Space(s) = *state {
        buf.copy_within(s + 1..*end, s);
        *end -= 1;
    }
    *state = SpaceState::Destructive;
}

/// Finds the first non-detached span in the list.
fn select_span(children: &[Pair]) -> Span {
    Span::find(children.iter().map(|(c, _)| c.span()))
}

/// Turn realized content with styles back into owned content and a trunk style
/// chain.
fn repack<'a>(buf: &[Pair<'a>]) -> (Content, StyleChain<'a>) {
    let trunk = StyleChain::trunk_from_pairs(buf).unwrap_or_default();
    let depth = trunk.links().count();

    let mut seq = Vec::with_capacity(buf.len());

    for (chain, group) in buf.group_by_key(|&(_, s)| s) {
        let iter = group.iter().map(|&(c, _)| c.clone());
        let suffix = chain.suffix(depth);
        if suffix.is_empty() {
            seq.extend(iter);
        } else if let &[(element, _)] = group {
            seq.push(element.clone().styled_with_map(suffix));
        } else {
            seq.push(Content::sequence(iter).styled_with_map(suffix));
        }
    }

    (Content::sequence(seq), trunk)
}
