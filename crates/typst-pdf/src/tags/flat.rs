use ecow::EcoString;
use krilla::tagging::{ArtifactType, ListNumbering};
use typst_library::text::Locale;
use typst_syntax::Span;

use crate::tags::context::{BBoxId, FigureId, TableId, TagId};
use crate::tags::groups::TagStorage;
use crate::tags::resolve::TagNode;

/// A compact, flat representation of the tag tree.
///
/// After `context::finish()` completes, the rich `Groups` tree is converted
/// into this flat form. Each group becomes an entry in parallel arrays
/// (`kinds`, `spans`, `children`, `weak`, `langs`, `bboxes`), indexed by
/// the same u32 id. The original `Groups` data (including the
/// `FxHashMap<Location, ...>`) is dropped, freeing significant memory for
/// large documents.
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
    pub kinds: Vec<ResolvedGroupKind>,
    pub spans: Vec<Span>,
    pub children: Vec<Vec<TagNode>>,
    pub weak: Vec<bool>,
    /// Language for each group, extracted from GroupKind::lang().
    pub langs: Vec<Option<Option<Locale>>>,
    /// Bounding box id for each group, extracted from GroupKind::bbox().
    pub bboxes: Vec<Option<BBoxId>>,
}

impl FlatTagData {
    /// Access the kind for a group by its raw index.
    #[inline]
    pub fn kind(&self, idx: usize) -> &ResolvedGroupKind {
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

    /// Access the language for a group by its raw index.
    #[inline]
    pub fn lang(&self, idx: usize) -> Option<Option<Locale>> {
        self.langs[idx]
    }

    /// Access the bounding box id for a group by its raw index.
    #[inline]
    pub fn bbox(&self, idx: usize) -> Option<BBoxId> {
        self.bboxes[idx]
    }
}

/// Lightweight replacement for GroupKind after `Groups::flatten()`.
///
/// Stores ONLY the data needed by the resolver. Drops all `Packed<T>`,
/// `Content`, and other expensive heap-allocated data (e.g.
/// `Packed<TableCell>` ~320 bytes, `Packed<ImageElem>`, `Content`, etc.).
///
/// The `lang` and `bbox` fields are stored separately in parallel arrays
/// in `FlatTagData` so they don't inflate the enum size.
#[derive(Debug)]
#[allow(dead_code)]
pub enum ResolvedGroupKind {
    Root,
    Artifact(ArtifactType),
    LogicalParent,
    LogicalChild,
    Outline,
    OutlineEntry,
    Table(TableId),
    TableCell(TagId),
    Grid,
    GridCell,
    List(ListNumbering),
    ListItemLabel,
    ListItemBody,
    TermsItemLabel,
    TermsItemBody,
    BibEntry,
    FigureWrapper(FigureId),
    Figure(FigureId),
    FigureCaption,
    Image { alt: Option<EcoString> },
    Formula { alt: Option<EcoString>, block: bool },
    Link,
    CodeBlock,
    CodeBlockLine,
    Par,
    TextAttr,
    Transparent,
    Standard(TagId),
}

impl ResolvedGroupKind {
    /// Whether this group is an artifact.
    #[inline]
    pub fn is_artifact(&self) -> bool {
        matches!(self, Self::Artifact(_))
    }

    /// Whether this group is a link.
    #[inline]
    pub fn is_link(&self) -> bool {
        matches!(self, Self::Link)
    }
}
