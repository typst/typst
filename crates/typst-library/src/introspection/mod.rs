//! Interaction between document parts.

mod convergence;
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

pub use self::convergence::*;
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

use crate::foundations::Scope;

/// Hook up all `introspection` definitions.
pub fn define(global: &mut Scope) {
    global.start_category(crate::Category::Introspection);
    global.define_type::<Location>();
    global.define_type::<Counter>();
    global.define_type::<State>();
    global.define_elem::<MetadataElem>();
    global.define_func::<here>();
    global.define_func::<query>();
    global.define_func::<locate>();
    global.reset_category();
}
