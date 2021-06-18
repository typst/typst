use super::*;
use crate::paper::{Paper, PaperClass};

/// `page`: Configure pages.
///
/// # Positional parameters
/// - Paper name: optional, of type `string`, see [here](crate::paper) for a
///   full list of all paper names.
/// - Body: optional, of type `template`.
///
/// # Named parameters
/// - Width of the page: `width`, of type `length`.
/// - Height of the page: `height`, of type `length`.
/// - Margins for all sides: `margins`, of type `linear` relative to sides.
/// - Left margin: `left`, of type `linear` relative to width.
/// - Right margin: `right`, of type `linear` relative to width.
/// - Top margin: `top`, of type `linear` relative to height.
/// - Bottom margin: `bottom`, of type `linear` relative to height.
/// - Flip width and height: `flip`, of type `bool`.
///
/// # Return value
/// A template that configures page properties. The effect is scoped to the body
/// if present.
pub fn page(ctx: &mut EvalContext, args: &mut FuncArgs) -> Value {
    let paper = args.eat::<Spanned<String>>(ctx).and_then(|name| {
        Paper::from_name(&name.v).or_else(|| {
            ctx.diag(error!(name.span, "invalid paper name"));
            None
        })
    });

    let width = args.named(ctx, "width");
    let height = args.named(ctx, "height");
    let margins = args.named(ctx, "margins");
    let left = args.named(ctx, "left");
    let top = args.named(ctx, "top");
    let right = args.named(ctx, "right");
    let bottom = args.named(ctx, "bottom");
    let flip = args.named(ctx, "flip");
    let body = args.eat::<TemplateValue>(ctx);
    let span = args.span;

    Value::template("page", move |ctx| {
        let snapshot = ctx.state.clone();

        if let Some(paper) = paper {
            ctx.state.page.class = paper.class;
            ctx.state.page.size = paper.size();
        }

        if let Some(width) = width {
            ctx.state.page.class = PaperClass::Custom;
            ctx.state.page.size.width = width;
        }

        if let Some(height) = height {
            ctx.state.page.class = PaperClass::Custom;
            ctx.state.page.size.height = height;
        }

        if let Some(margins) = margins {
            ctx.state.page.margins = Sides::splat(Some(margins));
        }

        if let Some(left) = left {
            ctx.state.page.margins.left = Some(left);
        }

        if let Some(top) = top {
            ctx.state.page.margins.top = Some(top);
        }

        if let Some(right) = right {
            ctx.state.page.margins.right = Some(right);
        }

        if let Some(bottom) = bottom {
            ctx.state.page.margins.bottom = Some(bottom);
        }

        if flip.unwrap_or(false) {
            let page = &mut ctx.state.page;
            std::mem::swap(&mut page.size.width, &mut page.size.height);
        }

        ctx.pagebreak(false, true, span);

        if let Some(body) = &body {
            // TODO: Restrict body to a single page?
            body.exec(ctx);
            ctx.state = snapshot;
            ctx.pagebreak(true, false, span);
        }
    })
}

/// `pagebreak`: Start a new page.
///
/// # Return value
/// A template that inserts a page break.
pub fn pagebreak(_: &mut EvalContext, args: &mut FuncArgs) -> Value {
    let span = args.span;
    Value::template("pagebreak", move |ctx| {
        ctx.pagebreak(true, true, span);
    })
}
