use super::*;

/// `paragraph`: Configure paragraphs.
///
/// # Named parameters
/// - Paragraph spacing: `spacing`, of type `linear` relative to current font size.
/// - Line leading: `leading`, of type `linear` relative to current font size.
/// - Word spacing: `word-spacing`, of type `linear` relative to current font size.
///
/// # Return value
/// A template that configures paragraph properties. The effect is scoped to the
/// body if present.
pub fn paragraph(ctx: &mut EvalContext, args: &mut ValueArgs) -> Value {
    let spacing = args.get(ctx, "spacing");
    let leading = args.get(ctx, "leading");
    let word_spacing = args.get(ctx, "word-spacing");
    let body = args.find::<ValueTemplate>(ctx);

    Value::template("paragraph", move |ctx| {
        let snapshot = ctx.state.clone();

        if let Some(spacing) = spacing {
            ctx.state.par.spacing = spacing;
        }

        if let Some(leading) = leading {
            ctx.state.par.leading = leading;
        }

        if let Some(word_spacing) = word_spacing {
            ctx.state.par.word_spacing = word_spacing;
        }

        ctx.push_parbreak();

        if let Some(body) = &body {
            body.exec(ctx);
            ctx.state = snapshot;
        }
    })
}
