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
mod tag;

pub use self::counter::*;
pub use self::here_::*;
pub use self::introspector::*;
pub use self::locate_::*;
pub use self::location::*;
pub use self::locator::*;
pub use self::metadata::*;
pub use self::query_::*;
pub use self::state::*;
pub use self::tag::*;

use crate::foundations::{category, Category, Scope};

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
    global.start_category(INTROSPECTION);
    global.define_type::<Location>();
    global.define_type::<Counter>();
    global.define_type::<State>();
    global.define_elem::<MetadataElem>();
    global.define_func::<here>();
    global.define_func::<query>();
    global.define_func::<locate>();
}
