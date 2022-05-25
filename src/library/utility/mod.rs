//! Computational utility functions.

mod color;
mod math;
mod string;

pub use color::*;
pub use math::*;
pub use string::*;

use std::mem;

use crate::eval::{Eval, Machine, Scopes};
use crate::library::prelude::*;
use crate::source::SourceFile;

/// The name of a value's type.
pub fn type_(_: &mut Context, args: &mut Args) -> TypResult<Value> {
    Ok(args.expect::<Value>("value")?.type_name().into())
}

/// Ensure that a condition is fulfilled.
pub fn assert(_: &mut Context, args: &mut Args) -> TypResult<Value> {
    let Spanned { v, span } = args.expect::<Spanned<bool>>("condition")?;
    if !v {
        bail!(span, "assertion failed");
    }
    Ok(Value::None)
}

/// Evaluate a string as Typst markup.
pub fn eval(ctx: &mut Context, args: &mut Args) -> TypResult<Value> {
    let Spanned { v: src, span } = args.expect::<Spanned<String>>("source")?;

    // Parse the source and set a synthetic span for all nodes.
    let source = SourceFile::synthesized(src, span);
    let ast = source.ast()?;

    // Save the old route, then detach it.
    let prev_route = mem::take(&mut ctx.route);

    // Evaluate the source.
    let std = ctx.config.std.clone();
    let scopes = Scopes::new(Some(&std));
    let mut vm = Machine::new(ctx, scopes);
    let result = ast.eval(&mut vm);
    let flow = vm.flow;

    // Restore the old route.
    ctx.route = prev_route;

    // Handle control flow.
    if let Some(flow) = flow {
        return Err(flow.forbidden());
    }

    Ok(Value::Content(result?))
}
