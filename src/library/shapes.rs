use super::*;

/// `rect`: Create a rectangular box.
///
/// # Positional arguments
/// - Body: optional, of type `template`.
///
/// # Named arguments
/// - Width of the box:          `width`, of type `linear` relative to parent width.
/// - Height of the box:         `height`, of type `linear` relative to parent height.
/// - Main layouting direction:  `main-dir`, of type `direction`.
/// - Cross layouting direction: `cross-dir`, of type `direction`.
/// - Fill color of the box:     `fill`, of type `color`.
///
/// # Relevant types and constants
/// - Type `direction`
///     - `ltr` (left to right)
///     - `rtl` (right to left)
///     - `ttb` (top to bottom)
///     - `btt` (bottom to top)
pub fn rect(ctx: &mut EvalContext, args: &mut ValueArgs) -> Value {
    let width = args.get(ctx, "width");
    let height = args.get(ctx, "height");
    let main = args.get(ctx, "main-dir");
    let cross = args.get(ctx, "cross-dir");
    let fill = args.get(ctx, "fill");
    let body = args.find::<ValueTemplate>(ctx).unwrap_or_default();

    Value::template("box", move |ctx| {
        let snapshot = ctx.state.clone();

        ctx.set_dirs(Gen::new(main, cross));

        let child = ctx.exec(&body).into();
        let fixed = NodeFixed { width, height, child };
        if let Some(color) = fill {
            ctx.push(NodeBackground {
                fill: Fill::Color(color),
                child: fixed.into(),
            });
        } else {
            ctx.push(fixed);
        }

        ctx.state = snapshot;
    })
}
