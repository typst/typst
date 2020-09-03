//! Parsing of source code into syntax trees.

mod escaping;
mod parser;

pub use parser::parse;

#[cfg(test)]
mod tests;
