use crate::func::Scope;
use super::*;


/// The result type for parsing.
pub type ParseResult<T> = crate::TypesetResult<T>;

/// Parses source code into a syntax tree given a context.
pub fn parse(src: &str, ctx: ParseContext) -> ParseResult<SyntaxTree> {
    unimplemented!()
}

/// The context for parsing.
#[derive(Debug, Copy, Clone)]
pub struct ParseContext<'a> {
    /// The scope containing function definitions.
    pub scope: &'a Scope,
}
