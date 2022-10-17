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

pub use kind::*;
pub use node::*;
pub use parsing::*;
pub use source::*;
pub use span::*;
pub use tokens::*;

use incremental::reparse;
use parser::*;

#[cfg(test)]
mod tests {
    use std::fmt::Debug;

    #[track_caller]
    pub fn check<T>(text: &str, found: T, expected: T)
    where
        T: Debug + PartialEq,
    {
        if found != expected {
            println!("source:   {text:?}");
            println!("expected: {expected:#?}");
            println!("found:    {found:#?}");
            panic!("test failed");
        }
    }
}
