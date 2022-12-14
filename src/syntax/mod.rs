//! Syntax definition, parsing, and highlighting.

pub mod ast;

mod incremental;
mod kind;
mod linked;
mod node;
mod parser;
mod parsing;
mod resolve;
mod source;
mod span;
mod tokens;

pub use self::kind::*;
pub use self::linked::*;
pub use self::node::*;
pub use self::parsing::*;
pub use self::source::*;
pub use self::span::*;
pub use self::tokens::*;

use incremental::reparse;
use parser::*;

#[cfg(test)]
mod tests;
