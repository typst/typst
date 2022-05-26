//! Computational utility functions.

mod color;
mod locate;
mod math;
mod string;

pub use color::*;
pub use locate::*;
pub use math::*;
pub use string::*;

use crate::eval::{Eval, Machine, Scopes};
use crate::library::prelude::*;
use crate::source::SourceFile;

/// The name of a value's type.
pub fn type_(_: &mut Machine, args: &mut Args) -> TypResult<Value> {
    Ok(args.expect::<Value>("value")?.type_name().into())
}

/// Ensure that a condition is fulfilled.
pub fn assert(_: &mut Machine, args: &mut Args) -> TypResult<Value> {
    let Spanned { v, span } = args.expect::<Spanned<bool>>("condition")?;
    if !v {
        bail!(span, "assertion failed");
    }
    Ok(Value::None)
}

/// Evaluate a string as Typst markup.
pub fn eval(vm: &mut Machine, args: &mut Args) -> TypResult<Value> {
    let Spanned { v: src, span } = args.expect::<Spanned<String>>("source")?;

    // Parse the source and set a synthetic span for all nodes.
    let source = SourceFile::synthesized(src, span);
    let ast = source.ast()?;

    // Evaluate the source.
    let std = vm.ctx.config.std.clone();
    let scopes = Scopes::new(Some(&std));
    let mut sub = Machine::new(vm.ctx, vec![], scopes);
    let result = ast.eval(&mut sub);
    assert!(vm.deps.is_empty());

    // Handle control flow.
    if let Some(flow) = sub.flow {
        return Err(flow.forbidden());
    }

    Ok(Value::Content(result?))
}
