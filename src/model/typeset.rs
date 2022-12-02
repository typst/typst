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
    let mut vt = Vt { world };
    (library.items.layout)(&mut vt, content, styles)
}

/// A virtual typesetter.
///
/// Holds the state needed to [typeset] content. This is the equivalent to the
/// [Vm](super::Vm) for typesetting.
pub struct Vt<'a> {
    /// The compilation environment.
    #[doc(hidden)]
    pub world: Tracked<'a, dyn World>,
}

impl<'a> Vt<'a> {
    /// Access the underlying world.
    pub fn world(&self) -> Tracked<'a, dyn World> {
        self.world
    }
}
