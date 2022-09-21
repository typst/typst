//! Computational utility functions.

mod color;
mod data;
mod math;
mod string;

pub use color::*;
pub use data::*;
pub use math::*;
pub use string::*;

use comemo::Track;

use crate::eval::{Eval, Route, Scopes, Vm};
use crate::library::prelude::*;
use crate::source::Source;

/// The name of a value's type.
pub fn type_(_: &mut Vm, args: &mut Args) -> SourceResult<Value> {
    Ok(args.expect::<Value>("value")?.type_name().into())
}

/// Ensure that a condition is fulfilled.
pub fn assert(_: &mut Vm, args: &mut Args) -> SourceResult<Value> {
    let Spanned { v, span } = args.expect::<Spanned<bool>>("condition")?;
    if !v {
        bail!(span, "assertion failed");
    }
    Ok(Value::None)
}

/// Evaluate a string as Typst markup.
pub fn eval(vm: &mut Vm, args: &mut Args) -> SourceResult<Value> {
    let Spanned { v: text, span } = args.expect::<Spanned<String>>("source")?;

    // Parse the source and set a synthetic span for all nodes.
    let source = Source::synthesized(text, span);
    let ast = source.ast()?;

    // Evaluate the source.
    let std = &vm.world.config().std;
    let scopes = Scopes::new(Some(std));
    let route = Route::default();
    let mut sub = Vm::new(vm.world, route.track(), None, scopes);
    let result = ast.eval(&mut sub);

    // Handle control flow.
    if let Some(flow) = sub.flow {
        bail!(flow.forbidden());
    }

    Ok(Value::Content(result?))
}
