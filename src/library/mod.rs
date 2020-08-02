//! The standard library.

use crate::func::prelude::*;
use crate::layout::{LayoutContext, Commands};
use crate::syntax::scope::Scope;

macro_rules! lib { ($name:ident) => { mod $name; pub use $name::*; }}
lib!(font);
lib!(layout);
lib!(page);
lib!(spacing);

/// Create a scope with all standard functions.
pub fn std() -> Scope {
    let mut std = Scope::new::<ValFunc>();

    std.add::<ValFunc>("val");
    std.add::<FontFunc>("font");
    std.add::<PageFunc>("page");
    std.add::<AlignFunc>("align");
    std.add::<BoxFunc>("box");
    std.add::<ParBreakFunc>("parbreak");
    std.add::<PageBreakFunc>("pagebreak");
    std.add_with_meta::<SpacingFunc>("h", Horizontal);
    std.add_with_meta::<SpacingFunc>("v", Vertical);

    std
}

function! {
    /// `val`: Layouts the body with no special effect.
    #[derive(Debug, Clone, PartialEq)]
    pub struct ValFunc {
        body: Option<SyntaxTree>,
    }

    parse(header, body, state, f) {
        header.args.pos.0.clear();
        header.args.key.0.clear();
        ValFunc { body: parse_maybe_body(body, state, f), }
    }

    layout(self, ctx, f) {
        match &self.body {
            Some(tree) => vec![LayoutSyntaxTree(tree)],
            None => vec![],
        }
    }
}

/// Layout an optional body with a change of the text style.
fn styled<'a, T, F>(
    body: &'a Option<SyntaxTree>,
    ctx: LayoutContext<'_>,
    data: Option<T>,
    f: F,
) -> Commands<'a> where F: FnOnce(&mut TextStyle, T) {
    if let Some(data) = data {
        let mut style = ctx.style.text.clone();
        f(&mut style, data);

        match body {
            Some(tree) => vec![
                SetTextStyle(style),
                LayoutSyntaxTree(tree),
                SetTextStyle(ctx.style.text.clone()),
            ],
            None => vec![SetTextStyle(style)],
        }
    } else {
        vec![]
    }
}
