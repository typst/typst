//! The standard library for the _Typst_ language.

use crate::func::Scope;

pub_use_mod!(boxed);
pub_use_mod!(align);
pub_use_mod!(spacing);
pub_use_mod!(style);
pub_use_mod!(page);

/// Create a scope with all standard functions.
pub fn std() -> Scope {
    let mut std = Scope::new();

    std.add::<Boxed>("box");

    std.add::<Align>("align");

    std.add::<LineBreak>("n");
    std.add::<LineBreak>("line.break");
    std.add::<ParagraphBreak>("paragraph.break");
    std.add::<PageBreak>("page.break");
    std.add::<HorizontalSpace>("h");
    std.add::<VerticalSpace>("v");

    std.add::<Bold>("bold");
    std.add::<Italic>("italic");
    std.add::<Monospace>("mono");

    std.add::<PageSize>("page.size");
    std.add::<PageMargins>("page.margins");

    std
}
