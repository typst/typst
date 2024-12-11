//! Structuring elements that define the document model.

// TODO!
pub mod grid;

mod bibliography;
mod cite;
mod document;
mod emph;
#[path = "enum.rs"]
mod enum_;
mod figure;
mod footnote;
mod heading;
mod link;
mod list;
#[path = "numbering.rs"]
mod numbering_;
mod outline;
mod par;
mod quote;
mod reference;
mod strong;
mod table;
mod terms;

pub use self::bibliography::*;
pub use self::cite::*;
pub use self::document::*;
pub use self::emph::*;
pub use self::enum_::*;
pub use self::figure::*;
pub use self::footnote::*;
pub use self::heading::*;
pub use self::link::*;
pub use self::list::*;
pub use self::numbering_::*;
pub use self::outline::*;
pub use self::par::*;
pub use self::quote::*;
pub use self::reference::*;
pub use self::strong::*;
pub use self::table::*;
pub use self::terms::*;

use crate::foundations::{category, Category, Scope};

/// Document structuring.
///
/// Here, you can find functions to structure your document and interact with
/// that structure. This includes section headings, figures, bibliography
/// management, cross-referencing and more.
#[category]
pub static MODEL: Category;

/// Hook up all `model` definitions.
pub fn define(global: &mut Scope) {
    global.category(MODEL);
    global.define_elem::<DocumentElem>();
    global.define_elem::<RefElem>();
    global.define_elem::<LinkElem>();
    global.define_elem::<OutlineElem>();
    global.define_elem::<HeadingElem>();
    global.define_elem::<FigureElem>();
    global.define_elem::<FootnoteElem>();
    global.define_elem::<QuoteElem>();
    global.define_elem::<CiteElem>();
    global.define_elem::<BibliographyElem>();
    global.define_elem::<EnumElem>();
    global.define_elem::<ListElem>();
    global.define_elem::<ParbreakElem>();
    global.define_elem::<ParElem>();
    global.define_elem::<TableElem>();
    global.define_elem::<TermsElem>();
    global.define_elem::<EmphElem>();
    global.define_elem::<StrongElem>();
    global.define_func::<numbering>();
}
