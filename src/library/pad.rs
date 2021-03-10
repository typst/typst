use super::*;

/// `pad`: Pad content at the sides.
///
/// # Positional arguments
/// - Padding for all sides: `padding`, of type `linear` relative to sides.
/// - Body: of type `template`.
///
/// # Named arguments
/// - Left padding:   `left`, of type `linear` relative to parent width.
/// - Right padding:  `right`, of type `linear` relative to parent width.
/// - Top padding:    `top`, of type `linear` relative to parent height.
/// - Bottom padding: `bottom`, of type `linear` relative to parent height.
pub fn pad(ctx: &mut EvalContext, args: &mut ValueArgs) -> Value {
    let all = args.find(ctx);
    let left = args.get(ctx, "left");
    let top = args.get(ctx, "top");
    let right = args.get(ctx, "right");
    let bottom = args.get(ctx, "bottom");
    let body = args.require::<ValueTemplate>(ctx, "body").unwrap_or_default();

    let padding = Sides::new(
        left.or(all).unwrap_or_default(),
        top.or(all).unwrap_or_default(),
        right.or(all).unwrap_or_default(),
        bottom.or(all).unwrap_or_default(),
    );

    Value::template("pad", move |ctx| {
        let snapshot = ctx.state.clone();

        let expand = Spec::uniform(Expansion::Fit);
        let child = ctx.exec_body(&body, expand);
        ctx.push(NodePad { padding, child });

        ctx.state = snapshot;
    })
}
