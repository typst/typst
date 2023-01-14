//! Syntax definition, parsing, and highlighting.

pub mod ast;

mod incremental;
mod kind;
mod lexer;
mod node;
mod parser;
mod parsing;
mod resolve;
mod source;
mod span;

pub use self::kind::*;
pub use self::lexer::*;
pub use self::node::*;
pub use self::parsing::*;
pub use self::source::*;
pub use self::span::*;

use incremental::reparse;
use parser::*;
