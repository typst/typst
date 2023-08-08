//! Interaction between document parts.

mod bibliography;
mod context;
mod counter;
mod document;
mod figure;
mod footnote;
mod heading;
mod link;
mod metadata;
mod numbering;
mod outline;
mod query;
mod reference;
mod state;

pub use self::bibliography::*;
pub use self::context::*;
pub use self::counter::*;
pub use self::document::*;
pub use self::figure::*;
pub use self::footnote::*;
pub use self::heading::*;
pub use self::link::*;
pub use self::metadata::*;
pub use self::numbering::*;
pub use self::outline::*;
pub use self::query::*;
pub use self::reference::*;
pub use self::state::*;

use crate::prelude::*;
use crate::text::TextElem;

/// Hook up all meta definitions.
pub(super) fn define(global: &mut Scope) {
    global.define("document", DocumentElem::func());
    global.define("ref", RefElem::func());
    global.define("link", LinkElem::func());
    global.define("outline", OutlineElem::func());
    global.define("heading", HeadingElem::func());
    global.define("figure", FigureElem::func());
    global.define("footnote", FootnoteElem::func());
    global.define("cite", CiteElem::func());
    global.define("bibliography", BibliographyElem::func());
    global.define("locate", locate_func());
    global.define("style", style_func());
    global.define("layout", layout_func());
    global.define("counter", counter_func());
    global.define("numbering", numbering_func());
    global.define("state", state_func());
    global.define("query", query_func());
    global.define("selector", selector_func());
    global.define("metadata", MetadataElem::func());
}

/// The named with which an element is referenced.
pub trait LocalName {
    /// Get the name in the given language and (optionally) region.
    fn local_name(&self, lang: Lang, region: Option<Region>) -> &'static str;

    /// Resolve the local name with a style chain.
    fn local_name_in(&self, styles: StyleChain) -> &'static str {
        self.local_name(TextElem::lang_in(styles), TextElem::region_in(styles))
    }
}
