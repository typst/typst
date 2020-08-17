//! The standard library.

mod align;
mod boxed;
mod font;
mod page;
mod spacing;

pub use align::*;
pub use boxed::*;
pub use font::*;
pub use page::*;
pub use spacing::*;

use std::rc::Rc;

use crate::compute::scope::Scope;
use crate::prelude::*;

macro_rules! std {
    (fallback: $fallback:expr $(, $name:literal => $func:expr)* $(,)?) => {
        /// Create a scope with all standard library functions.
        pub fn _std() -> Scope {
            let mut std = Scope::new(wrap!(val));
            $(std.insert($name, wrap!($func));)*
            std
        }
    };
}

macro_rules! wrap {
    ($func:expr) => {
        Rc::new(|name, args, ctx| Box::pin($func(name, args, ctx)))
    };
}

std! {
    fallback: val,
    "align" => align,
    "box" => boxed,
    "dump" => dump,
    "font" => font,
    "h" => h,
    "page" => page,
    "pagebreak" => pagebreak,
    "v" => v,
    "val" => val,
}

/// `val`: Layouts its body flatly, ignoring other arguments.
///
/// This is also the fallback function, which is used when a function name
/// cannot be resolved.
pub async fn val(_: Span, mut args: TableValue, _: LayoutContext<'_>) -> Pass<Value> {
    let commands = match args.take::<SyntaxTree>() {
        Some(tree) => vec![LayoutSyntaxTree(tree)],
        None => vec![],
    };

    Pass::commands(commands, Feedback::new())
}

/// `dump`: Dumps its arguments into the document.
pub async fn dump(_: Span, args: TableValue, _: LayoutContext<'_>) -> Pass<Value> {
    Pass::okay(Value::Table(args))
}
