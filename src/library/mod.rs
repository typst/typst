//! The standard library for the _Typst_ language.

use crate::func::Scope;

mod align;
mod boxed;
mod breaks;
mod styles;

/// Useful imports for creating your own functions.
pub mod prelude {
    pub use crate::func::{Command, CommandList, Function};
    pub use crate::layout::{layout_tree, Layout, LayoutContext, MultiLayout};
    pub use crate::layout::{LayoutError, LayoutResult};
    pub use crate::parsing::{parse, ParseContext, ParseError, ParseResult};
    pub use crate::syntax::{Expression, FuncHeader, SyntaxTree};
    pub use super::helpers::*;
}

pub use align::AlignFunc;
pub use boxed::BoxFunc;
pub use breaks::{LinebreakFunc, PagebreakFunc};
pub use styles::{BoldFunc, ItalicFunc, MonospaceFunc};

/// Create a scope with all standard functions.
pub fn std() -> Scope {
    let mut std = Scope::new();
    std.add::<AlignFunc>("align");
    std.add::<BoxFunc>("box");
    std.add::<LinebreakFunc>("linebreak");
    std.add::<LinebreakFunc>("n");
    std.add::<PagebreakFunc>("pagebreak");
    std.add::<BoldFunc>("bold");
    std.add::<ItalicFunc>("italic");
    std.add::<MonospaceFunc>("mono");
    std
}

pub mod helpers {
    use super::prelude::*;

    pub fn has_arguments(header: &FuncHeader) -> bool {
        !header.args.is_empty() || !header.kwargs.is_empty()
    }

    pub fn parse_maybe_body(body: Option<&str>, ctx: ParseContext) -> ParseResult<Option<SyntaxTree>> {
        if let Some(body) = body {
            Ok(Some(parse(body, ctx)?))
        } else {
            Ok(None)
        }
    }

    pub fn err<S: Into<String>, T>(message: S) -> ParseResult<T> {
        Err(ParseError::new(message))
    }
}
