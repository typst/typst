//! The _Typst_ standard library.

use crate::syntax::Scope;
use crate::func::prelude::*;

pub_use_mod!(font);
pub_use_mod!(layout);
pub_use_mod!(page);
pub_use_mod!(spacing);


/// Create a scope with all standard functions.
pub fn std() -> Scope {
    let mut std = Scope::new::<ValFunc>();

    // Basics
    std.add::<ValFunc>("val");

    // Font setup
    std.add::<FontFamilyFunc>("font.family");
    std.add::<FontStyleFunc>("font.style");
    std.add::<FontWeightFunc>("font.weight");
    std.add::<FontSizeFunc>("font.size");
    std.add_with_meta::<ContentSpacingFunc>("word.spacing", ContentKind::Word);

    // Layout
    std.add_with_meta::<ContentSpacingFunc>("line.spacing", ContentKind::Line);
    std.add_with_meta::<ContentSpacingFunc>("par.spacing", ContentKind::Paragraph);
    std.add::<AlignFunc>("align");
    std.add::<DirectionFunc>("direction");
    std.add::<BoxFunc>("box");

    // Spacing
    std.add::<LineBreakFunc>("n");
    std.add::<LineBreakFunc>("line.break");
    std.add::<ParBreakFunc>("par.break");
    std.add::<PageBreakFunc>("page.break");
    std.add_with_meta::<SpacingFunc>("spacing", None);
    std.add_with_meta::<SpacingFunc>("h", Some(Horizontal));
    std.add_with_meta::<SpacingFunc>("v", Some(Vertical));

    // Page setup
    std.add::<PageSizeFunc>("page.size");
    std.add::<PageMarginsFunc>("page.margins");

    std
}

function! {
    /// `val`: Layouts the body with no special effect.
    #[derive(Debug, Clone, PartialEq)]
    pub struct ValFunc {
        body: Option<SyntaxModel>,
    }

    parse(header, body, ctx, errors, decos) {
        ValFunc { body: body!(opt: body, ctx, errors, decos) }
    }

    layout(self, ctx, errors) {
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
