use std::fmt::{self, Debug, Formatter};

use crate::diag::{bail, SourceResult};
use crate::engine::Engine;
use crate::foundations::{
    elem, Args, Construct, Content, NativeElement, Packed, Unlabellable,
};
use crate::introspection::Location;

/// Holds a locatable element that was realized.
#[derive(Clone, PartialEq, Hash)]
pub struct Tag {
    /// Whether this is a start or end tag.
    kind: TagKind,
    /// The introspectible element.
    elem: Content,
    /// The element's key hash.
    key: u128,
}

impl Tag {
    /// Create a start tag from an element and its key hash.
    ///
    /// Panics if the element does not have a [`Location`].
    #[track_caller]
    pub fn new(elem: Content, key: u128) -> Self {
        assert!(elem.location().is_some());
        Self { elem, key, kind: TagKind::Start }
    }

    /// Returns the same tag with the given kind.
    pub fn with_kind(self, kind: TagKind) -> Self {
        Self { kind, ..self }
    }

    /// Whether this is a start or end tag.
    pub fn kind(&self) -> TagKind {
        self.kind
    }

    /// The locatable element that the tag holds.
    pub fn elem(&self) -> &Content {
        &self.elem
    }

    /// Access the location of the element.
    pub fn location(&self) -> Location {
        self.elem.location().unwrap()
    }

    /// The element's key hash, which forms the base of its location (but is
    /// locally disambiguated and combined with outer hashes).
    ///
    /// We need to retain this for introspector-assisted location assignment
    /// during measurement.
    pub fn key(&self) -> u128 {
        self.key
    }
}

impl Debug for Tag {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        write!(f, "Tag({:?}, {:?})", self.kind, self.elem.elem().name())
    }
}

/// Determines whether a tag marks the start or end of an element.
#[derive(Debug, Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub enum TagKind {
    /// The tag indicates that the element starts here.
    Start,
    /// The tag indicates that the element end here.
    End,
}

/// Holds a tag for a locatable element that was realized.
///
/// The `TagElem` is handled by all layouters. The held element becomes
/// available for introspection in the next compiler iteration.
#[elem(Construct, Unlabellable)]
pub struct TagElem {
    /// The introspectible element.
    #[required]
    #[internal]
    pub tag: Tag,
}

impl TagElem {
    /// Create a packed tag element.
    pub fn packed(tag: Tag) -> Content {
        let mut content = Self::new(tag).pack();
        // We can skip preparation for the `TagElem`.
        content.mark_prepared();
        content
    }
}

impl Construct for TagElem {
    fn construct(_: &mut Engine, args: &mut Args) -> SourceResult<Content> {
        bail!(args.span, "cannot be constructed manually")
    }
}

impl Unlabellable for Packed<TagElem> {}
