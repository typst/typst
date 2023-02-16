//! Interaction between document parts.

mod document;
mod heading;
mod link;
mod numbering;
mod outline;
mod reference;

pub use self::document::*;
pub use self::heading::*;
pub use self::link::*;
pub use self::numbering::*;
pub use self::outline::*;
pub use self::reference::*;
