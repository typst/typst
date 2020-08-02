//! The _Typst_ standard library.

use crate::syntax::scope::Scope;
use crate::func::prelude::*;

pub_use_mod!(font);
pub_use_mod!(layout);
pub_use_mod!(page);
pub_use_mod!(spacing);

/// Create a scope with all standard functions.
pub fn std() -> Scope {
    let mut std = Scope::new::<ValFunc>();

    std.add::<ValFunc>("val");
    std.add::<FontFunc>("font");
    std.add::<PageFunc>("page");
    std.add::<AlignFunc>("align");
    std.add::<BoxFunc>("box");
    std.add_with_meta::<SpacingFunc>("h", Horizontal);
    std.add_with_meta::<SpacingFunc>("v", Vertical);
    std.add::<ParBreakFunc>("parbreak");
    std.add::<PageBreakFunc>("pagebreak");

    std
}

function! {
    /// `val`: Layouts the body with no special effect.
    #[derive(Debug, Clone, PartialEq)]
    pub struct ValFunc {
        body: Option<SyntaxModel>,
    }

    parse(header, body, state, f) {
        header.args.pos.0.clear();
        header.args.key.0.clear();
        ValFunc { body: body!(opt: body, state, f) }
    }

    layout(self, ctx, f) {
        match &self.body {
            Some(model) => vec![LayoutSyntaxModel(model)],
            None => vec![],
        }
    }
}

/// Layout an optional body with a change of the text style.
fn styled<'a, T, F>(
    body: &'a Option<SyntaxModel>,
    ctx: LayoutContext<'_>,
    data: Option<T>,
    f: F,
) -> Commands<'a> where F: FnOnce(&mut TextStyle, T) {
    if let Some(data) = data {
        let mut style = ctx.style.text.clone();
        f(&mut style, data);

        match body {
            Some(model) => vec![
                SetTextStyle(style),
                LayoutSyntaxModel(model),
                SetTextStyle(ctx.style.text.clone()),
            ],
            None => vec![SetTextStyle(style)],
        }
    } else {
        vec![]
    }
}
