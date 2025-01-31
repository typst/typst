use comemo::Tracked;

use crate::diag::HintedStrResult;
use crate::foundations::{func, Context};
use crate::introspection::Location;

/// Provides the current location in the document.
///
/// You can think of `here` as a low-level building block that directly extracts
/// the current location from the active [context]. Some other functions use it
/// internally: For instance, `{counter.get()}` is equivalent to
/// `{counter.at(here())}`.
///
/// Within show rules on [locatable]($location/#locatable) elements, `{here()}`
/// will match the location of the shown element.
///
/// If you want to display the current page number, refer to the documentation
/// of the [`counter`] type. While `here` can be used to determine the physical
/// page number, typically you want the logical page number that may, for
/// instance, have been reset after a preface.
///
/// # Examples
/// Determining the current position in the document in combination with the
/// [`position`]($location.position) method:
/// ```example
/// #context [
///   I am located at
///   #here().position()
/// ]
/// ```
///
/// Running a [query] for elements before the current position:
/// ```example
/// = Introduction
/// = Background
///
/// There are
/// #context query(
///   selector(heading).before(here())
/// ).len()
/// headings before me.
///
/// = Conclusion
/// ```
/// Refer to the [`selector`] type for more details on before/after selectors.
#[func(contextual)]
pub fn here(context: Tracked<Context>) -> HintedStrResult<Location> {
    context.location()
}
