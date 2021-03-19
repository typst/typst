use super::*;
use crate::layout::{BackgroundNode, Fill, FixedNode};

/// `rect`: Create a rectangular box.
///
/// # Positional parameters
/// - Body: optional, of type `template`.
///
/// # Named parameters
/// - Width of the box: `width`, of type `linear` relative to parent width.
/// - Height of the box: `height`, of type `linear` relative to parent height.
/// - Main layouting direction: `main-dir`, of type `direction`.
/// - Cross layouting direction: `cross-dir`, of type `direction`.
/// - Fill color of the box: `fill`, of type `color`.
///
/// # Return value
/// A template that places the body into a rectangle.
///
/// # Relevant types and constants
/// - Type `direction`
///   - `ltr` (left to right)
///   - `rtl` (right to left)
///   - `ttb` (top to bottom)
///   - `btt` (bottom to top)
pub fn rect(ctx: &mut EvalContext, args: &mut FuncArgs) -> Value {
    let width = args.get(ctx, "width");
    let height = args.get(ctx, "height");
    let main = args.get(ctx, "main-dir");
    let cross = args.get(ctx, "cross-dir");
    let fill = args.get(ctx, "fill");
    let body = args.find::<TemplateValue>(ctx).unwrap_or_default();

    Value::template("box", move |ctx| {
        let snapshot = ctx.state.clone();

        ctx.set_dirs(Gen::new(main, cross));

        let child = ctx.exec(&body).into();
        let fixed = FixedNode { width, height, child };
        if let Some(color) = fill {
            ctx.push(BackgroundNode {
                fill: Fill::Color(color),
                child: fixed.into(),
            });
        } else {
            ctx.push(fixed);
        }

        ctx.state = snapshot;
    })
}
