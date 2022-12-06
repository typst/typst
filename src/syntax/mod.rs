//! Syntax definition, parsing, and highlighting.

pub mod ast;
pub mod highlight;

mod incremental;
mod kind;
mod node;
mod parser;
mod parsing;
mod resolve;
mod source;
mod span;
mod tokens;

pub use self::kind::*;
pub use self::node::*;
pub use self::parsing::*;
pub use self::source::*;
pub use self::span::*;
pub use self::tokens::*;

use incremental::reparse;
use parser::*;

#[cfg(test)]
mod tests;
