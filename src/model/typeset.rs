use comemo::Tracked;

use super::{Content, StyleChain};
use crate::diag::SourceResult;
use crate::doc::Document;
use crate::World;

/// Typeset content into a fully layouted document.
#[comemo::memoize]
pub fn typeset(world: Tracked<dyn World>, content: &Content) -> SourceResult<Document> {
    let library = world.library();
    let styles = StyleChain::new(&library.styles);
    (library.items.layout)(world, content, styles)
}
