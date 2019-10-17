//! The standard library for the _Typst_ language.

use crate::func::Scope;

mod align;
mod styles;
mod breaks;

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
pub use breaks::PagebreakFunc;
pub use styles::{BoldFunc, ItalicFunc, MonospaceFunc};

/// Create a scope with all standard functions.
pub fn std() -> Scope {
    let mut std = Scope::new();
    std.add::<BoldFunc>("bold");
    std.add::<ItalicFunc>("italic");
    std.add::<MonospaceFunc>("mono");
    std.add::<AlignFunc>("align");
    std.add::<PagebreakFunc>("pagebreak");
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
