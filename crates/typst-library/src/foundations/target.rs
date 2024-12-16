use comemo::Tracked;

use crate::diag::HintedStrResult;
use crate::foundations::{elem, func, Cast, Context};

/// The compilation target.
#[derive(Debug, Default, Copy, Clone, PartialEq, Hash, Cast)]
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

/// Returns the current compilation target.
#[func(contextual)]
pub fn target(
    /// The callsite context.
    context: Tracked<Context>,
) -> HintedStrResult<Target> {
    Ok(TargetElem::target_in(context.styles()?))
}
