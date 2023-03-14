//! Interaction between document parts.

mod document;
mod figure;
mod heading;
mod link;
mod numbering;
mod outline;
mod reference;

pub use self::document::*;
pub use self::figure::*;
pub use self::heading::*;
pub use self::link::*;
pub use self::numbering::*;
pub use self::outline::*;
pub use self::reference::*;

use typst::doc::Lang;

/// The named with which an element is referenced.
pub trait LocalName {
    /// Get the name in the given language.
    fn local_name(&self, lang: Lang) -> &'static str;
}
