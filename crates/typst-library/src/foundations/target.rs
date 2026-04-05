use std::any::Any;

use comemo::Tracked;

use crate::diag::{HintedStrResult, SourceResult};
use crate::engine::Engine;
use crate::foundations::{Cast, Content, Context, StyleChain, elem, func};
use crate::introspection::Introspector;

/// A compilation output for a particular target.
///
/// Has a 1-1 relationship with the variants of [`Target`].
pub trait Output: Any {
    /// The target associated with the output.
    fn target() -> Target
    where
        Self: Sized;

    /// Creates the output.
    fn create(
        engine: &mut Engine,
        content: &Content,
        styles: StyleChain,
    ) -> SourceResult<Self>
    where
        Self: Sized;

    /// Get the output's introspector.
    fn introspector(&self) -> &dyn Introspector;

    /// Drop heavy page data to free memory, keeping only the introspector.
    /// Used in the convergence loop where historical documents only need
    /// their introspector for the next iteration.
    fn drop_pages(&mut self) {}
}

/// A trait for accepting an arbitrary kind of output as n argument.
///
/// Can be used to accept a reference to
/// - any kind of sized type that implements [`Output`], or
/// - the trait object [`&dyn Output`].
///
/// Should be used as `impl AsOutput` rather than `&impl AsOutput`.
///
/// # Why is this needed?
/// Unfortunately, `&impl Output` can't be turned into `&dyn Output` in a
/// generic function. Directly accepting `&dyn Output` is of course also
/// possible, but is less convenient, especially in cases where the document is
/// optional.
///
/// See also
/// <https://users.rust-lang.org/t/converting-from-generic-unsized-parameter-to-trait-object/72376>
pub trait AsOutput {
    /// Turns the reference into the trait object.
    fn as_output(&self) -> &dyn Output;
}

impl AsOutput for &dyn Output {
    fn as_output(&self) -> &dyn Output {
        *self
    }
}

impl<T: Output> AsOutput for &T {
    fn as_output(&self) -> &dyn Output {
        *self
    }
}

/// The compilation target.
#[derive(Debug, Default, Copy, Clone, Eq, PartialEq, Hash, Cast)]
pub enum Target {
    /// The target that is used for paged, fully laid-out content.
    #[default]
    Paged,
    /// The target that is used for HTML export.
    Html,
    /// The target for _bundle_ export. This export target can produce multiple
    /// [documents]($document) and [assets]($asset) from a single Typst project.
    Bundle,
}

impl Target {
    /// Whether this is the HTML target.
    pub fn is_html(self) -> bool {
        self == Self::Html
    }
}

/// This element exists solely to host the `target` style chain field.
/// It is never constructed and not visible to users.
#[elem]
pub struct TargetElem {
    /// The compilation target.
    pub target: Target,
}

/// Returns the current export target.
///
/// This function returns either
/// - `{"paged"}` (for PDF, PNG, and SVG export), or
/// - `{"html"}` (for HTML export).
///
/// The design of this function is not yet finalized and for this reason it is
/// guarded behind the `html` and `bundle` features (enabling either one makes
/// the function available). Visit the [HTML documentation page]($html) for more
/// details.
///
/// # When to use it
/// This function allows you to format your document properly across both HTML
/// and paged export targets. It should primarily be used in templates and show
/// rules, rather than directly in content. This way, the document's contents
/// can be fully agnostic to the export target and content can be shared between
/// PDF and HTML export.
///
/// # Varying targets
/// This function is [contextual]($context) as the target can vary within a
/// single compilation: When exporting to HTML, the target will be `{"paged"}`
/// while within an [`html.frame`].
///
/// # Example
/// ```example
/// #let kbd(it) = context {
///   if target() == "html" {
///     html.elem("kbd", it)
///   } else {
///     set text(fill: rgb("#1f2328"))
///     let r = 3pt
///     box(
///       fill: rgb("#f6f8fa"),
///       stroke: rgb("#d1d9e0b3"),
///       outset: (y: r),
///       inset: (x: r),
///       radius: r,
///       raw(it)
///     )
///   }
/// }
///
/// Press #kbd("F1") for help.
/// ```
#[func(contextual)]
pub fn target(context: Tracked<Context>) -> HintedStrResult<Target> {
    Ok(context.styles()?.get(TargetElem::target))
}
