use super::{
    Content, Element, MetaElem, NativeElement, Recipe, Selector, StyleChain, Vt,
};
use crate::diag::SourceResult;
use crate::doc::Meta;
use crate::util::hash128;

/// Whether the target is affected by show rules in the given style chain.
pub fn applicable(target: &Content, styles: StyleChain) -> bool {
    if target.needs_preparation() {
        return true;
    }

    if target.can::<dyn Show>() && target.is_pristine() {
        return true;
    }

    // Find out how many recipes there are.
    let mut n = styles.recipes().count();

    // Find out whether any recipe matches and is unguarded.
    for recipe in styles.recipes() {
        if recipe.applicable(target) && !target.is_guarded(Guard::Nth(n)) {
            return true;
        }
        n -= 1;
    }

    false
}

/// Apply the show rules in the given style chain to a target.
pub fn realize(
    vt: &mut Vt,
    target: &Content,
    styles: StyleChain,
) -> SourceResult<Option<Content>> {
    // Pre-process.
    if target.needs_preparation() {
        let mut elem = target.clone();
        if target.can::<dyn Locatable>() || target.label().is_some() {
            let location = vt.locator.locate(hash128(target));
            elem.set_location(location);
        }

        if let Some(elem) = elem.with_mut::<dyn Synthesize>() {
            elem.synthesize(vt, styles)?;
        }

        elem.mark_prepared();

        if elem.location().is_some() {
            let span = elem.span();
            let meta = Meta::Elem(elem.clone());
            return Ok(Some(
                (elem + MetaElem::new().pack().spanned(span))
                    .styled(MetaElem::set_data(vec![meta])),
            ));
        }

        return Ok(Some(elem));
    }

    // Find out how many recipes there are.
    let mut n = styles.recipes().count();

    // Find an applicable recipe.
    let mut realized = None;
    for recipe in styles.recipes() {
        let guard = Guard::Nth(n);
        if recipe.applicable(target) && !target.is_guarded(guard) {
            if let Some(content) = try_apply(vt, target, recipe, guard)? {
                realized = Some(content);
                break;
            }
        }
        n -= 1;
    }

    // Realize if there was no matching recipe.
    if let Some(showable) = target.with::<dyn Show>() {
        let guard = Guard::Base(target.func());
        if realized.is_none() && !target.is_guarded(guard) {
            realized = Some(showable.show(vt, styles)?);
        }
    }

    // Finalize only if this is the first application for this element.
    if let Some(elem) = target.with::<dyn Finalize>() {
        if target.is_pristine() {
            if let Some(already) = realized {
                realized = Some(elem.finalize(already, styles));
            }
        }
    }

    Ok(realized)
}

/// Try to apply a recipe to the target.
fn try_apply(
    vt: &mut Vt,
    target: &Content,
    recipe: &Recipe,
    guard: Guard,
) -> SourceResult<Option<Content>> {
    match &recipe.selector {
        Some(Selector::Elem(element, _)) => {
            if target.func() != *element {
                return Ok(None);
            }

            recipe.apply_vt(vt, target.clone().guarded(guard)).map(Some)
        }

        Some(Selector::Label(label)) => {
            if target.label() != Some(label) {
                return Ok(None);
            }

            recipe.apply_vt(vt, target.clone().guarded(guard)).map(Some)
        }

        Some(Selector::Regex(regex)) => {
            let Some(text) = item!(text_str)(target) else {
                return Ok(None);
            };

            let make = |s: &str| target.clone().with_field("text", s);
            let mut result = vec![];
            let mut cursor = 0;

            for m in regex.find_iter(&text) {
                let start = m.start();
                if cursor < start {
                    result.push(make(&text[cursor..start]));
                }

                let piece = make(m.as_str()).guarded(guard);
                let transformed = recipe.apply_vt(vt, piece)?;
                result.push(transformed);
                cursor = m.end();
            }

            if result.is_empty() {
                return Ok(None);
            }

            if cursor < text.len() {
                result.push(make(&text[cursor..]));
            }

            Ok(Some(Content::sequence(result)))
        }

        // Not supported here.
        Some(
            Selector::Or(_)
            | Selector::And(_)
            | Selector::Location(_)
            | Selector::Can(_)
            | Selector::Before { .. }
            | Selector::After { .. },
        ) => Ok(None),

        None => Ok(None),
    }
}

/// Makes this element locatable through `vt.locate`.
pub trait Locatable {}

/// Synthesize fields on an element. This happens before execution of any show
/// rule.
pub trait Synthesize {
    /// Prepare the element for show rule application.
    fn synthesize(&mut self, vt: &mut Vt, styles: StyleChain) -> SourceResult<()>;
}

/// The base recipe for an element.
pub trait Show {
    /// Execute the base recipe for this element.
    fn show(&self, vt: &mut Vt, styles: StyleChain) -> SourceResult<Content>;
}

/// Post-process an element after it was realized.
pub trait Finalize {
    /// Finalize the fully realized form of the element. Use this for effects
    /// that should work even in the face of a user-defined show rule.
    fn finalize(&self, realized: Content, styles: StyleChain) -> Content;
}

/// How the element interacts with other elements.
pub trait Behave {
    /// The element's interaction behaviour.
    fn behaviour(&self) -> Behaviour;

    /// Whether this weak element is larger than a previous one and thus picked
    /// as the maximum when the levels are the same.
    #[allow(unused_variables)]
    fn larger(
        &self,
        prev: &(Content, Behaviour, StyleChain),
        styles: StyleChain,
    ) -> bool {
        false
    }
}

/// How an element interacts with other elements in a stream.
#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum Behaviour {
    /// A weak element which only survives when a supportive element is before
    /// and after it. Furthermore, per consecutive run of weak elements, only
    /// one survives: The one with the lowest weakness level (or the larger one
    /// if there is a tie).
    Weak(usize),
    /// An element that enables adjacent weak elements to exist. The default.
    Supportive,
    /// An element that destroys adjacent weak elements.
    Destructive,
    /// An element that does not interact at all with other elements, having the
    /// same effect as if it didn't exist, but has a visual representation.
    Ignorant,
    /// An element that does not have a visual representation.
    Invisible,
}

/// Guards content against being affected by the same show rule multiple times.
#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
pub enum Guard {
    /// The nth recipe from the top of the chain.
    Nth(usize),
    /// The [base recipe](Show) for a kind of element.
    Base(Element),
}
