//! The standard library.

mod font;
mod layout;
mod page;
mod spacing;

pub use font::*;
pub use layout::*;
pub use page::*;
pub use spacing::*;

use crate::func::prelude::*;
use crate::syntax::scope::Scope;

/// Create a scope with all standard library functions.
pub fn std() -> Scope {
    let mut std = Scope::new::<ValFunc>();

    std.add::<ValFunc>("val");
    std.add::<FontFunc>("font");
    std.add::<PageFunc>("page");
    std.add::<AlignFunc>("align");
    std.add::<BoxFunc>("box");
    std.add::<PageBreakFunc>("pagebreak");
    std.add_with_meta::<SpacingFunc>("h", Horizontal);
    std.add_with_meta::<SpacingFunc>("v", Vertical);

    std
}

function! {
    /// `val`: Ignores all arguments and layouts the body flatly.
    ///
    /// This is also the fallback function, which is used when a function name
    /// could not be resolved.
    #[derive(Debug, Clone, PartialEq)]
    pub struct ValFunc {
        body: Option<SyntaxTree>,
    }

    parse(header, body, state, f) {
        header.args.pos.0.clear();
        header.args.key.0.clear();
        Self { body: parse_maybe_body(body, state, f), }
    }

    layout(self, ctx, f) {
        match &self.body {
            Some(tree) => vec![LayoutSyntaxTree(tree)],
            None => vec![],
        }
    }
}
