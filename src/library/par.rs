use super::*;

/// `par`: Configure paragraphs.
///
/// # Named parameters
/// - Paragraph spacing: `spacing`, of type `linear` relative to current font size.
/// - Line leading: `leading`, of type `linear` relative to current font size.
/// - Word spacing: `word-spacing`, of type `linear` relative to current font size.
///
/// # Return value
/// A template that configures paragraph properties.
pub fn par(ctx: &mut EvalContext, args: &mut FuncArgs) -> Value {
    let spacing = args.get(ctx, "spacing");
    let leading = args.get(ctx, "leading");
    let word_spacing = args.get(ctx, "word-spacing");

    Value::template("par", move |ctx| {
        if let Some(spacing) = spacing {
            ctx.state.par.spacing = spacing;
        }

        if let Some(leading) = leading {
            ctx.state.par.leading = leading;
        }

        if let Some(word_spacing) = word_spacing {
            ctx.state.par.word_spacing = word_spacing;
        }

        ctx.parbreak();
    })
}
