use comemo::Tracked;

use super::{capability, Content, NodeId, Recipe, Selector, StyleChain};
use crate::diag::SourceResult;
use crate::World;

/// Whether the target is affected by show rules in the given style chain.
pub fn applicable(target: &Content, styles: StyleChain) -> bool {
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
    world: Tracked<dyn World>,
    target: &Content,
    styles: StyleChain,
) -> SourceResult<Option<Content>> {
    // Find out how many recipes there are.
    let mut n = styles.recipes().count();

    // Find an applicable recipe.
    let mut realized = None;
    for recipe in styles.recipes() {
        let guard = Guard::Nth(n);
        if recipe.applicable(target) && !target.is_guarded(guard) {
            if let Some(content) = try_apply(world, &target, recipe, guard)? {
                realized = Some(content);
                break;
            }
        }
        n -= 1;
    }

    // Realize if there was no matching recipe.
    if let Some(showable) = target.with::<dyn Show>() {
        let guard = Guard::Base(target.id());
        if realized.is_none() && !target.is_guarded(guard) {
            realized = Some(showable.show(world, styles));
        }
    }

    // Finalize only if this is the first application for this node.
    if let Some(node) = target.with::<dyn Finalize>() {
        if target.is_pristine() {
            if let Some(already) = realized {
                realized = Some(node.finalize(already));
            }
        }
    }

    Ok(realized)
}

/// Try to apply a recipe to the target.
fn try_apply(
    world: Tracked<dyn World>,
    target: &Content,
    recipe: &Recipe,
    guard: Guard,
) -> SourceResult<Option<Content>> {
    match &recipe.selector {
        Some(Selector::Node(id, _)) => {
            if target.id() != *id {
                return Ok(None);
            }

            recipe.apply(world, target.clone().guarded(guard)).map(Some)
        }

        Some(Selector::Label(label)) => {
            if target.label() != Some(label) {
                return Ok(None);
            }

            recipe.apply(world, target.clone().guarded(guard)).map(Some)
        }

        Some(Selector::Regex(regex)) => {
            let Some(text) = item!(text_str)(&target) else {
                return Ok(None);
            };

            let make = |s| {
                let mut content = item!(text)(s);
                content.copy_meta(&target);
                content
            };

            let mut result = vec![];
            let mut cursor = 0;

            for m in regex.find_iter(text) {
                let start = m.start();
                if cursor < start {
                    result.push(make(text[cursor..start].into()));
                }

                let piece = make(m.as_str().into()).guarded(guard);
                let transformed = recipe.apply(world, piece)?;
                result.push(transformed);
                cursor = m.end();
            }

            if result.is_empty() {
                return Ok(None);
            }

            if cursor < text.len() {
                result.push(make(text[cursor..].into()));
            }

            Ok(Some(Content::sequence(result)))
        }

        None => Ok(None),
    }
}

/// The base recipe for a node.
#[capability]
pub trait Show {
    /// Execute the base recipe for this node.
    fn show(&self, world: Tracked<dyn World>, styles: StyleChain) -> Content;
}

/// Post-process a node after it was realized.
#[capability]
pub trait Finalize {
    /// Finalize the fully realized form of the node. Use this for effects that
    /// should work even in the face of a user-defined show rule, for example
    /// the linking behaviour of a link node.
    fn finalize(&self, realized: Content) -> Content;
}

/// Guards content against being affected by the same show rule multiple times.
#[derive(Debug, Copy, Clone, PartialEq, Hash)]
pub enum Guard {
    /// The nth recipe from the top of the chain.
    Nth(usize),
    /// The [base recipe](Show) for a kind of node.
    Base(NodeId),
}
