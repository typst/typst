//! Interaction between document parts.

mod bibliography;
mod cite;
mod context;
mod counter;
mod document;
mod figure;
mod footnote;
mod heading;
mod link;
mod metadata;
#[path = "numbering.rs"]
mod numbering_;
mod outline;
#[path = "query.rs"]
mod query_;
mod reference;
mod state;

pub use self::bibliography::*;
pub use self::cite::*;
pub use self::context::*;
pub use self::counter::*;
pub use self::document::*;
pub use self::figure::*;
pub use self::footnote::*;
pub use self::heading::*;
pub use self::link::*;
pub use self::metadata::*;
pub use self::numbering_::*;
pub use self::outline::*;
pub use self::query_::*;
pub use self::reference::*;
pub use self::state::*;

use crate::prelude::*;
use crate::text::TextElem;

/// Hook up all meta definitions.
pub(super) fn define(global: &mut Scope) {
    global.category("meta");
    global.define_type::<Label>();
    global.define_type::<Selector>();
    global.define_type::<Location>();
    global.define_type::<Counter>();
    global.define_type::<State>();
    global.define_elem::<DocumentElem>();
    global.define_elem::<RefElem>();
    global.define_elem::<LinkElem>();
    global.define_elem::<OutlineElem>();
    global.define_elem::<HeadingElem>();
    global.define_elem::<FigureElem>();
    global.define_elem::<FootnoteElem>();
    global.define_elem::<CiteElem>();
    global.define_elem::<BibliographyElem>();
    global.define_elem::<MetadataElem>();
    global.define_func::<locate>();
    global.define_func::<style>();
    global.define_func::<layout>();
    global.define_func::<numbering>();
    global.define_func::<query>();
}

/// An element that has a local name.
pub trait LocalNameIn: LocalName {
    /// Gets the local name from the style chain.
    fn local_name_in(styles: StyleChain) -> &'static str
    where
        Self: Sized,
    {
        Self::local_name(TextElem::lang_in(styles), TextElem::region_in(styles))
    }
}

impl<T: LocalName> LocalNameIn for T {}
