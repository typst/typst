//! Interaction between document parts.

mod bibliography;
mod context;
mod counter;
mod document;
mod figure;
mod heading;
mod link;
mod numbering;
mod outline;
mod query;
mod reference;
mod state;

use typst::doc::Lang;
use typst::eval::Scope;
use typst::model::Element as _;

pub(super) fn define(scope: &mut Scope) {
    scope.define("document", DocumentElem::func());
    scope.define("ref", RefElem::func());
    scope.define("link", LinkElem::func());
    scope.define("outline", OutlineElem::func());
    scope.define("heading", HeadingElem::func());
    scope.define("figure", FigureElem::func());
    scope.define("cite", CiteElem::func());
    scope.define("bibliography", BibliographyElem::func());
    scope.define("locate", locate);
    scope.define("style", style);
    scope.define("counter", counter);
    scope.define("numbering", numbering);
    scope.define("state", state);
    scope.define("query", query);
}

pub use self::bibliography::{
    BibliographyElem, BibliographyStyle, CitationStyle, CiteElem,
};
pub use self::context::{locate, style};
pub use self::counter::{counter, Count, Counter, CounterKey, CounterUpdate};
pub use self::document::{Author, DocumentElem};
pub use self::figure::FigureElem;
pub use self::heading::HeadingElem;
pub use self::link::LinkElem;
pub use self::numbering::{numbering, Numbering, NumberingPattern};
pub use self::outline::OutlineElem;
pub use self::query::query;
pub use self::reference::{RefElem, Supplement};
pub use self::state::{state, State, StateUpdate};

/// The named with which an element is referenced.
pub trait LocalName {
    /// Get the name in the given language.
    #[must_use]
    fn local_name(&self, lang: Lang) -> &'static str;
}
