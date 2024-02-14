use typed_arena::Arena;

use crate::foundations::{Content, StyleChain};

/// Temporary storage arenas for building.
#[derive(Default)]
pub struct Arenas<'a> {
    chains: Arena<StyleChain<'a>>,
    content: Arena<Content>,
}

impl<'a> Arenas<'a> {
    /// Store a value in the matching arena.
    pub fn store<T: Store<'a>>(&'a self, val: T) -> &'a T {
        val.store(self)
    }
}

/// Implemented by storable types.
pub trait Store<'a> {
    fn store(self, arenas: &'a Arenas<'a>) -> &'a Self;
}

impl<'a> Store<'a> for Content {
    fn store(self, arenas: &'a Arenas<'a>) -> &'a Self {
        arenas.content.alloc(self)
    }
}

impl<'a> Store<'a> for StyleChain<'a> {
    fn store(self, arenas: &'a Arenas<'a>) -> &'a Self {
        arenas.chains.alloc(self)
    }
}
