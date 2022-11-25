use comemo::Tracked;

use super::{Content, StyleChain};
use crate::diag::SourceResult;
use crate::frame::Frame;
use crate::World;

/// Typeset content into a collection of layouted frames.
///
/// Returns either a vector of frames representing individual pages or
/// diagnostics in the form of a vector of error message with file and span
/// information.
#[comemo::memoize]
pub fn typeset(world: Tracked<dyn World>, content: &Content) -> SourceResult<Vec<Frame>> {
    let library = world.library();
    let styles = StyleChain::new(&library.styles);
    (library.items.layout)(world, content, styles)
}
