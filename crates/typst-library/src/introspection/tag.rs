use std::fmt::{self, Debug, Formatter};

use crate::diag::{SourceResult, bail};
use crate::engine::Engine;
use crate::foundations::{
    Args, Construct, Content, NativeElement, Packed, Unlabellable, elem,
};
use crate::introspection::Location;

/// Marks the start or end of a locatable element.
#[derive(Clone, PartialEq, Hash)]
pub enum Tag {
    /// The stored element starts here.
    ///
    /// Content placed in a tag **must** have a [`Location`] or there will be
    /// panics.
    Start(Content),
    /// The element with the given location and key hash ends here.
    ///
    /// Note: The key hash is stored here instead of in `Start` simply to make
    /// the two enum variants more balanced in size, keeping a `Tag`'s memory
    /// size down. There are no semantic reasons for this.
    End(Location, u128),
}

impl Tag {
    /// Access the location of the tag.
    pub fn location(&self) -> Location {
        match self {
            Tag::Start(elem) => elem.location().unwrap(),
            Tag::End(loc, _) => *loc,
        }
    }
}

impl Debug for Tag {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        match self {
            Tag::Start(elem) => write!(f, "Start({:?})", elem.elem().name()),
            Tag::End(..) => f.pad("End"),
        }
    }
}

/// Holds a tag for a locatable element that was realized.
///
/// The `TagElem` is handled by all layouters. The held element becomes
/// available for introspection in the next compiler iteration.
#[elem(Construct, Unlabellable)]
pub struct TagElem {
    /// The introspectable element.
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
