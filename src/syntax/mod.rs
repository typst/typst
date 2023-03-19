//! Syntax definition, parsing, and highlighting.

pub mod ast;

mod kind;
mod lexer;
mod node;
mod parser;
mod reparser;
mod source;
mod span;

pub use self::kind::*;
pub use self::lexer::*;
pub use self::node::*;
pub use self::parser::*;
pub use self::source::*;
pub use self::span::*;
