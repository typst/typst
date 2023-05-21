//! Interaction between document parts.

mod bibliography;
mod context;
mod counter;
mod document;
mod figure;
mod footnote;
mod heading;
mod link;
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
pub use self::numbering::*;
pub use self::outline::*;
pub use self::query::*;
pub use self::reference::*;
pub use self::state::*;

use crate::prelude::*;

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
    global.define("locate", locate);
    global.define("style", style);
    global.define("layout", layout);
    global.define("counter", counter);
    global.define("numbering", numbering);
    global.define("state", state);
    global.define("query", query);
    global.define("selector", selector);
}

/// The named with which an element is referenced.
pub trait LocalName {
    /// Get the name in the given language and (optionally) region.
    fn local_name(&self, lang: Lang, region: Option<Region>) -> &'static str;
}
