//! The standard library for the _Typst_ language.

use crate::func::Scope;

mod structure;
mod style;

pub use structure::*;
pub use style::*;

/// Create a scope with all standard functions.
pub fn std() -> Scope {
    let mut std = Scope::new();

    std.add::<Align>("align");
    std.add::<Boxed>("box");

    std.add::<Linebreak>("line.break");
    std.add::<Linebreak>("n");
    std.add::<Pagebreak>("page.break");

    std.add::<HorizontalSpace>("h");
    std.add::<VerticalSpace>("v");

    std.add::<Bold>("bold");
    std.add::<Italic>("italic");
    std.add::<Monospace>("mono");

    std
}
