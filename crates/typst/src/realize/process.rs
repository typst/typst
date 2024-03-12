use std::cell::OnceCell;

use comemo::{Track, Tracked};
use smallvec::smallvec;

use crate::diag::SourceResult;
use crate::engine::Engine;
use crate::foundations::{
    Content, Context, Packed, Recipe, RecipeIndex, Regex, Selector, Show, ShowSet, Style,
    StyleChain, Styles, Synthesize, Transformation,
};
use crate::introspection::{Locatable, Meta, MetaElem};
use crate::text::TextElem;
use crate::util::{hash128, BitSet};

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

/// Whether the `target` element needs processing.
pub fn processable<'a>(
    engine: &mut Engine,
    target: &'a Content,
    styles: StyleChain<'a>,
) -> bool {
    verdict(engine, target, styles).is_some()
}

/// Processes the given `target` element when encountering it during realization.
pub fn process(
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

        // Special handling for show-set rules. Exception: Regex show rules,
        // those need to be handled like normal transformations.
        if let (Transformation::Style(transform), false) =
            (&recipe.transform, matches!(&recipe.selector, Some(Selector::Regex(_))))
        {
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
        ShowStep::Recipe(recipe, guard) => {
            let context = Context::new(target.location(), Some(styles));
            match &recipe.selector {
                // If the selector is a regex, the `target` is guaranteed to be a
                // text element. This invokes special regex handling.
                Some(Selector::Regex(regex)) => {
                    let text = target.into_packed::<TextElem>().unwrap();
                    show_regex(engine, &text, regex, recipe, guard, context.track())
                }

                // Just apply the recipe.
                _ => recipe.apply(engine, context.track(), target.guarded(guard)),
            }
        }

        // If the verdict picks this step, the `target` is guaranteed to have a
        // built-in show rule.
        ShowStep::Builtin => target.with::<dyn Show>().unwrap().show(engine, styles),
    }
}

/// Apply a regex show rule recipe to a target.
fn show_regex(
    engine: &mut Engine,
    target: &Packed<TextElem>,
    regex: &Regex,
    recipe: &Recipe,
    index: RecipeIndex,
    context: Tracked<Context>,
) -> SourceResult<Content> {
    let make = |s: &str| {
        let mut fresh = target.clone();
        fresh.push_text(s.into());
        fresh.pack()
    };

    let mut result = vec![];
    let mut cursor = 0;

    let text = target.text();

    for m in regex.find_iter(target.text()) {
        let start = m.start();
        if cursor < start {
            result.push(make(&text[cursor..start]));
        }

        let piece = make(m.as_str());
        let transformed = recipe.apply(engine, context, piece)?;
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
