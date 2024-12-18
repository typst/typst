use comemo::Tracked;

use crate::diag::HintedStrResult;
use crate::engine::Engine;
use crate::foundations::{func, Context, LocatableSelector};
use crate::introspection::Location;

/// Determines the location of an element in the document.
///
/// Takes a selector that must match exactly one element and returns that
/// element's [`location`]. This location can, in particular, be used to
/// retrieve the physical [`page`]($location.page) number and
/// [`position`]($location.position) (page, x, y) for that element.
///
/// # Examples
/// Locating a specific element:
/// ```example
/// #context [
///   Introduction is at: \
///   #locate(<intro>).position()
/// ]
///
/// = Introduction <intro>
/// ```
#[func(contextual)]
pub fn locate(
    /// The engine.
    engine: &mut Engine,
    /// The callsite context.
    context: Tracked<Context>,
    /// A selector that should match exactly one element. This element will be
    /// located.
    ///
    /// Especially useful in combination with
    /// - [`here`] to locate the current context,
    /// - a [`location`] retrieved from some queried element via the
    ///   [`location()`]($content.location) method on content.
    selector: LocatableSelector,
) -> HintedStrResult<Location> {
    selector.resolve_unique(engine.introspector, context)
}
