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

use ecow::{eco_format, EcoString};
use smallvec::SmallVec;

use crate::foundations::{
    category, elem, ty, Category, Content, Packed, Repr, Scope, Unlabellable,
};
use crate::model::Destination;
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

/// Hosts metadata and ensures metadata is produced even for empty elements.
#[elem(Behave, Unlabellable)]
pub struct MetaElem {
    /// Metadata that should be attached to all elements affected by this style
    /// property.
    ///
    /// This must be accessed and applied to all frames produced by elements
    /// that manually handle styles (because their children can have varying
    /// styles). This currently includes flow, par, and equation.
    ///
    /// Other elements don't manually need to handle it because their parents
    /// that result from realization will take care of it and the metadata can
    /// only apply to them as a whole, not part of it (because they don't manage
    /// styles).
    #[fold]
    pub data: SmallVec<[Meta; 1]>,
}

impl Unlabellable for Packed<MetaElem> {}

impl Behave for Packed<MetaElem> {
    fn behaviour(&self) -> Behaviour {
        Behaviour::Invisible
    }
}

/// Meta information that isn't visible or renderable.
#[ty]
#[derive(Clone, PartialEq, Hash)]
pub enum Meta {
    /// An internal or external link to a destination.
    Link(Destination),
    /// An identifiable element that produces something within the area this
    /// metadata is attached to.
    Elem(Content),
    /// Indicates that content should be hidden. This variant doesn't appear
    /// in the final frames as it is removed alongside the content that should
    /// be hidden.
    Hide,
}

impl Debug for Meta {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        match self {
            Self::Link(dest) => write!(f, "Link({dest:?})"),
            Self::Elem(content) => write!(f, "Elem({:?})", content.func()),
            Self::Hide => f.pad("Hide"),
        }
    }
}

impl Repr for Meta {
    fn repr(&self) -> EcoString {
        eco_format!("{self:?}")
    }
}
