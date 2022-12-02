//! Interaction between document parts.

mod document;
mod link;
mod reference;
mod outline;

pub use self::document::*;
pub use self::outline::*;
pub use self::link::*;
pub use self::reference::*;
