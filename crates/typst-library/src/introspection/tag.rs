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
    Start(Content, TagFlags),
    /// The element with the given location and key hash ends here.
    ///
    /// Note: The key hash is stored here instead of in `Start` simply to make
    /// the two enum variants more balanced in size, keeping a `Tag`'s memory
    /// size down. There are no semantic reasons for this.
    End(Location, u128, TagFlags),
}

impl Tag {
    /// Access the location of the tag.
    pub fn location(&self) -> Location {
        match self {
            Tag::Start(elem, ..) => elem.location().unwrap(),
            Tag::End(loc, ..) => *loc,
        }
    }
}

impl Debug for Tag {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        let loc = self.location();
        match self {
            Tag::Start(elem, ..) => write!(f, "Start({:?}, {loc:?})", elem.elem().name()),
            Tag::End(..) => write!(f, "End({loc:?})"),
        }
    }
}

#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
pub struct TagFlags {
    /// Whether the element is [`Locatable`](super::Locatable).
    pub locatable: bool,
    /// Whether the element is [`Tagged`](super::Tagged).
    pub tagged: bool,
    /// Whether the element has a [`Label`](crate::foundations::Label).
    pub labelled: bool,
}

impl TagFlags {
    pub fn any(&self) -> bool {
        self.locatable || self.tagged || self.labelled
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
