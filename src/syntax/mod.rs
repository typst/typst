//! Syntax types.

mod expr;
mod ident;
mod node;
mod span;
mod token;

pub use expr::*;
pub use ident::*;
pub use node::*;
pub use span::*;
pub use token::*;

use crate::pretty::{Pretty, Printer};

/// The abstract syntax tree.
pub type Tree = SpanVec<Node>;

impl Pretty for Tree {
    fn pretty(&self, p: &mut Printer) {
        for node in self {
            node.v.pretty(p);
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::parse::parse;
    use crate::pretty::pretty;

    #[track_caller]
    pub fn test_pretty(src: &str, exp: &str) {
        let tree = parse(src).output;
        let found = pretty(&tree);
        if exp != found {
            println!("tree:     {:#?}", tree);
            println!("expected: {}", exp);
            println!("found:    {}", found);
            panic!("test failed");
        }
    }
}
