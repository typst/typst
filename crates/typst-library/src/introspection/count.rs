use comemo::Tracked;

use crate::diag::HintedStrResult;
use crate::engine::Engine;
use crate::foundations::{func, Array, Context, IntoValue, LocatableSelector, Selector};

/// Counts element in the document.
///
/// The `count` function lets you count elements in your document of a
/// particular type or with a particular label. It also allows to count them
/// in a hierachical way. To use it, you first need to ensure that [context]
/// is available.
///
/// Always returns an array of integers, even if called with just a single
/// target.

/// # Counting elements - simple
///
/// To just count elements of a single type/label up to the current location,
/// pass the selector you want to count:
/// ```example
/// = Heading
///
/// = Another Heading
///
/// #context count(heading)
///
/// = Third Heading
/// ```
///
/// Note that it will not return an integer, but an array with a single integer
/// entry.

/// # Counting elements - hierarchical
///
/// If you pass multiple targets, then it starts by counting the first target.
/// Then the second target is counted _starting only from the last counted
/// element of the first target_.
///
/// ```example
/// = Some Heading
///
/// == Some Subheading
///
/// = Another Heading
///
/// == Some Subheading
///
/// #context count(heading.where(level: 1), heading.where(level: 2))
///
/// == Another Subheading
/// ```
#[func(contextual)]
pub fn count(
    /// The engine.
    engine: &mut Engine,
    /// The callsite context.
    context: Tracked<Context>,
    /// Each target can be
    /// - an element function like a `heading` or `figure`,
    /// - a `{<label>}`,
    /// - a more complex selector like `{heading.where(level: 1)}`,
    /// - or `{selector(heading).before(here())}`.
    ///
    /// Only [locatable]($location/#locatable) element functions are supported.
    #[variadic]
    targets: Vec<LocatableSelector>,
    /// When passing this argument, it will count everything only from a
    /// certain location on.
    /// The selector must match exactly one element in the document. The most
    /// useful kinds of selectors for this are [labels]($label) and
    /// [locations]($location).
    // TODO remove Option as soon as there is a special `start` location
    #[named]
    after: Option<LocatableSelector>,
) -> HintedStrResult<Array> {
    // NOTE this could be made more efficient
    // one could directly get a slice &[Selector] from Vec<LocatableSelector>
    // by using #[repr(transparent)] on LocatableSelector
    let selectors: Vec<Selector> = targets.into_iter().map(|sel| sel.0).collect();
    // TODO add argument "at" with default value "here"
    // and compute `before` accordingly
    let before = Some(context.location()?);
    let after = match after {
        Some(selector) => Some(selector.resolve_unique(engine.introspector, context)?),
        None => None,
    };

    let nums = engine.introspector.count(&selectors, after, before);

    Ok(nums.into_iter().map(IntoValue::into_value).collect())
}
