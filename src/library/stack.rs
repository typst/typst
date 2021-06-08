use super::*;
use crate::layout::{StackChild, StackNode};

/// `stack`: Stack children along an axis.
///
/// # Positional parameters
/// - Children: variadic, of type `template`.
///
/// # Named parameters
/// - Stacking direction: `dir`, of type `direction`.
///
/// # Return value
/// A template that places its children along the specified layouting axis.
///
/// # Relevant types and constants
/// - Type `direction`
///   - `ltr`
///   - `rtl`
///   - `ttb`
///   - `btt`
pub fn stack(ctx: &mut EvalContext, args: &mut FuncArgs) -> Value {
    let dir = args.eat_named::<Dir>(ctx, "dir").unwrap_or(Dir::TTB);
    let children = args.eat_all::<TemplateValue>(ctx);

    Value::template("stack", move |ctx| {
        let children = children
            .iter()
            .map(|child| {
                let child = ctx.exec_template_stack(child).into();
                StackChild::Any(child, ctx.state.aligns)
            })
            .collect();

        ctx.push(StackNode {
            dirs: Gen::new(ctx.state.lang.dir, dir),
            aspect: None,
            children,
        });
    })
}
