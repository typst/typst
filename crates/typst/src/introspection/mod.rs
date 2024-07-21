//! Interaction between document parts.

mod counter;
#[path = "here.rs"]
mod here_;
mod introspector;
#[path = "locate.rs"]
mod locate_;
mod location;
mod locator;
mod metadata;
#[path = "query.rs"]
mod query_;
mod state;

pub use self::counter::*;
pub use self::here_::*;
pub use self::introspector::*;
pub use self::locate_::*;
pub use self::location::*;
pub use self::locator::*;
pub use self::metadata::*;
pub use self::query_::*;
pub use self::state::*;

use std::fmt::{self, Debug, Formatter};

use crate::diag::{bail, SourceResult};
use crate::engine::Engine;
use crate::foundations::{
    category, elem, Args, Category, Construct, Content, NativeElement, Packed, Scope,
    Unlabellable,
};
use crate::realize::{Behave, Behaviour};

/// Interactions between document parts.
///
/// This category is home to Typst's introspection capabilities: With the
/// `counter` function, you can access and manipulate page, section, figure, and
/// equation counters or create custom ones. Meanwhile, the `query` function
/// lets you search for elements in the document to construct things like a list
/// of figures or headers which show the current chapter title.
///
/// Most of the functions are _contextual._ It is recommended to read the chapter
/// on [context] before continuing here.
#[category]
pub static INTROSPECTION: Category;

/// Hook up all `introspection` definitions.
pub fn define(global: &mut Scope) {
    global.category(INTROSPECTION);
    global.define_type::<Location>();
    global.define_type::<Counter>();
    global.define_type::<State>();
    global.define_elem::<MetadataElem>();
    global.define_func::<here>();
    global.define_func::<query>();
    global.define_func::<locate>();
}

/// Holds a tag for a locatable element that was realized.
///
/// The `TagElem` is handled by all layouters. The held element becomes
/// available for introspection in the next compiler iteration.
#[elem(Behave, Unlabellable, Construct)]
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

impl Behave for Packed<TagElem> {
    fn behaviour(&self) -> Behaviour {
        Behaviour::Invisible
    }
}

/// Holds a locatable element that was realized.
#[derive(Clone, PartialEq, Hash)]
pub struct Tag {
    /// The introspectible element.
    pub elem: Content,
    /// The element's key hash, which forms the base of its location (but is
    /// locally disambiguated and combined with outer hashes).
    ///
    /// We need to retain this for introspector-assisted location assignment
    /// during measurement.
    pub(crate) key: u128,
}

impl Tag {
    /// Create a tag from an element and its key hash.
    pub fn new(elem: Content, key: u128) -> Self {
        Self { elem, key }
    }
}

impl Debug for Tag {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        write!(f, "Tag({:?})", self.elem.elem().name())
    }
}
