use typst_syntax::Span;

use crate::tags::groups::{GroupKind, TagStorage};
use crate::tags::resolve::TagNode;

/// A compact, flat representation of the tag tree.
///
/// After `context::finish()` completes, the rich `Groups` tree is converted
/// into this flat form. Each group becomes an entry in parallel arrays
/// (`kinds`, `spans`, `children`, `weak`), indexed by the same u32 id.
/// The original `Groups` data (including the `FxHashMap<Location, ...>`)
/// is dropped, freeing significant memory for large documents.
///
/// The resolver then walks the immutable `FlatTagData` by reference and
/// mutates only `TagStorage` (to take tag kinds for table cells and
/// standard tags).
pub struct FlatTagTree {
    pub data: FlatTagData,
    pub tag_storage: TagStorage,
}

/// Immutable tree structure data. The resolver borrows this by shared
/// reference so it can iterate children without cloning.
pub struct FlatTagData {
    pub kinds: Vec<GroupKind>,
    pub spans: Vec<Span>,
    pub children: Vec<Vec<TagNode>>,
    pub weak: Vec<bool>,
    #[allow(dead_code)]
    pub parent: Vec<u32>,
}

impl FlatTagData {
    /// Access the kind for a group by its raw index.
    #[inline]
    pub fn kind(&self, idx: usize) -> &GroupKind {
        &self.kinds[idx]
    }

    /// Access the span for a group by its raw index.
    #[inline]
    pub fn span(&self, idx: usize) -> Span {
        self.spans[idx]
    }

    /// Access the children for a group by its raw index.
    #[inline]
    pub fn children(&self, idx: usize) -> &[TagNode] {
        &self.children[idx]
    }

    /// Whether the group at this index is weak.
    #[inline]
    pub fn is_weak(&self, idx: usize) -> bool {
        self.weak[idx]
    }
}
