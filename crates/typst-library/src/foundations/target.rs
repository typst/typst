use comemo::Tracked;

use crate::diag::HintedStrResult;
use crate::foundations::{Cast, Context, elem, func};

/// The export target.
#[derive(Debug, Default, Copy, Clone, Eq, PartialEq, Hash, Cast)]
pub enum Target {
    /// The target that is used for paged, fully laid-out content.
    #[default]
    Paged,
    /// The target that is used for HTML export.
    Html,
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
/// guarded behind the `html` feature. Visit the [HTML documentation
/// page]($html) for more details.
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
