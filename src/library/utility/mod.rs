//! Computational utility functions.

mod color;
mod math;
mod string;

pub use color::*;
pub use math::*;
pub use string::*;

use std::mem;

use crate::eval::{Eval, Scopes};
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
    let mut source = SourceFile::detached(src);
    source.synthesize(span);
    let ast = source.ast()?;

    // Save the old context, then detach it.
    let prev_flow = ctx.flow.take();
    let prev_route = mem::take(&mut ctx.route);

    // Evaluate the source.
    let std = ctx.std.clone();
    let mut scp = Scopes::new(Some(&std));
    let result = ast.eval(ctx, &mut scp);

    // Restore the old context and handle control flow.
    ctx.route = prev_route;
    if let Some(flow) = mem::replace(&mut ctx.flow, prev_flow) {
        return Err(flow.forbidden());
    }

    Ok(Value::Content(result?))
}
